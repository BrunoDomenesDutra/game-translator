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

                // Garante posição e tamanho corretos
                let overlay_x = self.state.config.region_x as f32;
                let overlay_y = (self.state.config.region_y as i32 - 250).max(0) as f32;
                let overlay_width = self.state.config.region_width as f32;
                let overlay_height = 200.0;

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
        eframe::egui::CentralPanel::default()
            .frame(eframe::egui::Frame::none())
            .show(ctx, |ui| {
                // Fundo semi-transparente
                let rect = ui.max_rect();
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    eframe::egui::Color32::from_rgba_unmultiplied(0, 0, 0, 235),
                );

                ui.vertical_centered(|ui| {
                    ui.add_space(25.0);

                    // Texto da tradução
                    ui.label(
                        eframe::egui::RichText::new(text)
                            .color(eframe::egui::Color32::WHITE)
                            .size(36.0),
                    );

                    ui.add_space(15.0);

                    // Contador regressivo
                    let remaining = (self.display_duration - elapsed).as_secs();
                    ui.label(
                        eframe::egui::RichText::new(format!("⏱ {} segundos", remaining + 1))
                            .color(eframe::egui::Color32::from_rgb(150, 150, 150))
                            .size(14.0),
                    );
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
            if let Some(capture_mode) = hotkey_manager.check_hotkey() {
                info!("");
                info!("▶️  ============================================");

                match capture_mode {
                    hotkey::CaptureMode::FullScreen => {
                        info!("▶️  MODO: 🖥️  TELA INTEIRA");
                    }
                    hotkey::CaptureMode::Region => {
                        info!("▶️  MODO: 🎯 REGIÃO CUSTOMIZADA");
                    }
                }

                info!("▶️  ============================================");

                // Processa tradução
                let state_clone = state.clone();
                thread::spawn(move || {
                    if let Err(e) = process_translation_blocking(&state_clone, capture_mode) {
                        error!("❌ Erro: {}", e);
                    }
                });

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
fn process_translation_blocking(state: &AppState, capture_mode: hotkey::CaptureMode) -> Result<()> {
    info!("📸 [1/5] Capturando tela...");

    let screenshot_path = PathBuf::from("screenshot.png");

    let _image = match capture_mode {
        hotkey::CaptureMode::Region => {
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
        hotkey::CaptureMode::FullScreen => {
            info!("   🖥️  Capturando tela inteira");
            screenshot::capture_screen(&screenshot_path)?
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
    let overlay_x = state.config.region_x as f32;
    let overlay_y = (state.config.region_y as i32 - 250).max(0) as f32;
    let overlay_width = state.config.region_width as f32;
    let overlay_height = 200.0;

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

    let app = OverlayApp {
        state: state.clone(),
        display_duration: Duration::from_secs(5),
    };

    let _ = eframe::run_native(
        "Game Translator Overlay",
        options,
        Box::new(move |_cc| Ok(Box::new(app))),
    );

    Ok(())
}
