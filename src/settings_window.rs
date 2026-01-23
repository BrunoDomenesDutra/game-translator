// game-translator/src/settings_window.rs

// ============================================================================
// MÓDULO SETTINGS WINDOW - Janela de configurações
// ============================================================================
//
// Este módulo cria uma janela separada para editar as configurações do
// config.json de forma visual, sem precisar editar o arquivo manualmente.
//
// ============================================================================

use crate::config::AppConfig;
use eframe::egui;
use std::sync::{Arc, Mutex};

// ============================================================================
// ESTRUTURA DA JANELA DE CONFIGURAÇÕES
// ============================================================================

/// Janela de configurações
pub struct SettingsWindow {
    /// Configurações sendo editadas (cópia local)
    config: AppConfig,
    /// Referência para salvar as alterações
    config_path: String,
    /// Aba atual selecionada
    current_tab: SettingsTab,
    /// Mensagem de status (sucesso/erro)
    status_message: Option<(String, std::time::Instant)>,
}

/// Abas disponíveis na janela de configurações
#[derive(Debug, Clone, PartialEq)]
enum SettingsTab {
    Overlay,
    Font,
    Display,
    Subtitle,
}

impl Default for SettingsTab {
    fn default() -> Self {
        SettingsTab::Overlay
    }
}

impl SettingsWindow {
    /// Cria uma nova janela de configurações
    pub fn new(config: AppConfig) -> Self {
        SettingsWindow {
            config,
            config_path: "config.json".to_string(),
            current_tab: SettingsTab::default(),
            status_message: None,
        }
    }

    /// Define uma mensagem de status temporária
    fn set_status(&mut self, message: String) {
        self.status_message = Some((message, std::time::Instant::now()));
    }

    /// Salva as configurações no arquivo
    fn save_config(&mut self) {
        match self.config.save() {
            Ok(_) => {
                self.set_status("✅ Configurações salvas com sucesso!".to_string());
                info!("💾 Configurações salvas pela janela de settings");
            }
            Err(e) => {
                self.set_status(format!("❌ Erro ao salvar: {}", e));
                error!("❌ Erro ao salvar configurações: {}", e);
            }
        }
    }

    /// Renderiza a aba de Overlay
    fn render_overlay_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🖼️ Overlay");
        ui.add_space(10.0);

        // Show Background
        ui.checkbox(
            &mut self.config.overlay.show_background,
            "Mostrar fundo do overlay",
        );
        ui.label("   Se desativado, mostra apenas texto com contorno");

        ui.add_space(20.0);
    }

    /// Renderiza a aba de Font
    fn render_font_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔤 Fonte (Modo Região/Tela Cheia)");
        ui.add_space(10.0);

        // Font Size
        ui.horizontal(|ui| {
            ui.label("Tamanho da fonte:");
            ui.add(egui::Slider::new(&mut self.config.font.size, 12.0..=72.0).suffix("px"));
        });

        ui.add_space(10.0);

        // Outline
        ui.checkbox(&mut self.config.font.outline.enabled, "Contorno ativado");

        if self.config.font.outline.enabled {
            ui.horizontal(|ui| {
                ui.label("   Espessura do contorno:");
                let mut width = self.config.font.outline.width as i32;
                if ui
                    .add(egui::Slider::new(&mut width, 1..=10).suffix("px"))
                    .changed()
                {
                    self.config.font.outline.width = width as u32;
                }
            });
        }

        ui.add_space(20.0);
    }

    /// Renderiza a aba de Display (PreProcess)
    fn render_display_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🖥️ Display - Pré-processamento OCR");
        ui.add_space(10.0);

        ui.label("Configurações de pré-processamento de imagem para o OCR:");
        ui.add_space(5.0);

        // Enabled
        ui.checkbox(
            &mut self.config.display.preprocess.enabled,
            "Pré-processamento ativado",
        );

        if self.config.display.preprocess.enabled {
            ui.add_space(10.0);
            ui.indent("preprocess_indent", |ui| {
                // Grayscale
                ui.checkbox(
                    &mut self.config.display.preprocess.grayscale,
                    "Converter para escala de cinza",
                );

                // Invert
                ui.checkbox(&mut self.config.display.preprocess.invert, "Inverter cores");

                // Contrast
                ui.horizontal(|ui| {
                    ui.label("Contraste:");
                    ui.add(
                        egui::Slider::new(&mut self.config.display.preprocess.contrast, 0.5..=10.0)
                            .suffix("x"),
                    );
                });

                // Threshold
                ui.horizontal(|ui| {
                    ui.label("Threshold (0 = desativado):");
                    let mut threshold = self.config.display.preprocess.threshold as i32;
                    if ui.add(egui::Slider::new(&mut threshold, 0..=255)).changed() {
                        self.config.display.preprocess.threshold = threshold as u8;
                    }
                });

                // Save Debug Image
                ui.checkbox(
                    &mut self.config.display.preprocess.save_debug_image,
                    "Salvar imagem de debug",
                );
            });
        }

        ui.add_space(20.0);
    }

    /// Renderiza a aba de Subtitle
    fn render_subtitle_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("📺 Legendas");
        ui.add_space(10.0);

        // Capture Interval
        ui.horizontal(|ui| {
            ui.label("Intervalo de captura:");
            let mut interval = self.config.subtitle.capture_interval_ms as i32;
            if ui
                .add(egui::Slider::new(&mut interval, 50..=2000).suffix("ms"))
                .changed()
            {
                self.config.subtitle.capture_interval_ms = interval as u64;
            }
        });

        // Max Lines
        ui.horizontal(|ui| {
            ui.label("Máximo de linhas:");
            let mut lines = self.config.subtitle.max_lines as i32;
            if ui.add(egui::Slider::new(&mut lines, 1..=10)).changed() {
                self.config.subtitle.max_lines = lines as usize;
            }
        });

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);

        // Font Settings
        ui.label("🔤 Fonte das legendas:");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("   Tamanho:");
            ui.add(
                egui::Slider::new(&mut self.config.subtitle.font.size, 12.0..=72.0).suffix("px"),
            );
        });

        ui.checkbox(
            &mut self.config.subtitle.font.outline.enabled,
            "   Contorno ativado",
        );

        if self.config.subtitle.font.outline.enabled {
            ui.horizontal(|ui| {
                ui.label("      Espessura:");
                let mut width = self.config.subtitle.font.outline.width as i32;
                if ui
                    .add(egui::Slider::new(&mut width, 1..=10).suffix("px"))
                    .changed()
                {
                    self.config.subtitle.font.outline.width = width as u32;
                }
            });
        }

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);

        // PreProcess Settings
        ui.label("🔧 Pré-processamento OCR (Legendas):");
        ui.add_space(5.0);

        ui.checkbox(
            &mut self.config.subtitle.preprocess.enabled,
            "   Pré-processamento ativado",
        );

        if self.config.subtitle.preprocess.enabled {
            ui.indent("subtitle_preprocess", |ui| {
                ui.checkbox(
                    &mut self.config.subtitle.preprocess.grayscale,
                    "Escala de cinza",
                );

                ui.checkbox(
                    &mut self.config.subtitle.preprocess.invert,
                    "Inverter cores",
                );

                ui.horizontal(|ui| {
                    ui.label("Contraste:");
                    ui.add(
                        egui::Slider::new(
                            &mut self.config.subtitle.preprocess.contrast,
                            0.5..=10.0,
                        )
                        .suffix("x"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Threshold:");
                    let mut threshold = self.config.subtitle.preprocess.threshold as i32;
                    if ui.add(egui::Slider::new(&mut threshold, 0..=255)).changed() {
                        self.config.subtitle.preprocess.threshold = threshold as u8;
                    }
                });

                ui.checkbox(
                    &mut self.config.subtitle.preprocess.save_debug_image,
                    "Salvar imagem de debug",
                );
            });
        }

        ui.add_space(20.0);
    }
}

impl eframe::App for SettingsWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Painel principal
        egui::CentralPanel::default().show(ctx, |ui| {
            // Título
            ui.horizontal(|ui| {
                ui.heading("⚙️ Game Translator - Configurações");
            });

            ui.add_space(10.0);

            // Abas
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.current_tab == SettingsTab::Overlay, "🖼️ Overlay")
                    .clicked()
                {
                    self.current_tab = SettingsTab::Overlay;
                }
                if ui
                    .selectable_label(self.current_tab == SettingsTab::Font, "🔤 Fonte")
                    .clicked()
                {
                    self.current_tab = SettingsTab::Font;
                }
                if ui
                    .selectable_label(self.current_tab == SettingsTab::Display, "🖥️ Display")
                    .clicked()
                {
                    self.current_tab = SettingsTab::Display;
                }
                if ui
                    .selectable_label(self.current_tab == SettingsTab::Subtitle, "📺 Legendas")
                    .clicked()
                {
                    self.current_tab = SettingsTab::Subtitle;
                }
            });

            ui.separator();
            ui.add_space(10.0);

            // Conteúdo da aba selecionada
            egui::ScrollArea::vertical().show(ui, |ui| match self.current_tab {
                SettingsTab::Overlay => self.render_overlay_tab(ui),
                SettingsTab::Font => self.render_font_tab(ui),
                SettingsTab::Display => self.render_display_tab(ui),
                SettingsTab::Subtitle => self.render_subtitle_tab(ui),
            });

            // Barra inferior com botões
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(10.0);

                // Mensagem de status
                if let Some((msg, time)) = &self.status_message {
                    if time.elapsed() < std::time::Duration::from_secs(3) {
                        ui.label(msg);
                    }
                }

                ui.horizontal(|ui| {
                    if ui.button("💾 Salvar").clicked() {
                        self.save_config();
                    }

                    if ui.button("🔄 Recarregar").clicked() {
                        match AppConfig::load() {
                            Ok(config) => {
                                self.config = config;
                                self.set_status("🔄 Configurações recarregadas!".to_string());
                            }
                            Err(e) => {
                                self.set_status(format!("❌ Erro ao recarregar: {}", e));
                            }
                        }
                    }
                });

                ui.add_space(5.0);
            });
        });
    }
}

// ============================================================================
// FUNÇÃO PARA ABRIR A JANELA DE CONFIGURAÇÕES
// ============================================================================

/// Abre a janela de configurações em uma thread separada
pub fn open_settings_window(config: AppConfig) {
    info!("⚙️  Iniciando thread da janela de configurações...");

    std::thread::spawn(move || {
        info!("⚙️  Thread iniciada, criando janela...");

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([500.0, 600.0])
                .with_min_inner_size([400.0, 400.0])
                .with_title("Game Translator - Configurações"),
            ..Default::default()
        };

        info!("⚙️  Chamando eframe::run_native...");

        let result = eframe::run_native(
            "Game Translator Settings",
            options,
            Box::new(|_cc| Ok(Box::new(SettingsWindow::new(config)))),
        );

        match result {
            Ok(_) => info!("⚙️  Janela de configurações fechada normalmente"),
            Err(e) => error!("❌ Erro ao abrir janela de configurações: {}", e),
        }
    });
}
