// game-translator/src/settings_ui/history_openai_tabs.rs

use crate::config;
use crate::subtitle;
use std::sync::{Arc, Mutex};

pub(super) fn render_history_tab(ui: &mut eframe::egui::Ui, subtitle_state: &subtitle::SubtitleState) {
    ui.heading("Historico de Legendas");
    ui.add_space(10.0);

    // --- Controles ---
    super::full_width_group(ui, |ui| {
        let history = subtitle_state.get_full_history();
        ui.label(format!("{} legendas no historico", history.len()));

        ui.add_space(5.0);
        if ui.button("Limpar historico").clicked() {
            subtitle_state.clear_full_history();
        }
    });

    ui.add_space(10.0);

    // --- Lista de legendas ---
    super::full_width_group(ui, |ui| {
        ui.label("Legendas traduzidas:");
        ui.add_space(5.0);

        let history = subtitle_state.get_full_history();

        if history.is_empty() {
            ui.label("Nenhuma legenda traduzida ainda.");
            ui.label("Ative o modo legenda para comecar.");
        } else {
            // Lista as legendas (mais recente em cima)
            for (i, entry) in history.iter().rev().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(eframe::egui::RichText::new(format!("{}.", i + 1)).weak());
                    ui.label(&entry.translated);
                });

                if i < history.len() - 1 {
                    ui.separator();
                }
            }
        }
    });
}

pub(super) fn render_openai_tab(
    ui: &mut eframe::egui::Ui,
    cfg: &mut config::AppConfig,
    openai_request_count: &Arc<Mutex<u32>>,
) {
    ui.heading("OpenAI - Traducao com IA");
    ui.add_space(10.0);

    // --- API Key ---
    super::full_width_group(ui, |ui| {
        ui.label("Autenticacao:");
        ui.add_space(5.0);

        eframe::egui::Grid::new("openai_auth_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("API Key:");
                ui.add(
                    eframe::egui::TextEdit::singleline(&mut cfg.openai.api_key)
                        .password(true)
                        .desired_width(350.0),
                );
                ui.end_row();
            });

        ui.add_space(3.0);
        if cfg.openai.api_key.is_empty() {
            ui.label("Necessario para usar OpenAI como provedor");
        } else {
            ui.label("Configurado");
        }
    });

    ui.add_space(10.0);

    // --- Modelo e parametros ---
    super::full_width_group(ui, |ui| {
        ui.label("Modelo e Parametros:");
        ui.add_space(5.0);

        let models = vec!["gpt-4o-mini", "gpt-5-mini", "gpt-5-nano"];

        eframe::egui::Grid::new("openai_model_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Modelo:");
                eframe::egui::ComboBox::from_id_source("openai_model")
                    .selected_text(&cfg.openai.model)
                    .show_ui(ui, |ui| {
                        for m in &models {
                            ui.selectable_value(&mut cfg.openai.model, m.to_string(), *m);
                        }
                    });
                ui.end_row();

                ui.label("Temperature:");
                ui.add(
                    eframe::egui::Slider::new(&mut cfg.openai.temperature, 0.0..=2.0).step_by(0.1),
                );
                ui.end_row();

                ui.label("Max tokens:");
                let mut tokens = cfg.openai.max_tokens as i32;
                if ui
                    .add(eframe::egui::Slider::new(&mut tokens, 128..=4096))
                    .changed()
                {
                    cfg.openai.max_tokens = tokens as u32;
                }
                ui.end_row();
            });

        ui.add_space(3.0);
        ui.label("Temperature: 0.0 = literal, 0.3 = recomendado, 1.0+ = criativo");
    });

    ui.add_space(10.0);

    // --- Controle de custo ---
    super::full_width_group(ui, |ui| {
        ui.label("Controle de Custo:");
        ui.add_space(5.0);

        eframe::egui::Grid::new("openai_cost_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Limite requests/sessao:");
                let mut limit = cfg.openai.max_requests_per_session as i32;
                if ui
                    .add(eframe::egui::Slider::new(&mut limit, 0..=2000).suffix(" req"))
                    .changed()
                {
                    cfg.openai.max_requests_per_session = limit as u32;
                }
                ui.end_row();

                ui.label("Fallback ao atingir limite:");
                let fallbacks = vec!["google", "deepl", "libretranslate"];
                eframe::egui::ComboBox::from_id_source("openai_fallback")
                    .selected_text(&cfg.openai.fallback_provider)
                    .show_ui(ui, |ui| {
                        for f in &fallbacks {
                            ui.selectable_value(
                                &mut cfg.openai.fallback_provider,
                                f.to_string(),
                                *f,
                            );
                        }
                    });
                ui.end_row();
            });

        ui.add_space(5.0);
        ui.label("0 = ilimitado");

        // Status de uso
        ui.add_space(5.0);
        let count = *openai_request_count.lock().unwrap();
        let limit = cfg.openai.max_requests_per_session;
        let status_text = if limit == 0 {
            format!("Requests nesta sessao: {} (sem limite)", count)
        } else {
            format!("Requests nesta sessao: {} / {}", count, limit)
        };
        ui.label(status_text);

        if count > 0 {
            if ui.button("Resetar contador").clicked() {
                *openai_request_count.lock().unwrap() = 0;
            }
        }
    });

    ui.add_space(10.0);

    // --- Contexto de conversa ---
    super::full_width_group(ui, |ui| {
        ui.label("Contexto de Conversa:");
        ui.add_space(5.0);

        eframe::egui::Grid::new("openai_context_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Falas anteriores:");
                let mut lines = cfg.openai.context_lines as i32;
                if ui
                    .add(eframe::egui::Slider::new(&mut lines, 0..=20).suffix(" falas"))
                    .changed()
                {
                    cfg.openai.context_lines = lines as u32;
                }
                ui.end_row();
            });

        ui.add_space(3.0);
        ui.label("0 = desativado. Recomendado: 3-5 falas");
        ui.label("Envia legendas anteriores pra IA manter coerencia");

        ui.add_space(5.0);
        ui.label("Informacoes do jogo (ajuda a IA com nomes e termos):");
        ui.add(
            eframe::egui::TextEdit::singleline(&mut cfg.openai.game_context)
                .desired_width(ui.available_width() - 8.0)
                .hint_text("Ex: Judgment - jogo de detetive yakuza em Kamurocho, Japao"),
        );
    });

    ui.add_space(10.0);

    // --- System Prompt ---
    super::full_width_group(ui, |ui| {
        ui.label("System Prompt (instrucao para a IA):");
        ui.add_space(5.0);
        ui.label("Define o tom, estilo e regras da traducao:");
        ui.add_space(5.0);

        // ScrollArea dentro do grupo pro prompt nao ficar gigante
        eframe::egui::ScrollArea::vertical()
            .max_height(250.0)
            .show(ui, |ui| {
                ui.add(
                    eframe::egui::TextEdit::multiline(&mut cfg.openai.system_prompt)
                        .desired_width(ui.available_width() - 8.0)
                        .desired_rows(12)
                        .font(eframe::egui::TextStyle::Monospace),
                );
            });

        ui.add_space(5.0);
        if ui.button("Restaurar prompt padrao").clicked() {
            cfg.openai.system_prompt = config::default_openai_system_prompt();
        }
    });

    ui.add_space(10.0);

    // --- Dica de uso ---
    super::full_width_group(ui, |ui| {
        ui.label("Como usar:");
        ui.add_space(3.0);
        ui.label("1. Cole sua API key da OpenAI acima");
        ui.label("2. Na aba Servicos, selecione 'openai' como provedor");
        ui.label("3. Ajuste o prompt conforme o jogo que esta traduzindo");
        ui.label("4. Salve as configuracoes");
    });
}
