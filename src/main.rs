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
}

impl AppState {
    fn new(config: Config, command_sender: Sender<AppCommand>) -> Self {
        // Cria cache com persistência em disco
        let translation_cache = cache::TranslationCache::new(true);

        AppState {
            config: Arc::new(Mutex::new(config)),
            translated_items: Arc::new(Mutex::new(Vec::new())),
            capture_region: Arc::new(Mutex::new(None)),
            translation_timestamp: Arc::new(Mutex::new(None)),
            command_sender,
            translation_cache,
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

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
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
            }
        }

        // ====================================================================
        // VERIFICA SE HÁ TRADUÇÕES PARA EXIBIR
        // ====================================================================
        let should_display = if let Some((_, _, timestamp)) = self.state.get_translations() {
            timestamp.elapsed() < self.display_duration
        } else {
            false
        };

        if should_display {
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
            hotkey::HotkeyAction::SelectRegion => {
                anyhow::bail!("SelectRegion não deveria chamar process_translation")
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

    let (api_key, provider, source_lang, target_lang) = {
        let config = state.config.lock().unwrap();
        (
            config.deepl_api_key.clone(),
            config.app_config.translation.provider.clone(),
            config.app_config.translation.source_language.clone(),
            config.app_config.translation.target_language.clone(),
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
