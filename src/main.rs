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
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use config::Config;

// ============================================================================
// ESTRUTURA DE ESTADO COMPARTILHADO
// ============================================================================
/// Estado compartilhado entre a UI (overlay) e a thread de hotkeys
#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    current_translation: Arc<Mutex<Option<String>>>,
    translation_timestamp: Arc<Mutex<Option<std::time::Instant>>>,
}

impl AppState {
    fn new(config: Config) -> Self {
        AppState {
            config: Arc::new(config),
            current_translation: Arc::new(Mutex::new(None)),
            translation_timestamp: Arc::new(Mutex::new(None)),
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
}

impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
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
                let overlay_x = self.state.config.app_config.overlay.x as f32;
                let overlay_y = self.state.config.app_config.overlay.y as f32;
                let overlay_width = self.state.config.app_config.overlay.width as f32;
                let overlay_height = self.state.config.app_config.overlay.height as f32;

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
        ctx.request_repaint_after(Duration::from_millis(100));
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
                            ui.add(
                                eframe::egui::Label::new(
                                    eframe::egui::RichText::new(text)
                                        .color(eframe::egui::Color32::WHITE) // Cor do texto
                                        .size(30.0), // Tamanho da fonte em pixels
                                )
                                .wrap_mode(eframe::egui::TextWrapMode::Wrap), // Quebra linha em palavras
                            );

                            // ───────────────────────────────────────────────────
                            // ESPAÇO ENTRE TEXTO E CONTADOR
                            // ───────────────────────────────────────────────────
                            ui.add_space(10.0); // 10 pixels entre tradução e contador

                            // ───────────────────────────────────────────────────
                            // CONTADOR REGRESSIVO
                            // ───────────────────────────────────────────────────
                            let remaining = (self.display_duration - elapsed).as_secs();

                            ui.label(
                                eframe::egui::RichText::new(format!(
                                    "⏱ {} segundos",
                                    remaining + 1
                                ))
                                .color(eframe::egui::Color32::from_rgb(150, 150, 150)) // Cinza
                                .size(14.0), // Fonte menor que o texto principal
                            );
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

        let hotkey_manager = hotkey::HotkeyManager::new();

        loop {
            // Verifica se alguma hotkey foi pressionada
            if let Some(action) = hotkey_manager.check_hotkey() {
                match action {
                    hotkey::HotkeyAction::SelectRegion => {
                        info!("");
                        info!("🎯 ============================================");
                        info!("🎯 ABRINDO SELETOR DE REGIÃO");
                        info!("🎯 ============================================");

                        // Abre seletor (precisa ser na main thread - vamos resolver isso)
                        // Por enquanto, só avisa
                        info!("⚠️  Seletor de região em desenvolvimento...");
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
                hotkey_manager.wait_for_key_release();
            }

            thread::sleep(Duration::from_millis(50));
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
            info!(
                "   🎯 Capturando região: {}x{} na posição ({}, {})",
                state.config.region_width,
                state.config.region_height,
                state.config.region_x,
                state.config.region_y
            );
            screenshot::capture_region(
                &screenshot_path,
                state.config.region_x,
                state.config.region_y,
                state.config.region_width,
                state.config.region_height,
            )?
        }
        hotkey::HotkeyAction::TranslateFullScreen => {
            info!("   🖥️  Capturando tela inteira");
            screenshot::capture_screen(&screenshot_path)?
        }
        hotkey::HotkeyAction::SelectRegion => {
            // Não deve chegar aqui
            anyhow::bail!("SelectRegion não deveria chamar process_translation")
        }
    };

    info!("✅ Screenshot capturada!");

    info!("🔍 [2/5] Executando OCR...");
    let extracted_text = ocr::extract_text(&screenshot_path)?;

    if extracted_text.is_empty() {
        info!("⚠️  Nenhum texto detectado!");
        return Ok(());
    }

    info!("✅ Texto extraído:");
    info!("   📝 {}", extracted_text);

    info!("🌐 [3/5] Traduzindo texto...");

    // Tradução precisa ser assíncrona - vamos usar tokio runtime
    let runtime = tokio::runtime::Runtime::new()?;
    let translated_text = runtime.block_on(async {
        translator::translate(&extracted_text, &state.config.deepl_api_key).await
    })?;

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

    // Cria estado compartilhado
    let state = AppState::new(config);

    // Inicia thread de hotkeys
    start_hotkey_thread(state.clone());

    info!("✅ Sistema pronto!");
    info!("");
    info!("🎯 Pressione Numpad - para capturar TELA INTEIRA");
    info!("🎯 Pressione Numpad + para capturar REGIÃO customizada");
    info!("🎯 Pressione Ctrl+C para sair");
    info!("");

    // ========================================================================
    // INICIA OVERLAY NA MAIN THREAD
    // ========================================================================
    let overlay_x = state.config.app_config.overlay.x as f32;
    let overlay_y = state.config.app_config.overlay.y as f32;
    let overlay_width = state.config.app_config.overlay.width as f32;
    let overlay_height = state.config.app_config.overlay.height as f32;

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
    let display_duration = state.config.app_config.display.overlay_duration_secs;

    let _ = eframe::run_native(
        "Game Translator Overlay",
        options,
        Box::new(move |cc| {
            // ================================================================
            // Carrega fonte customizada se configurado
            // ================================================================
            if state_for_fonts.config.app_config.display.use_custom_font {
                let font_path = &state_for_fonts.config.app_config.display.font_file;

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

            // ================================================================
            // Cria o app do overlay
            // ================================================================
            Ok(Box::new(OverlayApp {
                state: state_for_fonts.clone(),
                display_duration: Duration::from_secs(display_duration),
            }) as Box<dyn eframe::App>)
        }),
    );

    Ok(())
}
