// game-translator/src/main.rs

// ============================================================================
// GAME TRANSLATOR - Aplicação para traduzir textos de jogos em tempo real
// ============================================================================

#[macro_use]
extern crate log;

// ============================================================================
// DECLARAÇÃO DE MÓDULOS
// ============================================================================
mod cache;
mod config;
mod hotkey;
mod ocr;
mod overlay;
mod region_selector;
mod screenshot;
mod subtitle;
mod translator;
mod tts;

// ============================================================================
// IMPORTS
// ============================================================================
use anyhow::Result;
use config::Config;
use crossbeam_channel::{unbounded, Receiver, Sender};
use notify::{RecursiveMode, Watcher};
use ocr::TranslatedText;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// ============================================================================
// COMANDOS ENTRE THREADS
// ============================================================================

/// Comandos que podem ser enviados da thread de hotkeys para a main thread
#[derive(Debug, Clone)]
enum AppCommand {
    /// Abre o seletor de região
    OpenRegionSelector,
    /// Abre o seletor de região de legendas
    OpenSubtitleRegionSelector,
}

// ============================================================================
// ESTRUTURA DE ESTADO COMPARTILHADO
// ============================================================================

/// Estado compartilhado entre a UI (overlay) e a thread de hotkeys
/// Região onde o texto foi capturado
#[derive(Clone, Debug)]
struct CaptureRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Clone)]
struct AppState {
    config: Arc<Mutex<Config>>,
    translated_items: Arc<Mutex<Vec<TranslatedText>>>,
    capture_region: Arc<Mutex<Option<CaptureRegion>>>,
    translation_timestamp: Arc<Mutex<Option<std::time::Instant>>>,
    command_sender: Sender<AppCommand>,
    /// Cache de traduções
    translation_cache: cache::TranslationCache,
    /// Indica se o modo legenda está ativo
    subtitle_mode_active: Arc<Mutex<bool>>,
    /// Estado do sistema de legendas
    subtitle_state: subtitle::SubtitleState,
    /// Controla se o overlay deve ficar escondido (durante captura)
    overlay_hidden: Arc<Mutex<bool>>,
}

impl AppState {
    fn new(config: Config, command_sender: Sender<AppCommand>) -> Self {
        // Cria cache com persistência em disco
        let translation_cache = cache::TranslationCache::new(true);

        // Cria estado de legendas com configurações do config
        let subtitle_state = subtitle::SubtitleState::new(
            config.app_config.subtitle.min_display_secs,
            config.app_config.subtitle.max_display_secs,
        );

        AppState {
            config: Arc::new(Mutex::new(config)),
            translated_items: Arc::new(Mutex::new(Vec::new())),
            capture_region: Arc::new(Mutex::new(None)),
            translation_timestamp: Arc::new(Mutex::new(None)),
            command_sender,
            translation_cache,
            subtitle_mode_active: Arc::new(Mutex::new(false)),
            subtitle_state,
            overlay_hidden: Arc::new(Mutex::new(false)),
        }
    }

    /// Define a lista de textos traduzidos com posições e a região de captura
    fn set_translations(&self, items: Vec<TranslatedText>, region: CaptureRegion) {
        *self.translated_items.lock().unwrap() = items;
        *self.capture_region.lock().unwrap() = Some(region);
        *self.translation_timestamp.lock().unwrap() = Some(std::time::Instant::now());
    }

    /// Obtém a lista de traduções e a região
    fn get_translations(&self) -> Option<(Vec<TranslatedText>, CaptureRegion, std::time::Instant)> {
        let items = self.translated_items.lock().unwrap().clone();
        let region = self.capture_region.lock().unwrap().clone()?;
        let timestamp = self.translation_timestamp.lock().unwrap().clone()?;

        if items.is_empty() {
            return None;
        }

        Some((items, region, timestamp))
    }

    /// Limpa as traduções
    fn clear_translations(&self) {
        *self.translated_items.lock().unwrap() = Vec::new();
        *self.capture_region.lock().unwrap() = None;
        *self.translation_timestamp.lock().unwrap() = None;
    }
}
// ============================================================================
// APLICAÇÃO DE OVERLAY (roda na main thread)
// ============================================================================

struct OverlayApp {
    state: AppState,
    display_duration: Duration,
    command_receiver: Receiver<AppCommand>,
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // Totalmente transparente
    }

    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        // ====================================================================
        // TORNA A JANELA CLICK-THROUGH (apenas uma vez)
        // ====================================================================
        #[cfg(windows)]
        {
            use std::sync::Once;
            static INIT: Once = Once::new();
            INIT.call_once(|| {
                // Pequeno delay para garantir que a janela foi criada
                std::thread::sleep(std::time::Duration::from_millis(100));
                make_window_click_through();
            });
        }
        // ====================================================================
        // VERIFICA SE O OVERLAY DEVE FICAR ESCONDIDO (durante captura)
        // ====================================================================
        let is_hidden = *self.state.overlay_hidden.lock().unwrap();
        if is_hidden {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                eframe::egui::vec2(1.0, 1.0),
            ));
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            return;
        }
        // ====================================================================
        // PROCESSA COMANDOS RECEBIDOS
        // ====================================================================
        while let Ok(command) = self.command_receiver.try_recv() {
            match command {
                AppCommand::OpenRegionSelector => {
                    info!("🎯 Abrindo seletor de região...");

                    // Esconde o overlay temporariamente
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                        eframe::egui::vec2(1.0, 1.0),
                    ));

                    // Abre o seletor de região
                    match region_selector::select_region() {
                        Ok(Some(selected)) => {
                            info!(
                                "✅ Região selecionada: {}x{} na posição ({}, {})",
                                selected.width, selected.height, selected.x, selected.y
                            );

                            let mut config = self.state.config.lock().unwrap();
                            if let Err(e) = config.app_config.update_region(
                                selected.x,
                                selected.y,
                                selected.width,
                                selected.height,
                            ) {
                                error!("❌ Erro ao salvar região: {}", e);
                            } else {
                                info!("💾 Região salva no config.json!");
                                config.region_x = selected.x;
                                config.region_y = selected.y;
                                config.region_width = selected.width;
                                config.region_height = selected.height;
                            }
                        }
                        Ok(None) => info!("❌ Seleção cancelada"),
                        Err(e) => error!("❌ Erro no seletor: {}", e),
                    }
                }

                AppCommand::OpenSubtitleRegionSelector => {
                    info!("📺 Abrindo seletor de região de legendas...");

                    // Esconde o overlay temporariamente
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                        eframe::egui::vec2(1.0, 1.0),
                    ));

                    // Abre o seletor de região
                    match region_selector::select_region() {
                        Ok(Some(selected)) => {
                            info!(
                                "✅ Região de legendas selecionada: {}x{} na posição ({}, {})",
                                selected.width, selected.height, selected.x, selected.y
                            );

                            let mut config = self.state.config.lock().unwrap();
                            // Atualiza a região de legendas
                            config.app_config.subtitle.region.x = selected.x;
                            config.app_config.subtitle.region.y = selected.y;
                            config.app_config.subtitle.region.width = selected.width;
                            config.app_config.subtitle.region.height = selected.height;

                            // Salva no arquivo
                            if let Err(e) = config.app_config.save() {
                                error!("❌ Erro ao salvar região de legendas: {}", e);
                            } else {
                                info!("💾 Região de legendas salva no config.json!");
                            }
                        }
                        Ok(None) => info!("❌ Seleção cancelada"),
                        Err(e) => error!("❌ Erro no seletor: {}", e),
                    }
                }
            }
        }

        // ====================================================================
        // VERIFICA SE HÁ LEGENDAS DO MODO LEGENDA PARA EXIBIR
        // ====================================================================
        let subtitle_mode_active = *self.state.subtitle_mode_active.lock().unwrap();
        let has_subtitles = self.state.subtitle_state.has_subtitles();

        // ====================================================================
        // VERIFICA SE HÁ TRADUÇÕES NORMAIS PARA EXIBIR
        // ====================================================================
        let should_display = if let Some((_, _, timestamp)) = self.state.get_translations() {
            timestamp.elapsed() < self.display_duration
        } else {
            false
        };

        // ====================================================================
        // MODO LEGENDA: Exibe histórico de legendas acima da região
        // ====================================================================
        if subtitle_mode_active && has_subtitles {
            // Pega a região de legenda do config
            let (sub_x, sub_y, sub_w, sub_h) = {
                let config = self.state.config.lock().unwrap();
                (
                    config.app_config.subtitle.region.x as f32,
                    config.app_config.subtitle.region.y as f32,
                    config.app_config.subtitle.region.width as f32,
                    config.app_config.subtitle.region.height as f32,
                )
            };

            // Pega configurações de fonte (específica de legendas) e fundo
            let (font_size, font_color, show_background, bg_color) = {
                let config = self.state.config.lock().unwrap();
                (
                    config.app_config.subtitle.font.size,
                    config.app_config.subtitle.font.color,
                    config.app_config.overlay.show_background,
                    config.app_config.overlay.background_color,
                )
            };

            // Pega o histórico de legendas
            let history = self.state.subtitle_state.get_subtitle_history();

            // Configuração: máximo de 3 linhas de legenda
            let max_lines: usize = 3;
            let line_height = font_size + 10.0;
            let overlay_height = (max_lines as f32 * line_height) + 15.0;

            // Pega apenas as últimas N legendas
            let visible_history: Vec<_> = if history.len() > max_lines {
                history[(history.len() - max_lines)..].to_vec()
            } else {
                history.clone()
            };

            // Posiciona o overlay ACIMA da região de legenda
            let overlay_x = sub_x;
            let overlay_y = sub_y - overlay_height - 10.0; // 10px de espaço
            let overlay_width = sub_w;

            // Posiciona e redimensiona a janela
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                eframe::egui::pos2(overlay_x, overlay_y),
            ));
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                eframe::egui::vec2(overlay_width, overlay_height),
            ));

            // Renderiza o histórico de legendas
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
                .show(ctx, |ui| {
                    // Se show_background = true, desenha o fundo
                    if show_background {
                        let rect = ui.max_rect();
                        ui.painter().rect_filled(
                            rect,
                            0.0,
                            eframe::egui::Color32::from_rgba_unmultiplied(
                                bg_color[0],
                                bg_color[1],
                                bg_color[2],
                                bg_color[3],
                            ),
                        );
                    }

                    // Renderiza cada legenda do histórico
                    let mut y_offset = 5.0;
                    let font_id = eframe::egui::FontId::proportional(font_size);
                    let max_width = overlay_width - 20.0; // Margem de 10px em cada lado

                    for entry in &visible_history {
                        let text = format!("-- {}", entry.translated);
                        let text_pos = eframe::egui::pos2(10.0, y_offset);

                        let text_color = eframe::egui::Color32::from_rgba_unmultiplied(
                            font_color[0],
                            font_color[1],
                            font_color[2],
                            font_color[3],
                        );

                        // Se não tem fundo, desenha contorno
                        if !show_background {
                            let outline_size = 2.0;
                            let outline_color = eframe::egui::Color32::BLACK;
                            let offsets = [
                                (-outline_size, 0.0),
                                (outline_size, 0.0),
                                (0.0, -outline_size),
                                (0.0, outline_size),
                            ];

                            for (dx, dy) in offsets {
                                let offset_pos = text_pos + eframe::egui::vec2(dx, dy);
                                let galley = ui.painter().layout(
                                    text.clone(),
                                    font_id.clone(),
                                    outline_color,
                                    max_width,
                                );
                                ui.painter().galley(offset_pos, galley, outline_color);
                            }
                        }

                        // Desenha o texto principal com wrap automático
                        let galley = ui.painter().layout(
                            text.clone(),
                            font_id.clone(),
                            text_color,
                            max_width,
                        );
                        ui.painter().galley(text_pos, galley, text_color);

                        y_offset += line_height;
                    }
                });
        } else if should_display {
            // ================================================================
            // HÁ TRADUÇÃO: Mostra overlay com os textos
            // ================================================================
            if let Some((items, region, timestamp)) = self.state.get_translations() {
                let elapsed = timestamp.elapsed();

                // Usa a região de captura para posicionar o overlay
                let overlay_x = region.x as f32;
                let overlay_y = region.y as f32;
                let overlay_width = region.width as f32;
                let overlay_height = region.height as f32;

                // Pega tamanho da fonte do config
                let font_size = self.state.config.lock().unwrap().app_config.font.size;

                // Posiciona e redimensiona a janela
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                    eframe::egui::pos2(overlay_x, overlay_y),
                ));
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                    eframe::egui::vec2(overlay_width, overlay_height),
                ));

                // Pega configuração de fundo
                let show_background = self
                    .state
                    .config
                    .lock()
                    .unwrap()
                    .app_config
                    .overlay
                    .show_background;
                let bg_color = self
                    .state
                    .config
                    .lock()
                    .unwrap()
                    .app_config
                    .overlay
                    .background_color;

                // Renderiza o conteúdo
                eframe::egui::CentralPanel::default()
                    .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
                    .show(ctx, |ui| {
                        // Junta todas as traduções em um texto só
                        let combined_text: String = items
                            .iter()
                            .filter(|item| item.original != item.translated)
                            .map(|item| item.translated.as_str())
                            .collect::<Vec<&str>>()
                            .join(" ");

                        if !combined_text.is_empty() {
                            // Se show_background = true, desenha o fundo preto
                            if show_background {
                                let rect = ui.max_rect();
                                ui.painter().rect_filled(
                                    rect,
                                    0.0,
                                    eframe::egui::Color32::from_rgba_unmultiplied(
                                        bg_color[0],
                                        bg_color[1],
                                        bg_color[2],
                                        bg_color[3],
                                    ),
                                );
                            }

                            // Posição inicial do texto (com margem)
                            let text_pos = eframe::egui::pos2(20.0, 15.0);

                            // Configura a fonte
                            let font_id = eframe::egui::FontId::proportional(font_size);

                            // Largura máxima para wrap
                            let max_width = overlay_width - 40.0;

                            // Cria o layout do texto (com wrap)
                            let galley = ui.painter().layout(
                                combined_text.clone(),
                                font_id.clone(),
                                eframe::egui::Color32::WHITE,
                                max_width,
                            );

                            // Se não tem fundo, desenha contorno
                            if !show_background {
                                let outline_size = 2.0;
                                let outline_color = eframe::egui::Color32::BLACK;
                                let offsets = [
                                    (-outline_size, -outline_size),
                                    (0.0, -outline_size),
                                    (outline_size, -outline_size),
                                    (-outline_size, 0.0),
                                    (outline_size, 0.0),
                                    (-outline_size, outline_size),
                                    (0.0, outline_size),
                                    (outline_size, outline_size),
                                ];

                                for (dx, dy) in offsets {
                                    let offset_pos = text_pos + eframe::egui::vec2(dx, dy);
                                    let outline_galley = ui.painter().layout(
                                        combined_text.clone(),
                                        font_id.clone(),
                                        outline_color,
                                        max_width,
                                    );
                                    ui.painter()
                                        .galley(offset_pos, outline_galley, outline_color);
                                }
                            }

                            // Desenha o texto principal (branco) por cima
                            ui.painter()
                                .galley(text_pos, galley, eframe::egui::Color32::WHITE);
                        }
                    });

                // Verifica se o tempo acabou
                if elapsed >= self.display_duration {
                    self.state.clear_translations();
                }
            }
        } else {
            // ================================================================
            // SEM TRADUÇÃO: Janela mínima e invisível
            // ================================================================
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                eframe::egui::vec2(1.0, 1.0),
            ));
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                eframe::egui::pos2(0.0, 0.0),
            ));

            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
                .show(ctx, |_ui| {});
        }

        // Repaint contínuo
        ctx.request_repaint();
    }
}

// ============================================================================
// THREAD DE HOTKEYS (roda em background)
// ============================================================================

fn start_hotkey_thread(state: AppState) {
    thread::spawn(move || {
        info!("⌨️  Thread de hotkeys iniciada");

        let mut hotkey_manager = hotkey::HotkeyManager::new();

        loop {
            if let Some(action) = hotkey_manager.check_hotkey() {
                match action {
                    hotkey::HotkeyAction::SelectRegion => {
                        info!("");
                        info!("🎯 ============================================");
                        info!("🎯 SOLICITANDO ABERTURA DO SELETOR DE REGIÃO");
                        info!("🎯 ============================================");

                        if let Err(e) = state.command_sender.send(AppCommand::OpenRegionSelector) {
                            error!("❌ Erro ao enviar comando: {}", e);
                        }
                    }

                    hotkey::HotkeyAction::SelectSubtitleRegion => {
                        info!("");
                        info!("📺 ============================================");
                        info!("📺 SOLICITANDO ABERTURA DO SELETOR DE LEGENDA");
                        info!("📺 ============================================");

                        if let Err(e) = state
                            .command_sender
                            .send(AppCommand::OpenSubtitleRegionSelector)
                        {
                            error!("❌ Erro ao enviar comando: {}", e);
                        }
                    }

                    hotkey::HotkeyAction::ToggleSubtitleMode => {
                        let mut active = state.subtitle_mode_active.lock().unwrap();
                        *active = !*active;

                        info!("");
                        if *active {
                            info!("📺 ============================================");
                            info!("📺 MODO LEGENDA: ✅ ATIVADO");
                            info!("📺 ============================================");
                        } else {
                            info!("📺 ============================================");
                            info!("📺 MODO LEGENDA: ❌ DESATIVADO");
                            info!("📺 ============================================");
                        }
                    }

                    hotkey::HotkeyAction::TranslateFullScreen => {
                        info!("");
                        info!("▶️  ============================================");
                        info!("▶️  MODO: 🖥️  TELA INTEIRA");
                        info!("▶️  ============================================");

                        let state_clone = state.clone();
                        thread::spawn(move || {
                            if let Err(e) = process_translation_blocking(
                                &state_clone,
                                hotkey::HotkeyAction::TranslateFullScreen,
                            ) {
                                error!("❌ Erro: {}", e);
                            }
                        });
                    }

                    hotkey::HotkeyAction::TranslateRegion => {
                        info!("");
                        info!("▶️  ============================================");
                        info!("▶️  MODO: 🎯 REGIÃO CUSTOMIZADA");
                        info!("▶️  ============================================");

                        let state_clone = state.clone();
                        thread::spawn(move || {
                            if let Err(e) = process_translation_blocking(
                                &state_clone,
                                hotkey::HotkeyAction::TranslateRegion,
                            ) {
                                error!("❌ Erro: {}", e);
                            }
                        });
                    }
                }
            }

            thread::sleep(Duration::from_millis(50));
        }
    });
}

// ============================================================================
// THREAD DE CONFIG WATCHER (monitora mudanças no config.json)
// ============================================================================

fn start_config_watcher(state: AppState) {
    thread::spawn(move || {
        info!("👁️  Thread de monitoramento do config.json iniciada");

        let (tx, rx) = channel();

        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                error!("❌ Erro ao criar watcher: {}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(Path::new("config.json"), RecursiveMode::NonRecursive) {
            error!("❌ Erro ao monitorar config.json: {}", e);
            return;
        }

        info!("✅ Monitorando config.json para mudanças...");

        let mut last_reload = std::time::Instant::now();
        let debounce_duration = Duration::from_millis(500);

        loop {
            match rx.recv() {
                Ok(event_result) => {
                    if let Ok(event) = event_result {
                        if matches!(event.kind, notify::EventKind::Modify(_)) {
                            if last_reload.elapsed() < debounce_duration {
                                continue;
                            }

                            last_reload = std::time::Instant::now();

                            info!("");
                            info!("🔄 CONFIG.JSON MODIFICADO - RECARREGANDO");

                            thread::sleep(Duration::from_millis(100));

                            match Config::load() {
                                Ok(new_config) => {
                                    let mut config = state.config.lock().unwrap();
                                    *config = new_config;
                                    info!("✅ Configurações recarregadas!");
                                }
                                Err(e) => {
                                    error!("❌ Erro ao recarregar config: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Erro ao receber evento: {}", e);
                    break;
                }
            }
        }
    });
}

// ============================================================================
// PROCESSAMENTO DE TRADUÇÃO
// ============================================================================

fn process_translation_blocking(state: &AppState, action: hotkey::HotkeyAction) -> Result<()> {
    // === ESCONDE O OVERLAY ANTES DE CAPTURAR ===
    {
        let mut hidden = state.overlay_hidden.lock().unwrap();
        *hidden = true;
    }
    thread::sleep(Duration::from_millis(100));
    // Verifica se usa modo memória (mais rápido) ou arquivo (debug)
    let use_memory = state
        .config
        .lock()
        .unwrap()
        .app_config
        .display
        .use_memory_capture;

    info!("📸 [1/4] Capturando tela...");

    // OCR result vai ser preenchido de acordo com o modo
    let ocr_result = if use_memory {
        // ====================================================================
        // MODO MEMÓRIA (RÁPIDO) - Não salva arquivo em disco
        // ====================================================================
        let image = match action {
            hotkey::HotkeyAction::TranslateRegion => {
                let (x, y, w, h) = {
                    let config = state.config.lock().unwrap();
                    (
                        config.region_x,
                        config.region_y,
                        config.region_width,
                        config.region_height,
                    )
                };
                info!("   🎯 Região: {}x{} em ({}, {}) [MEMÓRIA]", w, h, x, y);
                screenshot::capture_region_to_memory(x, y, w, h)?
            }
            hotkey::HotkeyAction::TranslateFullScreen => {
                info!("   🖥️  Tela inteira [MEMÓRIA]");
                screenshot::capture_screen_to_memory()?
            }
            hotkey::HotkeyAction::SelectRegion
            | hotkey::HotkeyAction::SelectSubtitleRegion
            | hotkey::HotkeyAction::ToggleSubtitleMode => {
                anyhow::bail!("Esta ação não deveria chamar process_translation")
            }
        };

        info!("✅ Screenshot capturada em memória!");
        info!("🔍 [2/4] Executando OCR...");
        ocr::extract_text_from_memory(&image)?
    } else {
        // ====================================================================
        // MODO ARQUIVO (DEBUG) - Salva screenshot.png em disco
        // ====================================================================
        let screenshot_path = PathBuf::from("screenshot.png");

        match action {
            hotkey::HotkeyAction::TranslateRegion => {
                let (x, y, w, h) = {
                    let config = state.config.lock().unwrap();
                    (
                        config.region_x,
                        config.region_y,
                        config.region_width,
                        config.region_height,
                    )
                };
                info!("   🎯 Região: {}x{} em ({}, {}) [ARQUIVO]", w, h, x, y);
                screenshot::capture_region(&screenshot_path, x, y, w, h)?;
            }
            hotkey::HotkeyAction::TranslateFullScreen => {
                info!("   🖥️  Tela inteira [ARQUIVO]");
                screenshot::capture_screen(&screenshot_path)?;
            }
            hotkey::HotkeyAction::SelectRegion => {
                anyhow::bail!("SelectRegion não deveria chamar process_translation")
            }
            hotkey::HotkeyAction::SelectSubtitleRegion
            | hotkey::HotkeyAction::ToggleSubtitleMode => {
                unreachable!("Esta ação não deveria chamar process_translation")
            }
        };

        info!("✅ Screenshot capturada!");
        info!("🔍 [2/4] Executando OCR...");
        ocr::extract_text_with_positions(&screenshot_path)?
    };

    if ocr_result.lines.is_empty() {
        info!("⚠️  Nenhum texto detectado!");
        return Ok(());
    }

    info!("   📍 {} linhas detectadas", ocr_result.lines.len());

    // Extrai textos para traduzir
    let texts_to_translate: Vec<String> = ocr_result
        .lines
        .iter()
        .map(|line| line.text.clone())
        .collect();

    // Tradução em batch
    info!("🌐 [3/4] Traduzindo {} textos...", texts_to_translate.len());

    let (api_key, provider, source_lang, target_lang, libre_url) = {
        // ← ADICIONOU libre_url
        let config = state.config.lock().unwrap();
        (
            config.deepl_api_key.clone(),
            config.app_config.translation.provider.clone(),
            config.app_config.translation.source_language.clone(),
            config.app_config.translation.target_language.clone(),
            config.app_config.translation.libretranslate_url.clone(),
        )
    };

    // Verifica quais textos já estão no cache
    let (cached, not_cached) = state.translation_cache.get_batch(
        &provider,
        &source_lang,
        &target_lang,
        &texts_to_translate,
    );

    info!(
        "   📦 Cache: {} encontrados, {} novos",
        cached.len(),
        not_cached.len()
    );

    // Prepara vetor de resultados
    let mut translated_texts: Vec<String> = vec![String::new(); texts_to_translate.len()];

    // Preenche com os que estavam no cache
    for (index, translated) in &cached {
        translated_texts[*index] = translated.clone();
    }

    // Traduz apenas os que não estavam no cache
    if !not_cached.is_empty() {
        let texts_to_api: Vec<String> = not_cached.iter().map(|(_, t)| t.clone()).collect();

        let runtime = tokio::runtime::Runtime::new()?;
        let new_translations = runtime.block_on(async {
            translator::translate_batch_with_provider(
                &texts_to_api,
                &provider,
                &api_key,
                &source_lang,
                &target_lang,
                Some(&libre_url), // ← ADICIONE ESSA LINHA
            )
            .await
        })?;

        // Preenche os resultados e adiciona ao cache
        let mut cache_pairs: Vec<(String, String)> = Vec::new();

        for (i, (original_index, original_text)) in not_cached.iter().enumerate() {
            if let Some(translated) = new_translations.get(i) {
                translated_texts[*original_index] = translated.clone();
                cache_pairs.push((original_text.clone(), translated.clone()));
            }
        }

        // Salva no cache
        state
            .translation_cache
            .set_batch(&provider, &source_lang, &target_lang, &cache_pairs);

        // Salva cache em disco periodicamente
        let _ = state.translation_cache.save_to_disk();
    }

    let (cache_total, cache_size) = state.translation_cache.stats();
    info!(
        "✅ Tradução concluída! (Cache: {} entradas, {} bytes)",
        cache_total, cache_size
    );

    // Monta lista com posições
    let (region_x, region_y) = {
        let config = state.config.lock().unwrap();
        (config.region_x as f64, config.region_y as f64)
    };

    let translated_items: Vec<TranslatedText> = ocr_result
        .lines
        .iter()
        .zip(translated_texts.iter())
        .map(|(detected, translated)| TranslatedText {
            original: detected.text.clone(),
            translated: translated.clone(),
            screen_x: detected.x + region_x,
            screen_y: detected.y + region_y,
            width: detected.width,
            height: detected.height,
        })
        .collect();

    // Define a região de captura (para posicionar o overlay)
    let capture_region = match action {
        hotkey::HotkeyAction::TranslateRegion => {
            let config = state.config.lock().unwrap();
            CaptureRegion {
                x: config.region_x,
                y: config.region_y,
                width: config.region_width,
                height: config.region_height,
            }
        }
        hotkey::HotkeyAction::TranslateFullScreen => {
            // Tela inteira: usa a região do config para o overlay
            let config = state.config.lock().unwrap();
            CaptureRegion {
                x: config.app_config.overlay.x,
                y: config.app_config.overlay.y,
                width: config.app_config.overlay.width,
                height: config.app_config.overlay.height,
            }
        }
        _ => unreachable!(),
    };

    // Envia para o overlay
    info!("🖼️  [4/4] Exibindo traduções...");
    state.set_translations(translated_items, capture_region);

    // ========================================================================
    // TTS - Fala a tradução (se configurado)
    // ========================================================================
    let (elevenlabs_key, elevenlabs_voice, tts_enabled) = {
        let config = state.config.lock().unwrap();
        (
            config.elevenlabs_api_key.clone(),
            config.elevenlabs_voice_id.clone(),
            // TTS só ativa se: está habilitado no config E tem API key E tem voice ID
            config.app_config.display.tts_enabled
                && !config.elevenlabs_api_key.is_empty()
                && !config.elevenlabs_voice_id.is_empty(),
        )
    };

    if tts_enabled {
        info!("🔊 [5/5] Sintetizando voz...");

        // Junta as traduções para falar (com espaço, não ponto)
        // Isso mantém o texto contínuo como um parágrafo natural
        let text_to_speak: String = translated_texts
            .iter()
            .filter(|t| !t.is_empty())
            .cloned()
            .collect::<Vec<String>>()
            .join(" ");

        if !text_to_speak.is_empty() {
            // Executa TTS em thread separada para não bloquear
            let key = elevenlabs_key.clone();
            let voice = elevenlabs_voice.clone();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Err(e) = tts::speak(&text_to_speak, &key, &voice).await {
                        error!("❌ Erro no TTS: {}", e);
                    }
                });
            });
        }
    } else {
        info!("🔇 [5/5] TTS desabilitado (configure ELEVENLABS_API_KEY e ELEVENLABS_VOICE_ID no .env)");
    }

    info!("✅ Completo!");
    info!("");

    // === MOSTRA O OVERLAY DE NOVO ===
    {
        let mut hidden = state.overlay_hidden.lock().unwrap();
        *hidden = false;
    }

    Ok(())
}

// ============================================================================
// THREAD DE LEGENDAS (captura contínua)
// ============================================================================

fn start_subtitle_thread(state: AppState) {
    thread::spawn(move || {
        info!("📺 Thread de legendas iniciada (aguardando ativação)");

        // Timeout em segundos (sem texto = esconde legendas)
        let timeout_secs: u64 = 5;

        loop {
            // Verifica se o modo legenda está ativo
            let is_active = *state.subtitle_mode_active.lock().unwrap();

            if is_active {
                // Verifica timeout (sem texto por X segundos)
                if state.subtitle_state.has_subtitles()
                    && state.subtitle_state.is_timed_out(timeout_secs)
                {
                    state.subtitle_state.reset();
                }

                // Pega configurações da região de legenda
                let (region_x, region_y, region_w, region_h, interval_ms) = {
                    let config = state.config.lock().unwrap();
                    (
                        config.app_config.subtitle.region.x,
                        config.app_config.subtitle.region.y,
                        config.app_config.subtitle.region.width,
                        config.app_config.subtitle.region.height,
                        config.app_config.subtitle.capture_interval_ms,
                    )
                };

                // Captura a região da legenda
                match screenshot::capture_region_to_memory(region_x, region_y, region_w, region_h) {
                    Ok(image) => {
                        // Executa OCR
                        match ocr::extract_text_from_memory(&image) {
                            Ok(ocr_result) => {
                                // Junta todo o texto detectado
                                let full_text = ocr_result.full_text.trim().to_string();

                                // Se detectou texto, atualiza o tempo
                                if full_text.len() >= 3 {
                                    state.subtitle_state.update_detection_time();
                                }

                                // Processa o texto detectado
                                if let Some(text_to_translate) =
                                    state.subtitle_state.process_detected_text(&full_text)
                                {
                                    // Texto mudou! Traduz
                                    let state_clone = state.clone();

                                    thread::spawn(move || {
                                        if let Err(e) = process_subtitle_translation(
                                            &state_clone,
                                            &text_to_translate,
                                        ) {
                                            error!("❌ Erro ao traduzir legenda: {}", e);
                                        }
                                    });
                                }
                            }
                            Err(e) => {
                                // OCR falhou silenciosamente (pode ser região sem texto)
                                trace!("OCR falhou: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ Erro ao capturar região de legenda: {}", e);
                    }
                }

                // Aguarda o intervalo configurado
                thread::sleep(Duration::from_millis(interval_ms));
            } else {
                // Modo inativo - aguarda um pouco antes de verificar novamente
                thread::sleep(Duration::from_millis(500));
            }
        }
    });
}

/// Processa a tradução de uma legenda
fn process_subtitle_translation(state: &AppState, text: &str) -> anyhow::Result<()> {
    info!("📺 Traduzindo legenda: \"{}\"", text);

    // Pega configurações de tradução
    let (api_key, provider, source_lang, target_lang, libre_url) = {
        let config = state.config.lock().unwrap();
        (
            config.deepl_api_key.clone(),
            config.app_config.translation.provider.clone(),
            config.app_config.translation.source_language.clone(),
            config.app_config.translation.target_language.clone(),
            config.app_config.translation.libretranslate_url.clone(),
        )
    };

    // Verifica cache primeiro
    if let Some(cached) = state
        .translation_cache
        .get(&provider, &source_lang, &target_lang, text)
    {
        info!("   📦 Cache hit!");
        state.subtitle_state.add_translated_subtitle(cached);
        return Ok(());
    }

    // Traduz via API
    let runtime = tokio::runtime::Runtime::new()?;
    let translated = runtime.block_on(async {
        translator::translate_batch_with_provider(
            &[text.to_string()],
            &provider,
            &api_key,
            &source_lang,
            &target_lang,
            Some(&libre_url),
        )
        .await
    })?;

    if let Some(translated_text) = translated.first() {
        info!("   ✅ Traduzido: \"{}\"", translated_text);

        // Salva no cache
        state
            .translation_cache
            .set(&provider, &source_lang, &target_lang, text, translated_text);

        // Adiciona ao histórico de legendas
        state
            .subtitle_state
            .add_translated_subtitle(translated_text.clone());
    }

    Ok(())
}
// ============================================================================
// FUNÇÃO PRINCIPAL
// ============================================================================

fn main() -> Result<()> {
    env_logger::init();

    info!("🎮 ============================================");
    info!("🎮 GAME TRANSLATOR - Tradutor para Jogos");
    info!("🎮 ============================================");
    info!("");

    // Carrega configurações
    let config = Config::load()?;

    // Cria canal de comunicação
    let (command_sender, command_receiver) = unbounded::<AppCommand>();

    // Cria estado compartilhado
    let state = AppState::new(config, command_sender);

    // Inicia threads
    start_hotkey_thread(state.clone());
    start_config_watcher(state.clone());
    start_subtitle_thread(state.clone());

    info!("✅ Sistema pronto!");
    info!("   Numpad - = Tela inteira");
    info!("   Numpad + = Região customizada");
    info!("   Numpad * = Selecionar região");
    info!("");

    // Configurações do overlay
    let config = state.config.lock().unwrap();
    let overlay_width = config.app_config.overlay.width as f32;
    let overlay_height = config.app_config.overlay.height as f32;
    let display_duration = config.app_config.display.overlay_duration_secs;
    drop(config);

    // Opções da janela
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([overlay_width, overlay_height])
            .with_position([0.0, 0.0])
            .with_always_on_top()
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true),
        ..Default::default()
    };

    // Inicia o overlay
    let _ = eframe::run_native(
        "Game Translator",
        options,
        Box::new(move |cc| {
            // Configura visual transparente
            let mut visuals = eframe::egui::Visuals::dark();
            visuals.panel_fill = eframe::egui::Color32::TRANSPARENT;
            visuals.window_fill = eframe::egui::Color32::TRANSPARENT;
            cc.egui_ctx.set_visuals(visuals);

            Ok(Box::new(OverlayApp {
                state: state.clone(),
                display_duration: Duration::from_secs(display_duration),
                command_receiver,
            }) as Box<dyn eframe::App>)
        }),
    );

    Ok(())
}

// ============================================================================
// FUNÇÃO PARA TORNAR JANELA CLICK-THROUGH (WINDOWS)
// ============================================================================

#[cfg(windows)]
fn make_window_click_through() {
    use winapi::um::winuser::{
        FindWindowW, GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };

    unsafe {
        // Encontra a janela pelo título
        let title: Vec<u16> = "Game Translator\0".encode_utf16().collect();
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());

        if !hwnd.is_null() {
            // Pega o estilo atual
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);

            // Adiciona WS_EX_LAYERED e WS_EX_TRANSPARENT para click-through
            let new_style = ex_style | WS_EX_LAYERED as i32 | WS_EX_TRANSPARENT as i32;
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);

            info!("✅ Janela configurada como click-through!");
        } else {
            warn!("⚠️  Não foi possível encontrar a janela para click-through");
        }
    }
}
