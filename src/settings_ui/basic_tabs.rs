// game-translator/src/settings_ui/basic_tabs.rs

use crate::config;
use crate::subtitle;

pub(super) fn render_overlay_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Overlay");
    ui.add_space(10.0);

    // --- Fundo ---
    super::full_width_group(ui, |ui| {
        ui.label("Aparencia:");
        ui.add_space(5.0);
        ui.checkbox(&mut cfg.overlay.show_background, "Mostrar fundo do overlay");
        ui.label("Se desativado, mostra apenas texto com contorno");
    });

    ui.add_space(10.0);

    // --- Duração ---
    super::full_width_group(ui, |ui| {
        ui.label("Exibicao:");
        ui.add_space(5.0);

        eframe::egui::Grid::new("overlay_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Duracao do overlay:");
                let mut duration = cfg.display.overlay_duration_secs as i32;
                if ui
                    .add(eframe::egui::Slider::new(&mut duration, 1..=60).suffix("s"))
                    .changed()
                {
                    cfg.display.overlay_duration_secs = duration as u64;
                }
                ui.end_row();
            });

        ui.add_space(5.0);
        ui.checkbox(
            &mut cfg.display.use_memory_capture,
            "Captura em memoria (mais rapido)",
        );
        ui.label("Se desativado, salva screenshot em disco (modo debug)");
    });
}

pub(super) fn render_font_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Fonte das Traducoes");
    ui.add_space(10.0);

    // --- Seleção de fonte ---
    super::full_width_group(ui, |ui| {
        ui.label("Fonte para traducoes (Display e Legendas):");
        ui.add_space(5.0);
        ui.label("Coloque arquivos .ttf na pasta 'fonts/' ao lado do executavel.");
        ui.add_space(5.0);

        // Lista arquivos .ttf da pasta fonts/
        let fonts_dir = std::path::Path::new("fonts");
        let mut font_files: Vec<String> = Vec::new();

        if fonts_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(fonts_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext.to_string_lossy().to_lowercase() == "ttf" {
                            if let Some(name) = path.file_name() {
                                font_files.push(name.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        font_files.sort();

        if font_files.is_empty() {
            ui.label("Nenhuma fonte .ttf encontrada na pasta 'fonts/'.");
        } else {
            eframe::egui::ComboBox::from_id_source("translation_font_selector")
                .selected_text(&cfg.font.translation_font)
                .show_ui(ui, |ui| {
                    for file in &font_files {
                        ui.selectable_value(&mut cfg.font.translation_font, file.clone(), file);
                    }
                });
        }

        ui.add_space(5.0);
        ui.label("A mesma fonte e usada no modo Display e Legendas.");
    });

    ui.add_space(10.0);

    // --- Tamanho ---
    super::full_width_group(ui, |ui| {
        ui.label("Tamanho da fonte (Display):");
        ui.add_space(5.0);

        eframe::egui::Grid::new("font_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Tamanho:");
                ui.add(eframe::egui::Slider::new(&mut cfg.font.size, 12.0..=72.0).suffix("px"));
                ui.end_row();
            });
    });

    ui.add_space(10.0);

    // --- Contorno ---
    super::full_width_group(ui, |ui| {
        ui.label("Contorno do texto:");
        ui.add_space(5.0);

        ui.checkbox(&mut cfg.font.outline.enabled, "Contorno ativado");

        if cfg.font.outline.enabled {
            ui.add_space(5.0);
            eframe::egui::Grid::new("font_outline_grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Espessura:");
                    let mut width = cfg.font.outline.width as i32;
                    if ui
                        .add(eframe::egui::Slider::new(&mut width, 1..=10).suffix("px"))
                        .changed()
                    {
                        cfg.font.outline.width = width as u32;
                    }
                    ui.end_row();
                });
        }
    });
}

pub(super) fn render_display_tab(
    ui: &mut eframe::egui::Ui,
    cfg: &mut config::AppConfig,
    debug_texture: &mut Option<eframe::egui::TextureHandle>,
    debug_texture_last_update: &mut std::time::Instant,
    subtitle_state: &subtitle::SubtitleState,
) {
    ui.heading("Pre-processamento OCR");
    ui.add_space(10.0);

    // --- Pré-processamento ---
    super::full_width_group(ui, |ui| {
        ui.label("Pre-processamento (Regiao/Tela Cheia):");
        ui.add_space(5.0);

        ui.checkbox(&mut cfg.display.preprocess.enabled, "Ativado");

        if cfg.display.preprocess.enabled {
            ui.add_space(5.0);
            super::render_preprocess_controls(ui, &mut cfg.display.preprocess, "preprocess");
        }
    });

    ui.add_space(10.0);

    // --- Preview da imagem debug ---
    if cfg.display.preprocess.save_debug_image {
        super::render_debug_preview(ui, debug_texture, debug_texture_last_update, subtitle_state);
    }
}

pub(super) fn render_subtitle_tab(
    ui: &mut eframe::egui::Ui,
    cfg: &mut config::AppConfig,
    debug_texture: &mut Option<eframe::egui::TextureHandle>,
    debug_texture_last_update: &mut std::time::Instant,
    subtitle_state: &subtitle::SubtitleState,
) {
    ui.heading("Legendas");
    ui.add_space(10.0);

    // --- Configurações gerais ---
    super::full_width_group(ui, |ui| {
        ui.label("Captura de legendas:");
        ui.add_space(5.0);

        eframe::egui::Grid::new("subtitle_general_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Intervalo de captura:");
                let mut interval = cfg.subtitle.capture_interval_ms as i32;
                if ui
                    .add(eframe::egui::Slider::new(&mut interval, 50..=2000).suffix("ms"))
                    .changed()
                {
                    cfg.subtitle.capture_interval_ms = interval as u64;
                }
                ui.end_row();

                ui.label("Maximo de linhas:");
                let mut lines = cfg.subtitle.max_lines as i32;
                if ui
                    .add(eframe::egui::Slider::new(&mut lines, 1..=10))
                    .changed()
                {
                    cfg.subtitle.max_lines = lines as usize;
                }
                ui.end_row();

                ui.label("Timeout (sem texto):");
                let mut timeout = cfg.subtitle.max_display_secs as i32;
                if ui
                    .add(eframe::egui::Slider::new(&mut timeout, 1..=30).suffix("s"))
                    .changed()
                {
                    cfg.subtitle.max_display_secs = timeout as u64;
                }
                ui.end_row();
            });
    });

    ui.add_space(10.0);

    // --- Fonte das legendas ---
    super::full_width_group(ui, |ui| {
        ui.label("Fonte das legendas:");
        ui.add_space(5.0);

        eframe::egui::Grid::new("subtitle_font_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Tamanho:");
                ui.add(
                    eframe::egui::Slider::new(&mut cfg.subtitle.font.size, 12.0..=72.0)
                        .suffix("px"),
                );
                ui.end_row();
            });

        ui.add_space(5.0);
        ui.checkbox(&mut cfg.subtitle.font.outline.enabled, "Contorno ativado");

        if cfg.subtitle.font.outline.enabled {
            ui.add_space(5.0);
            eframe::egui::Grid::new("subtitle_outline_grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Espessura:");
                    let mut width = cfg.subtitle.font.outline.width as i32;
                    if ui
                        .add(eframe::egui::Slider::new(&mut width, 1..=10).suffix("px"))
                        .changed()
                    {
                        cfg.subtitle.font.outline.width = width as u32;
                    }
                    ui.end_row();
                });
        }
    });

    ui.add_space(10.0);

    // --- Pré-processamento OCR (Legendas) ---
    super::full_width_group(ui, |ui| {
        ui.label("Pre-processamento OCR (Legendas):");
        ui.add_space(5.0);

        ui.checkbox(&mut cfg.subtitle.preprocess.enabled, "Ativado");

        if cfg.subtitle.preprocess.enabled {
            ui.add_space(5.0);
            super::render_preprocess_controls(ui, &mut cfg.subtitle.preprocess, "sub_preprocess");
        }
    });

    ui.add_space(10.0);

    // --- Preview da imagem debug ---
    if cfg.subtitle.preprocess.save_debug_image {
        super::render_debug_preview(ui, debug_texture, debug_texture_last_update, subtitle_state);
    }
}
