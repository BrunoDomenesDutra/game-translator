// game-translator/src/settings_ui/services_tab.rs

use crate::config;

pub(super) fn render_services_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Servicos de Traducao e Voz");
    ui.add_space(10.0);

    // --- Provedor de traducao ---
    super::full_width_group(ui, |ui| {
        ui.label("Provedor de Traducao:");
        ui.add_space(5.0);

        let providers = vec!["google", "deepl", "libretranslate", "openai"];

        eframe::egui::Grid::new("services_provider_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Provedor ativo:");
                eframe::egui::ComboBox::from_id_source("translation_provider")
                    .selected_text(&cfg.translation.provider)
                    .show_ui(ui, |ui| {
                        for p in &providers {
                            ui.selectable_value(&mut cfg.translation.provider, p.to_string(), *p);
                        }
                    });
                ui.end_row();

                ui.label("Idioma origem:");
                ui.add(
                    eframe::egui::TextEdit::singleline(&mut cfg.translation.source_language)
                        .desired_width(200.0),
                );
                ui.end_row();

                ui.label("Idioma destino:");
                ui.add(
                    eframe::egui::TextEdit::singleline(&mut cfg.translation.target_language)
                        .desired_width(200.0),
                );
                ui.end_row();
            });
    });

    ui.add_space(10.0);

    // --- DeepL ---
    super::full_width_group(ui, |ui| {
        ui.label("DeepL:");
        ui.add_space(5.0);

        eframe::egui::Grid::new("services_deepl_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("API Key:");
                ui.add(
                    eframe::egui::TextEdit::singleline(&mut cfg.translation.deepl_api_key)
                        .password(true)
                        .desired_width(300.0),
                );
                ui.end_row();
            });

        ui.add_space(3.0);
        if cfg.translation.deepl_api_key.is_empty() {
            ui.label("Necessario para usar DeepL");
        } else {
            ui.label("Configurado");
        }
    });

    ui.add_space(10.0);

    // --- LibreTranslate ---
    super::full_width_group(ui, |ui| {
        ui.label("LibreTranslate:");
        ui.add_space(5.0);

        eframe::egui::Grid::new("services_libre_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("URL:");
                ui.add(
                    eframe::egui::TextEdit::singleline(&mut cfg.translation.libretranslate_url)
                        .desired_width(300.0),
                );
                ui.end_row();
            });

        ui.add_space(3.0);
        ui.label("Gratuito e offline (requer servidor local)");
    });

    ui.add_space(10.0);

    // --- Google ---
    super::full_width_group(ui, |ui| {
        ui.label("Google Translate:");
        ui.add_space(5.0);
        ui.label("Sem API key necessaria (usa API nao oficial)");
    });

    ui.add_space(10.0);

    // --- ElevenLabs TTS ---
    super::full_width_group(ui, |ui| {
        ui.label("ElevenLabs (Text-to-Speech):");
        ui.add_space(5.0);

        ui.checkbox(&mut cfg.display.tts_enabled, "TTS ativado");
        ui.add_space(5.0);

        eframe::egui::Grid::new("services_tts_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("API Key:");
                ui.add(
                    eframe::egui::TextEdit::singleline(&mut cfg.translation.elevenlabs_api_key)
                        .password(true)
                        .desired_width(300.0),
                );
                ui.end_row();

                ui.label("Voice ID:");
                ui.add(
                    eframe::egui::TextEdit::singleline(&mut cfg.translation.elevenlabs_voice_id)
                        .desired_width(300.0),
                );
                ui.end_row();
            });

        ui.add_space(3.0);
        if cfg.translation.elevenlabs_api_key.is_empty() {
            ui.label("Configure API Key e Voice ID para usar TTS");
        } else {
            ui.label("Configurado");
        }
    });
}
