#![windows_subsystem = "windows"]

// game-translator/src/main.rs

// ============================================================================
// GAME TRANSLATOR - Aplicação para traduzir textos de jogos em tempo real
// ============================================================================

#[macro_use]
extern crate log;

// ============================================================================
// DECLARAÇÃO DE MÓDULOS
// ============================================================================
mod app_state;
mod cache;
mod config;
mod hotkey;
mod ocr;
mod overlay;
mod platform;
mod processing;
mod region_selector;
mod runtime;
mod screenshot;
mod settings_ui;
mod subtitle;
mod translator;
mod tts;

// ============================================================================
// IMPORTS
// ============================================================================
use anyhow::Result;
use app_state::{AppCommand, AppState};
use config::Config;
use crossbeam_channel::{unbounded, Receiver};
use std::time::Duration;

// ============================================================================
// APLICAÇÃO DE OVERLAY (roda na main thread)
// ============================================================================

struct OverlayApp {
    state: AppState,
    display_duration: Duration,
    command_receiver: Receiver<AppCommand>,
    /// Cópia local das configurações para edição
    settings_config: Option<config::AppConfig>,
    /// Aba atual das configurações
    settings_tab: u8,
    /// Mensagem de status
    settings_status: Option<(String, std::time::Instant)>,
    /// Se já posicionou a janela de configurações (evita forçar posição todo frame)
    settings_positioned: bool,
    /// Textura da imagem debug de pré-processamento (preview em tempo real)
    debug_texture: Option<eframe::egui::TextureHandle>,
    /// Quando a textura debug foi atualizada pela última vez
    debug_texture_last_update: std::time::Instant,
    /// Textura da imagem original do laboratório
    lab_original_texture: Option<eframe::egui::TextureHandle>,
    /// Textura da imagem processada do laboratório
    lab_processed_texture: Option<eframe::egui::TextureHandle>,
    /// Configuração de pré-processamento do laboratório (independente)
    lab_preprocess: Option<crate::config::PreprocessConfig>,
    /// Nome do arquivo selecionado no laboratório
    lab_selected_file: Option<String>,
    /// Imagem original carregada (pra não reler do disco toda hora)
    lab_original_image: Option<image::DynamicImage>,
    /// Flag que indica que os parâmetros mudaram e precisa reprocessar
    lab_needs_reprocess: bool,
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // Totalmente transparente
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // ====================================================================
        // TORNA A JANELA CLICK-THROUGH (apenas uma vez)
        // ====================================================================
        #[cfg(windows)]
        {
            // Verifica se está no modo configurações
            let is_settings = *self.state.settings_mode.lock().unwrap();

            // ====================================================================
            // VERIFICA SE PRECISA RECARREGAR A FONTE DE TRADUÇÃO
            // ====================================================================
            overlay::fonts::reload_translation_font_if_needed(ctx, &self.state);
            platform::windows_overlay::apply_click_through_mode(is_settings);
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
        overlay::commands::process_pending_commands(
            ctx,
            &self.state,
            &self.command_receiver,
            &mut self.settings_config,
            &mut self.settings_tab,
            &mut self.settings_status,
            &mut self.settings_positioned,
        );
        // ====================================================================
        if overlay::settings_window::render_settings_window(
            ctx,
            &self.state,
            &mut self.settings_config,
            &mut self.settings_tab,
            &mut self.settings_status,
            &mut self.settings_positioned,
            &mut self.debug_texture,
            &mut self.debug_texture_last_update,
            &mut self.lab_original_texture,
            &mut self.lab_processed_texture,
            &mut self.lab_preprocess,
            &mut self.lab_selected_file,
            &mut self.lab_original_image,
            &mut self.lab_needs_reprocess,
        ) {
            return;
        }

        // ====================================================================
        // PREVIEW DE ÁREAS DE LEGENDA (captura + área de exibição)
        // ====================================================================
        if overlay::render::render_subtitle_areas_preview(ctx, &self.state) {
            return;
        }

        // ====================================================================
        // MODO LEGENDA: Exibe histórico de legendas acima da região
        // ====================================================================
        if overlay::render::render_subtitle_history_overlay(ctx, &self.state) {
            return;
        } else if overlay::render::render_translations_overlay(
            ctx,
            &self.state,
            self.display_duration,
        ) {
            // Traduções renderizadas em overlay::render
        } else {
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
// FUNÇÃO PRINCIPAL
// ============================================================================

fn main() -> Result<()> {
    // ================================================================
    // GARANTE QUE SÓ UMA INSTÂNCIA DO PROGRAMA RODE
    // ================================================================
    // Cria um Named Mutex no Windows. Se já existe, outra instância
    // está rodando e este processo encerra imediatamente.
    let _single_instance = unsafe {
        use widestring::U16CString;
        use winapi::shared::winerror::ERROR_ALREADY_EXISTS;
        use winapi::um::errhandlingapi::GetLastError;
        use winapi::um::synchapi::CreateMutexW;
        use winapi::um::winuser::{MessageBoxW, MB_ICONWARNING, MB_OK};

        // Cria a string wide corretamente
        let mutex_name = U16CString::from_str("Global\\RanmzaGameTranslatorMutex").unwrap();

        let handle = CreateMutexW(
            std::ptr::null_mut(),
            0,
            mutex_name.as_ptr(), // ✅ correto
        );

        if GetLastError() == ERROR_ALREADY_EXISTS {
            eprintln!("Ranmza Game Translator ja esta em execucao!");

            let title = U16CString::from_str("Ranmza Game Translator").unwrap();
            let msg = U16CString::from_str(
            "O Ranmza Game Translator ja esta em execucao.\nApenas uma instancia pode rodar por vez.",
        )
        .unwrap();

            MessageBoxW(
                std::ptr::null_mut(),
                msg.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONWARNING,
            );

            std::process::exit(0);
        }

        handle
    };

    // Declara que o programa é DPI-aware (Per-Monitor V2)
    // Sem isso, o Windows "mente" e diz que o DPI é 96 (100%)
    // mesmo quando o usuário tem 125%, 150%, etc.
    unsafe {
        winapi::um::shellscalingapi::SetProcessDpiAwareness(2); // 2 = Per-Monitor DPI Aware
    }

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
    let dpi = unsafe { winapi::um::winuser::GetDpiForSystem() };
    let dpi_scale = dpi as f32 / 96.0;
    info!(
        "📐 DPI do sistema: {} (escala: {}%)",
        dpi,
        (dpi_scale * 100.0) as u32
    );

    let state = AppState::new(config, command_sender, dpi_scale);

    // Inicia threads
    runtime::hotkeys::start_hotkey_thread(state.clone());
    runtime::config_watcher::start_config_watcher(state.clone());
    processing::start_subtitle_thread(state.clone());

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
    // Carrega ícone da janela (icon.png ao lado do executável)
    // Ícone embutido no binário (compilado junto com o .exe)
    // O arquivo icon.png deve estar na raiz do projeto (ao lado do Cargo.toml)
    let window_icon = match eframe::icon_data::from_png_bytes(include_bytes!("../icon.png")) {
        Ok(icon) => {
            info!("✅ Ícone carregado do binário");
            Some(std::sync::Arc::new(icon))
        }
        Err(e) => {
            warn!("⚠️  Erro ao carregar ícone embutido: {}", e);
            None
        }
    };

    // Opções da janela
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([overlay_width, overlay_height])
        .with_position([0.0, 0.0])
        .with_always_on_top()
        .with_decorations(false)
        .with_resizable(false)
        .with_transparent(true);

    // Aplica ícone se carregou
    if let Some(icon) = window_icon {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    // Inicia o overlay
    let _ = eframe::run_native(
        "Ranmza Game Translator",
        options,
        Box::new(move |cc| {
            // Configura visual transparente
            let mut visuals = eframe::egui::Visuals::dark();
            visuals.panel_fill = eframe::egui::Color32::TRANSPARENT;
            visuals.window_fill = eframe::egui::Color32::TRANSPARENT;
            cc.egui_ctx.set_visuals(visuals);

            // ============================================================
            // CARREGAMENTO DE FONTES
            // ============================================================
            // 1. Roboto-Regular embutida no binário = fonte da UI (menus, config)
            // 2. Fonte de tradução = lida da pasta fonts/ (configurável)
            // Se a fonte de tradução não existir, usa Roboto como fallback
            {
                let mut fonts = eframe::egui::FontDefinitions::default();

                // --- Roboto embutida (UI do programa) ---
                let roboto_data = include_bytes!("../fonts/Roboto-Regular.ttf");
                fonts.font_data.insert(
                    "roboto".to_owned(),
                    eframe::egui::FontData::from_static(roboto_data),
                );

                // Roboto como fonte principal da UI (Proportional)
                fonts
                    .families
                    .get_mut(&eframe::egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "roboto".to_owned());

                info!("🔤 Fonte da UI: Roboto-Regular (embutida)");

                // --- Fonte de tradução (da pasta fonts/) ---
                let translation_font_name = {
                    let config = state.config.lock().unwrap();
                    config.app_config.font.translation_font.clone()
                };

                let translation_font_path =
                    std::path::Path::new("fonts").join(&translation_font_name);

                let translation_loaded =
                    if !translation_font_name.is_empty() && translation_font_path.exists() {
                        match std::fs::read(&translation_font_path) {
                            Ok(font_data) => {
                                fonts.font_data.insert(
                                    "translation".to_owned(),
                                    eframe::egui::FontData::from_owned(font_data),
                                );
                                info!(
                                    "🔤 Fonte de tradução: {} (pasta fonts/)",
                                    translation_font_name
                                );
                                true
                            }
                            Err(e) => {
                                error!(
                                    "❌ Erro ao carregar fonte '{}': {}",
                                    translation_font_name, e
                                );
                                false
                            }
                        }
                    } else {
                        if !translation_font_name.is_empty() {
                            warn!(
                                "⚠️  Fonte '{}' não encontrada em fonts/",
                                translation_font_name
                            );
                        }
                        false
                    };

                // Registra família "translation" no egui
                // Se carregou a fonte custom, usa ela; senão, usa Roboto como fallback
                let translation_family = eframe::egui::FontFamily::Name("translation".into());
                if translation_loaded {
                    fonts.families.insert(
                        translation_family,
                        vec!["translation".to_owned(), "roboto".to_owned()],
                    );
                } else {
                    info!("🔤 Fonte de tradução: usando Roboto (fallback)");
                    fonts
                        .families
                        .insert(translation_family, vec!["roboto".to_owned()]);
                }

                cc.egui_ctx.set_fonts(fonts);
            }

            Ok(Box::new(OverlayApp {
                state: state.clone(),
                display_duration: Duration::from_secs(display_duration),
                command_receiver,
                settings_config: None,
                settings_tab: 0,
                settings_status: None,
                settings_positioned: false,
                debug_texture: None,
                debug_texture_last_update: std::time::Instant::now(),
                lab_original_texture: None,
                lab_processed_texture: None,
                lab_preprocess: None,
                lab_selected_file: None,
                lab_original_image: None,
                lab_needs_reprocess: false,
                // last_window_size: (0.0, 0.0),
            }) as Box<dyn eframe::App>)
        }),
    );

    Ok(())
}


