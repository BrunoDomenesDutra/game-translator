// game-translator/src/main.rs

// ============================================================================
// GAME TRANSLATOR - Aplicação para traduzir textos de jogos em tempo real
// ============================================================================

#[macro_use]
extern crate log;

// ============================================================================
// DECLARAÇÃO DE MÓDULOS
// ============================================================================
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
#[derive(Clone)]
struct AppState {
    config: Arc<Mutex<Config>>, // <-- Agora é Mutex para poder recarregar
    current_translation: Arc<Mutex<Option<String>>>,
    translation_timestamp: Arc<Mutex<Option<std::time::Instant>>>,
    command_sender: Sender<AppCommand>, // <-- Canal de comandos
}

impl AppState {
    fn new(config: Config, command_sender: Sender<AppCommand>) -> Self {
        AppState {
            config: Arc::new(Mutex::new(config)),
            current_translation: Arc::new(Mutex::new(None)),
            translation_timestamp: Arc::new(Mutex::new(None)),
            command_sender,
        }
    }

    fn set_translation(&self, text: String) {
        *self.current_translation.lock().unwrap() = Some(text);
        *self.translation_timestamp.lock().unwrap() = Some(std::time::Instant::now());
    }

    fn get_translation(&self) -> Option<(String, std::time::Instant)> {
        let text = self.current_translation.lock().unwrap().clone()?;
        let timestamp = self.translation_timestamp.lock().unwrap().clone()?;
        Some((text, timestamp))
    }

    fn clear_translation(&self) {
        *self.current_translation.lock().unwrap() = None;
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

                            // Atualiza o config
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

                                // Atualiza os atalhos de retrocompatibilidade
                                config.region_x = selected.x;
                                config.region_y = selected.y;
                                config.region_width = selected.width;
                                config.region_height = selected.height;
                            }
                        }
                        Ok(None) => {
                            info!("❌ Seleção cancelada");
                        }
                        Err(e) => {
                            error!("❌ Erro no seletor: {}", e);
                        }
                    }
                }
            }
        }

        // Verifica se há tradução para exibir
        let should_display = if let Some((text, timestamp)) = self.state.get_translation() {
            let elapsed = timestamp.elapsed();
            elapsed < self.display_duration
        } else {
            false
        };

        if should_display {
            // ====================================================================
            // HÁ TRADUÇÃO: Janela visível e no tamanho normal
            // ====================================================================

            if let Some((text, timestamp)) = self.state.get_translation() {
                let elapsed = timestamp.elapsed();

                // Garante posição e tamanho corretos (do config.json)
                let config = self.state.config.lock().unwrap();
                let overlay_x = config.app_config.overlay.x as f32;
                let overlay_y = config.app_config.overlay.y as f32;
                let overlay_width = config.app_config.overlay.width as f32;
                let overlay_height = config.app_config.overlay.height as f32;
                drop(config); // Libera o lock

                // Reposiciona
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                    eframe::egui::pos2(overlay_x, overlay_y),
                ));

                // Redimensiona para tamanho normal
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                    eframe::egui::vec2(overlay_width, overlay_height),
                ));

                // Renderiza o conteúdo
                self.render_translation(ctx, &text, elapsed);

                // Verifica se o tempo acabou
                if elapsed >= self.display_duration {
                    self.state.clear_translation();
                }
            }
        } else {
            // ====================================================================
            // SEM TRADUÇÃO: Janela minúscula (1x1 pixel) e transparente
            // ====================================================================

            // Reduz para 1x1 pixel (praticamente invisível)
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                eframe::egui::vec2(1.0, 1.0),
            ));

            // Move para canto superior esquerdo (discreto)
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                eframe::egui::pos2(0.0, 0.0),
            ));

            // Painel vazio e completamente transparente
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
                .show(ctx, |_ui| {});
        }

        // Repaint contínuo
        ctx.request_repaint();
    }
}

impl OverlayApp {
    fn render_translation(&self, ctx: &eframe::egui::Context, text: &str, elapsed: Duration) {
        // ═══════════════════════════════════════════════════════════
        // PAINEL CENTRAL - A "tela" onde tudo será desenhado
        // ═══════════════════════════════════════════════════════════
        eframe::egui::CentralPanel::default()
            .frame(eframe::egui::Frame::none()) // Remove bordas padrão
            .show(ctx, |ui| {
                // ───────────────────────────────────────────────────
                // FUNDO PRETO SEMI-TRANSPARENTE
                // ───────────────────────────────────────────────────
                let rect = ui.max_rect(); // Pega o tamanho total da janela

                ui.painter().rect_filled(
                    rect, // Onde desenhar (janela inteira)
                    0.0,  // Raio das bordas arredondadas (0 = quadrado)
                    eframe::egui::Color32::from_rgba_unmultiplied(
                        0,   // Red (0 = sem vermelho)
                        0,   // Green (0 = sem verde)
                        0,   // Blue (0 = sem azul)
                        235, // Alpha (0-255, onde 255 = opaco, 0 = invisível)
                    ),
                );

                // ═══════════════════════════════════════════════════════════
                // LAYOUT VERTICAL - Organiza elementos de cima para baixo
                // ═══════════════════════════════════════════════════════════
                ui.vertical(|ui| {
                    // ───────────────────────────────────────────────────
                    // MARGEM SUPERIOR (espaço do topo da janela)
                    // ───────────────────────────────────────────────────
                    ui.add_space(20.0); // 20 pixels de espaço vazio no topo

                    // ═══════════════════════════════════════════════════════════
                    // LAYOUT HORIZONTAL - Cria padding esquerdo e direito
                    // ═══════════════════════════════════════════════════════════
                    ui.horizontal(|ui| {
                        // ───────────────────────────────────────────────────
                        // PADDING ESQUERDO (margem lateral esquerda)
                        // ───────────────────────────────────────────────────
                        ui.add_space(25.0); // 25 pixels vazios à esquerda

                        // ═══════════════════════════════════════════════════════════
                        // CONTEÚDO PRINCIPAL - Coluna interna com texto
                        // ═══════════════════════════════════════════════════════════
                        ui.vertical(|ui| {
                            // ───────────────────────────────────────────────────
                            // TEXTO DA TRADUÇÃO
                            // ───────────────────────────────────────────────────
                            // Pega o tamanho da fonte do config
                            let font_size = self.state.config.lock().unwrap().app_config.font.size;

                            // Define espaçamento entre linhas
                            let line_spacing = font_size * 0.2; // 20% do tamanho da fonte

                            ui.spacing_mut().item_spacing.y = line_spacing;

                            ui.add(
                                eframe::egui::Label::new(
                                    eframe::egui::RichText::new(text)
                                        .color(eframe::egui::Color32::WHITE)
                                        .size(font_size)
                                        .line_height(Some(font_size * 1.2)), // 1.2x o tamanho da fonte
                                )
                                .wrap_mode(eframe::egui::TextWrapMode::Wrap),
                            );

                            // ───────────────────────────────────────────────────
                            // ESPAÇO ENTRE TEXTO E CONTADOR
                            // ───────────────────────────────────────────────────
                            // ui.add_space(5.0); // 10 pixels entre tradução e contador

                            // ───────────────────────────────────────────────────
                            // CONTADOR REGRESSIVO
                            // ───────────────────────────────────────────────────
                            // let remaining = (self.display_duration - elapsed).as_secs();

                            // ui.label(
                            //     eframe::egui::RichText::new(format!(
                            //         "⏱ {} segundos",
                            //         remaining + 1
                            //     ))
                            //     .color(eframe::egui::Color32::from_rgb(150, 150, 150)) // Cinza
                            //     .size(14.0), // Fonte menor que o texto principal
                            // );
                        });

                        // ───────────────────────────────────────────────────
                        // PADDING DIREITO (margem lateral direita)
                        // ───────────────────────────────────────────────────
                        ui.add_space(25.0); // 25 pixels vazios à direita
                    });
                });
            });
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
            // Verifica se alguma hotkey foi pressionada
            if let Some(action) = hotkey_manager.check_hotkey() {
                match action {
                    hotkey::HotkeyAction::SelectRegion => {
                        info!("");
                        info!("🎯 ============================================");
                        info!("🎯 SOLICITANDO ABERTURA DO SELETOR DE REGIÃO");
                        info!("🎯 ============================================");

                        // Envia comando para main thread abrir o seletor
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

                // Aguarda tecla ser solta
                // hotkey_manager.wait_for_key_release();
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

        // Canal para receber eventos do notify
        let (tx, rx) = channel();

        // Cria o watcher
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                error!("❌ Erro ao criar watcher: {}", e);
                return;
            }
        };

        // Monitora o arquivo config.json
        if let Err(e) = watcher.watch(Path::new("config.json"), RecursiveMode::NonRecursive) {
            error!("❌ Erro ao monitorar config.json: {}", e);
            return;
        }

        info!("✅ Monitorando config.json para mudanças...");

        // Loop aguardando eventos
        // Variável para debounce (ignorar eventos muito próximos)
        let mut last_reload = std::time::Instant::now();
        let debounce_duration = Duration::from_millis(500); // Ignora eventos em 500ms

        // Loop aguardando eventos
        loop {
            match rx.recv() {
                Ok(event_result) => {
                    match event_result {
                        Ok(event) => {
                            // Verifica se foi modificação
                            if matches!(event.kind, notify::EventKind::Modify(_)) {
                                // Debounce: ignora se recarregou há menos de 500ms
                                if last_reload.elapsed() < debounce_duration {
                                    continue;
                                }

                                last_reload = std::time::Instant::now();

                                info!("");
                                info!("🔄 ============================================");
                                info!("🔄 CONFIG.JSON MODIFICADO - RECARREGANDO");
                                info!("🔄 ============================================");

                                // Pequeno delay para garantir que o arquivo foi salvo completamente
                                thread::sleep(Duration::from_millis(100));

                                // Recarrega o config
                                match Config::load() {
                                    Ok(new_config) => {
                                        let mut config = state.config.lock().unwrap();
                                        *config = new_config;
                                        drop(config);

                                        info!("✅ Configurações recarregadas com sucesso!");
                                        info!("   O overlay será atualizado na próxima tradução");
                                        info!("🔄 ============================================");
                                        info!("");
                                    }
                                    Err(e) => {
                                        error!("❌ Erro ao recarregar config: {}", e);
                                        error!("   Mantendo configurações antigas");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("❌ Erro no evento do watcher: {}", e);
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
// PROCESSAMENTO DE TRADUÇÃO (versão bloqueante para thread)
// ============================================================================
fn process_translation_blocking(state: &AppState, action: hotkey::HotkeyAction) -> Result<()> {
    info!("📸 [1/5] Capturando tela...");

    let screenshot_path = PathBuf::from("screenshot.png");

    let _image = match action {
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
            info!(
                "   🎯 Capturando região: {}x{} na posição ({}, {})",
                w, h, x, y
            );
            screenshot::capture_region(&screenshot_path, x, y, w, h)?
        }
        hotkey::HotkeyAction::TranslateFullScreen => {
            info!("   🖥️  Capturando tela inteira");
            screenshot::capture_screen(&screenshot_path)?
        }
        hotkey::HotkeyAction::SelectRegion => {
            anyhow::bail!("SelectRegion não deveria chamar process_translation")
        }
    };

    info!("✅ Screenshot capturada!");

    info!("🔍 [2/5] Executando OCR (com posições)...");
    let ocr_result = ocr::extract_text_with_positions(&screenshot_path)?;

    if ocr_result.lines.is_empty() {
        info!("⚠️  Nenhum texto detectado!");
        return Ok(());
    }

    // Log das posições detectadas
    info!("   📍 {} linhas detectadas:", ocr_result.lines.len());
    for (i, line) in ocr_result.lines.iter().enumerate() {
        info!(
            "      [{}] \"{}\" @ ({:.0}, {:.0})",
            i, line.text, line.x, line.y
        );
    }

    // Extrai só os textos para traduzir (mantendo a ordem!)
    let texts_to_translate: Vec<String> = ocr_result
        .lines
        .iter()
        .map(|line| line.text.clone())
        .collect();

    info!(
        "🌐 [3/5] Traduzindo {} textos em batch...",
        texts_to_translate.len()
    );

    // Tradução em batch (uma única chamada para todos os textos!)
    let api_key = state.config.lock().unwrap().deepl_api_key.clone();
    let runtime = tokio::runtime::Runtime::new()?;
    let translated_texts = runtime
        .block_on(async { translator::translate_batch(&texts_to_translate, &api_key).await })?;

    // Log das traduções
    info!("✅ Traduções recebidas:");
    for (i, translated) in translated_texts.iter().enumerate() {
        let original = &ocr_result.lines[i].text;
        if original != translated {
            info!("      [{}] \"{}\" → \"{}\"", i, original, translated);
        } else {
            info!("      [{}] \"{}\" (sem mudança)", i, original);
        }
    }

    // Por enquanto, junta tudo para o overlay existente
    // (depois vamos mudar para múltiplos overlays)
    let translated_text = translated_texts.join("\n");

    info!("✅ Texto traduzido:");
    info!("   🇧🇷 {}", translated_text);

    info!("🖼️  [4/5] Enviando para overlay...");
    state.set_translation(translated_text);
    info!("✅ Enviado!");

    info!("✅ Processo completo!");
    info!("▶️  ============================================");
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
    info!("📋 Configurações:");
    info!("   🎯 Jogo: Judgment (Yakuza)");
    info!("   🌐 Tradução: DeepL (EN → PT-BR)");
    info!("   🔊 Voz: ElevenLabs");
    info!("   ⌨️  Hotkeys:");
    info!("      - Numpad - (menos) = Tela inteira");
    info!("      - Numpad + (mais)  = Região customizada");
    info!("");

    info!("⚙️  Configurando sistema...");

    // Carrega configurações
    let config = Config::load()?;

    // Cria canal de comunicação
    let (command_sender, command_receiver) = unbounded::<AppCommand>();

    // Cria estado compartilhado
    let state = AppState::new(config, command_sender);

    // Inicia thread de hotkeys
    start_hotkey_thread(state.clone());

    // Inicia thread de monitoramento do config
    start_config_watcher(state.clone());

    info!("✅ Sistema pronto!");
    info!("");
    info!("🎯 Pressione Numpad - para capturar TELA INTEIRA");
    info!("🎯 Pressione Numpad + para capturar REGIÃO customizada");
    info!("🎯 Pressione Ctrl+C para sair");
    info!("");

    // ========================================================================
    // INICIA OVERLAY NA MAIN THREAD
    // ========================================================================
    let overlay_x = state.config.lock().unwrap().app_config.overlay.x as f32;
    let overlay_y = state.config.lock().unwrap().app_config.overlay.y as f32;
    let overlay_width = state.config.lock().unwrap().app_config.overlay.width as f32;
    let overlay_height = state.config.lock().unwrap().app_config.overlay.height as f32;

    info!("🖼️  Configurando overlay:");
    info!("   Posição: ({}, {})", overlay_x, overlay_y);
    info!("   Tamanho: {}x{}", overlay_width, overlay_height);

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([overlay_width, overlay_height])
            .with_position([overlay_x, overlay_y])
            .with_always_on_top()
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true),

        ..Default::default()
    };

    // ========================================================================
    // CONFIGURAÇÃO E CARREGAMENTO DE FONTES
    // ========================================================================
    let state_for_fonts = state.clone();
    let display_duration = state
        .config
        .lock()
        .unwrap()
        .app_config
        .display
        .overlay_duration_secs;

    let command_receiver_for_app = command_receiver;

    let _ = eframe::run_native(
        "Game Translator Overlay",
        options,
        Box::new(move |cc| {
            // ================================================================
            // Carrega fonte customizada se configurado
            // ================================================================
            let config = state_for_fonts.config.lock().unwrap();
            if config.app_config.font.font_type == "file" {
                let font_path = &config.app_config.font.file_path;

                match std::fs::read(font_path) {
                    Ok(font_data) => {
                        info!("✅ Carregando fonte customizada: {}", font_path);

                        let mut fonts = eframe::egui::FontDefinitions::default();

                        // Adiciona a fonte customizada
                        fonts.font_data.insert(
                            "custom_font".to_owned(),
                            eframe::egui::FontData::from_owned(font_data),
                        );

                        // Define como fonte padrão
                        fonts.families.insert(
                            eframe::egui::FontFamily::Proportional,
                            vec!["custom_font".to_owned()],
                        );

                        cc.egui_ctx.set_fonts(fonts);
                    }
                    Err(e) => {
                        warn!("⚠️  Erro ao carregar fonte {}: {}", font_path, e);
                        warn!("   Usando fonte padrão do sistema");
                    }
                }
            }

            drop(config); // Libera o lock antes de criar o app

            // ================================================================
            // Cria o app do overlay
            // ================================================================
            Ok(Box::new(OverlayApp {
                state: state_for_fonts.clone(),
                display_duration: Duration::from_secs(display_duration),
                command_receiver: command_receiver_for_app,
            }) as Box<dyn eframe::App>)
        }),
    );

    Ok(())
}
