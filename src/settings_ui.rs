// game-translator/src/settings_ui.rs

// ============================================================================
// MÓDULO SETTINGS UI - Interface de configurações moderna
// ============================================================================
// Interface redesenhada com tema dark profissional, sidebar de navegação
// e cards estilizados. Mantém toda a funcionalidade existente.
// ============================================================================

use crate::config;
use crate::subtitle;
use eframe::egui::{self, Color32, Frame, Layout, Margin, RichText, Stroke};
use std::sync::{Arc, Mutex};

// Cores do tema (exatas conforme especificação)
const BG_DARK: Color32 = Color32::from_rgb(27, 27, 27); // #1B1B1B
const CARD_BG: Color32 = Color32::from_rgb(36, 36, 36); // #242424
const BORDER_COLOR: Color32 = Color32::from_rgb(50, 50, 50); // Subtle border
const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246); // #3B82F6

/// Renderiza a interface completa de configurações com sidebar e top bar
pub fn render_settings_window(
    ui: &mut egui::Ui,
    selected_tab: &mut usize,
    cfg: &mut config::AppConfig,
    subtitle_state: &subtitle::SubtitleState,
    openai_request_count: &Arc<Mutex<u32>>,
    save_callback: impl Fn(),
) {
    // Top bar com título e botão Save
    render_top_bar(ui, save_callback);

    ui.add_space(16.0);

    // Layout principal: sidebar + conteúdo
    ui.horizontal(|ui| {
        // Sidebar de navegação vertical
        render_sidebar(ui, selected_tab);

        ui.add_space(24.0);

        // Área de conteúdo principal com cards estilizados
        ui.vertical(|ui| {
            ui.set_width(580.0);
            render_content_area(ui, *selected_tab, cfg, subtitle_state, openai_request_count);
        });
    });
}

// ============================================================================
// TOP BAR - Título e botão Save
// ============================================================================
fn render_top_bar(ui: &mut egui::Ui, save_callback: impl Fn()) {
    Frame::none()
        .fill(Color32::from_rgb(30, 30, 30))
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .inner_margin(Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("Game Translator")
                            .color(Color32::from_rgb(240, 240, 240))
                            .size(20.0)
                            .strong(),
                    );
                });

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Salvar Configurações")
                                    .color(Color32::WHITE)
                                    .size(14.0),
                            )
                            .fill(ACCENT_BLUE)
                            .rounding(6.0),
                        )
                        .clicked()
                    {
                        save_callback();
                    }
                    ui.add_space(4.0);
                });
            });
        });
}

// ============================================================================
// SIDEBAR - Navegação vertical com ícones
// ============================================================================
fn render_sidebar(ui: &mut egui::Ui, selected_tab: &mut usize) {
    Frame::none()
        .fill(BG_DARK)
        .inner_margin(Margin::symmetric(8.0, 16.0))
        .show(ui, |ui| {
            ui.set_width(180.0);
            ui.set_height(ui.available_height());

            let tabs = [
                ("📺", "Overlay"),
                ("🔤", "Fonte"),
                ("🖼️", "Display"),
                ("💬", "Legendas"),
                ("⌨️", "Atalhos"),
                ("🌐", "Serviços"),
                ("📜", "Histórico"),
                ("🧠", "OpenAI"),
            ];

            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;

                for (i, (icon, label)) in tabs.iter().enumerate() {
                    let is_selected = *selected_tab == i;
                    let bg_color = if is_selected { ACCENT_BLUE } else { CARD_BG };
                    let text_color = if is_selected {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(190, 190, 190)
                    };

                    let response = Frame::none()
                        .fill(bg_color)
                        .rounding(6.0)
                        .inner_margin(Margin::symmetric(14.0, 12.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(4.0);
                                ui.label(RichText::new(*icon).color(text_color).size(16.0));
                                ui.add_space(10.0);
                                ui.label(RichText::new(*label).color(text_color).size(14.0));
                            });
                        })
                        .response;

                    if response.clicked() {
                        *selected_tab = i;
                    }
                }
            });
        });
}

// ============================================================================
// ÁREA DE CONTEÚDO - Renderiza a aba selecionada com cards estilizados
// ============================================================================
fn render_content_area(
    ui: &mut egui::Ui,
    tab: usize,
    cfg: &mut config::AppConfig,
    subtitle_state: &subtitle::SubtitleState,
    openai_request_count: &Arc<Mutex<u32>>,
) {
    match tab {
        0 => render_overlay_tab(ui, cfg),
        1 => render_font_tab(ui, cfg),
        2 => render_display_tab(ui, cfg),
        3 => render_subtitle_tab(ui, cfg),
        4 => render_hotkeys_tab(ui, cfg),
        5 => render_services_tab(ui, cfg),
        6 => render_history_tab(ui, subtitle_state),
        7 => render_openai_tab(ui, cfg, openai_request_count),
        _ => {}
    }
}

// ============================================================================
// COMPONENTE: Card estilizado para seções de configuração
// ============================================================================
fn settings_card(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    Frame::none()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .rounding(8.0)
        .inner_margin(Margin::symmetric(18.0, 16.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(title)
                        .color(Color32::from_rgb(220, 220, 220))
                        .size(16.0)
                        .strong(),
                );
                ui.add_space(14.0);
                body(ui);
            });
        });
}

// ============================================================================
// ABA 0 - OVERLAY
// ============================================================================
fn render_overlay_tab(ui: &mut egui::Ui, cfg: &mut config::AppConfig) {
    settings_card(ui, "Configurações de Overlay", |ui| {
        ui.checkbox(&mut cfg.overlay.show_background, "Mostrar fundo do overlay");
        ui.add_space(6.0);
        ui.label(
            RichText::new("Se desativado, mostra apenas texto com contorno")
                .color(Color32::from_rgb(150, 150, 150))
                .size(13.0),
        );
    });
}

// ============================================================================
// ABA 1 - FONTE
// ============================================================================
fn render_font_tab(ui: &mut egui::Ui, cfg: &mut config::AppConfig) {
    settings_card(ui, "Fonte (Modo Região/Tela Cheia)", |ui| {
        ui.horizontal(|ui| {
            ui.set_width(180.0);
            ui.label(RichText::new("Tamanho da fonte").color(Color32::from_rgb(190, 190, 190)));
            ui.add_space(8.0);
            ui.add(egui::Slider::new(&mut cfg.font.size, 12.0..=72.0).suffix("px"));
        });

        ui.add_space(14.0);
        ui.checkbox(&mut cfg.font.outline.enabled, "Contorno ativado");

        if cfg.font.outline.enabled {
            ui.add_space(8.0);
            ui.indent("font_outline", |ui| {
                ui.horizontal(|ui| {
                    ui.set_width(150.0);
                    ui.label("Espessura do contorno");
                    ui.add_space(8.0);
                    let mut width = cfg.font.outline.width as i32;
                    if ui
                        .add(egui::Slider::new(&mut width, 1..=10).suffix("px"))
                        .changed()
                    {
                        cfg.font.outline.width = width as u32;
                    }
                });
            });
        }
    });
}

// ============================================================================
// ABA 2 - DISPLAY (Pré-processamento OCR)
// ============================================================================
fn render_display_tab(ui: &mut egui::Ui, cfg: &mut config::AppConfig) {
    settings_card(ui, "Pré-processamento OCR", |ui| {
        ui.checkbox(
            &mut cfg.display.preprocess.enabled,
            "Ativar pré-processamento de imagem",
        );

        if cfg.display.preprocess.enabled {
            ui.add_space(12.0);
            render_preprocess_controls(ui, &mut cfg.display.preprocess, "display_preprocess");
        }
    });
}

// ============================================================================
// ABA 3 - LEGENDAS
// ============================================================================
fn render_subtitle_tab(ui: &mut egui::Ui, cfg: &mut config::AppConfig) {
    settings_card(ui, "Comportamento das Legendas", |ui| {
        ui.horizontal(|ui| {
            ui.set_width(180.0);
            ui.label(RichText::new("Intervalo de captura").color(Color32::from_rgb(190, 190, 190)));
            ui.add_space(8.0);
            let mut interval = cfg.subtitle.capture_interval_ms as i32;
            if ui
                .add(egui::Slider::new(&mut interval, 50..=2000).suffix("ms"))
                .changed()
            {
                cfg.subtitle.capture_interval_ms = interval as u64;
            }
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.set_width(180.0);
            ui.label(RichText::new("Máximo de linhas").color(Color32::from_rgb(190, 190, 190)));
            ui.add_space(8.0);
            let mut lines = cfg.subtitle.max_lines as i32;
            if ui.add(egui::Slider::new(&mut lines, 1..=10)).changed() {
                cfg.subtitle.max_lines = lines as usize;
            }
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.set_width(180.0);
            ui.label(RichText::new("Timeout sem texto").color(Color32::from_rgb(190, 190, 190)));
            ui.add_space(8.0);
            let mut timeout = cfg.subtitle.max_display_secs as i32;
            if ui
                .add(egui::Slider::new(&mut timeout, 1..=30).suffix("s"))
                .changed()
            {
                cfg.subtitle.max_display_secs = timeout as u64;
            }
        });
    });

    ui.add_space(20.0);

    settings_card(ui, "Fonte das Legendas", |ui| {
        ui.horizontal(|ui| {
            ui.set_width(150.0);
            ui.label(RichText::new("Tamanho").color(Color32::from_rgb(190, 190, 190)));
            ui.add_space(8.0);
            ui.add(egui::Slider::new(&mut cfg.subtitle.font.size, 12.0..=72.0).suffix("px"));
        });

        ui.add_space(12.0);
        ui.checkbox(&mut cfg.subtitle.font.outline.enabled, "Contorno ativado");

        if cfg.subtitle.font.outline.enabled {
            ui.add_space(8.0);
            ui.indent("sub_outline", |ui| {
                ui.horizontal(|ui| {
                    ui.set_width(150.0);
                    ui.label("Espessura");
                    ui.add_space(8.0);
                    let mut width = cfg.subtitle.font.outline.width as i32;
                    if ui
                        .add(egui::Slider::new(&mut width, 1..=10).suffix("px"))
                        .changed()
                    {
                        cfg.subtitle.font.outline.width = width as u32;
                    }
                });
            });
        }
    });

    ui.add_space(20.0);

    settings_card(ui, "Pré-processamento OCR (Legendas)", |ui| {
        ui.checkbox(
            &mut cfg.subtitle.preprocess.enabled,
            "Ativar pré-processamento",
        );

        if cfg.subtitle.preprocess.enabled {
            ui.add_space(12.0);
            render_preprocess_controls(ui, &mut cfg.subtitle.preprocess, "sub_preprocess");
        }
    });
}

// ============================================================================
// ABA 4 - ATALHOS
// ============================================================================
fn render_hotkeys_tab(ui: &mut egui::Ui, cfg: &mut config::AppConfig) {
    settings_card(ui, "Teclas de Atalho", |ui| {
        ui.label(
            RichText::new("Selecione a tecla para cada ação")
                .color(Color32::from_rgb(190, 190, 190))
                .size(14.0),
        );
        ui.add_space(12.0);

        let teclas_disponiveis = vec![
            "Numpad0",
            "Numpad1",
            "Numpad2",
            "Numpad3",
            "Numpad4",
            "Numpad5",
            "Numpad6",
            "Numpad7",
            "Numpad8",
            "Numpad9",
            "NumpadAdd",
            "NumpadSubtract",
            "NumpadMultiply",
            "NumpadDivide",
            "NumpadDecimal",
            "F1",
            "F2",
            "F3",
            "F4",
            "F5",
            "F6",
            "F7",
            "F8",
            "F9",
            "F10",
            "F11",
            "F12",
        ];

        // Tela Cheia
        ui.group(|ui| {
            ui.label(RichText::new("Tela Cheia").color(ACCENT_BLUE).strong());
            render_hotkey_combo(
                ui,
                "hotkey_fullscreen",
                "Capturar e traduzir:",
                &mut cfg.hotkeys.translate_fullscreen,
                &teclas_disponiveis,
            );
        });

        ui.add_space(10.0);

        // Captura em Área
        ui.group(|ui| {
            ui.label(RichText::new("Captura em Área").color(ACCENT_BLUE).strong());
            render_hotkey_combo(
                ui,
                "hotkey_select_region",
                "Selecionar área:",
                &mut cfg.hotkeys.select_region,
                &teclas_disponiveis,
            );
            render_hotkey_combo(
                ui,
                "hotkey_translate_region",
                "Traduzir área:",
                &mut cfg.hotkeys.translate_region,
                &teclas_disponiveis,
            );
        });

        ui.add_space(10.0);

        // Modo Legenda
        ui.group(|ui| {
            ui.label(RichText::new("Modo Legenda").color(ACCENT_BLUE).strong());
            render_hotkey_combo(
                ui,
                "hotkey_select_subtitle",
                "Selecionar área:",
                &mut cfg.hotkeys.select_subtitle_region,
                &teclas_disponiveis,
            );
            render_hotkey_combo(
                ui,
                "hotkey_toggle_subtitle",
                "Ligar/Desligar:",
                &mut cfg.hotkeys.toggle_subtitle_mode,
                &teclas_disponiveis,
            );
        });

        ui.add_space(10.0);

        // Outros
        ui.group(|ui| {
            ui.label(RichText::new("Outros").color(ACCENT_BLUE).strong());
            render_hotkey_combo(
                ui,
                "hotkey_hide",
                "Esconder tradução:",
                &mut cfg.hotkeys.hide_translation,
                &teclas_disponiveis,
            );
        });

        ui.add_space(12.0);
        ui.label(
            RichText::new("⚠️ Reinicie o programa após alterar os atalhos")
                .color(Color32::from_rgb(220, 100, 100))
                .size(13.0),
        );
    });
}

// ============================================================================
// ABA 5 - SERVIÇOS
// ============================================================================
fn render_services_tab(ui: &mut egui::Ui, cfg: &mut config::AppConfig) {
    settings_card(ui, "Provedor de Tradução", |ui| {
        let providers = vec!["google", "deepl", "libretranslate", "openai"];
        ui.horizontal(|ui| {
            ui.set_width(120.0);
            ui.label(RichText::new("Provedor ativo").color(Color32::from_rgb(190, 190, 190)));
            egui::ComboBox::from_id_source("translation_provider")
                .selected_text(&cfg.translation.provider)
                .show_ui(ui, |ui| {
                    for p in &providers {
                        ui.selectable_value(&mut cfg.translation.provider, p.to_string(), *p);
                    }
                });
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.set_width(120.0);
            ui.label("Idioma origem");
            ui.text_edit_singleline(&mut cfg.translation.source_language);
        });

        ui.horizontal(|ui| {
            ui.set_width(120.0);
            ui.label("Idioma destino");
            ui.text_edit_singleline(&mut cfg.translation.target_language);
        });
    });

    ui.add_space(20.0);

    settings_card(ui, "DeepL", |ui| {
        ui.horizontal(|ui| {
            ui.set_width(100.0);
            ui.label("API Key");
            ui.add(
                egui::TextEdit::singleline(&mut cfg.translation.deepl_api_key)
                    .password(true)
                    .desired_width(320.0),
            );
        });

        if cfg.translation.deepl_api_key.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new("⚠️ Necessário para usar DeepL")
                    .color(Color32::from_rgb(220, 100, 100))
                    .size(13.0),
            );
        } else {
            ui.add_space(6.0);
            ui.label(
                RichText::new("✓ Configurado corretamente")
                    .color(Color32::from_rgb(80, 200, 120))
                    .size(13.0),
            );
        }
    });

    ui.add_space(20.0);

    settings_card(ui, "LibreTranslate", |ui| {
        ui.horizontal(|ui| {
            ui.set_width(80.0);
            ui.label("URL");
            ui.text_edit_singleline(&mut cfg.translation.libretranslate_url);
        });
        ui.add_space(6.0);
        ui.label(
            RichText::new("Gratuito e offline (requer servidor local)")
                .color(Color32::from_rgb(150, 150, 150))
                .size(13.0),
        );
    });

    ui.add_space(20.0);

    settings_card(ui, "Google Translate", |ui| {
        ui.label(
            RichText::new("Sem API key necessária (usa API não oficial)")
                .color(Color32::from_rgb(150, 150, 150))
                .size(13.0),
        );
    });

    ui.add_space(20.0);

    settings_card(ui, "ElevenLabs (Text-to-Speech)", |ui| {
        ui.checkbox(&mut cfg.display.tts_enabled, "Ativar TTS");

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.set_width(100.0);
            ui.label("API Key");
            ui.add(
                egui::TextEdit::singleline(&mut cfg.translation.elevenlabs_api_key)
                    .password(true)
                    .desired_width(320.0),
            );
        });

        ui.horizontal(|ui| {
            ui.set_width(100.0);
            ui.label("Voice ID");
            ui.text_edit_singleline(&mut cfg.translation.elevenlabs_voice_id);
        });

        if cfg.translation.elevenlabs_api_key.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new("⚠️ Configure API Key e Voice ID para usar TTS")
                    .color(Color32::from_rgb(220, 100, 100))
                    .size(13.0),
            );
        } else {
            ui.add_space(6.0);
            ui.label(
                RichText::new("✓ Configurado corretamente")
                    .color(Color32::from_rgb(80, 200, 120))
                    .size(13.0),
            );
        }
    });
}

// ============================================================================
// ABA 6 - HISTÓRICO
// ============================================================================
fn render_history_tab(ui: &mut egui::Ui, subtitle_state: &subtitle::SubtitleState) {
    settings_card(ui, "Histórico de Legendas", |ui| {
        if ui.button("Limpar histórico").clicked() {
            subtitle_state.reset();
        }

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        let history = subtitle_state.get_subtitle_history();

        if history.is_empty() {
            ui.label(
                RichText::new("Nenhuma legenda traduzida ainda")
                    .color(Color32::from_rgb(170, 170, 170))
                    .italics(),
            );
            ui.add_space(4.0);
            ui.label("Ative o modo legenda (Numpad 0) para começar.");
        } else {
            ui.label(format!("{} legendas no histórico:", history.len()));
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for (i, entry) in history.iter().rev().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{:2}. ", i + 1))
                                    .color(Color32::from_rgb(150, 150, 150))
                                    .monospace(),
                            );
                            ui.label(&entry.translated);
                        });

                        if i < history.len() - 1 {
                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(6.0);
                        }
                    }
                });
        }
    });
}

// ============================================================================
// ABA 7 - OPENAI
// ============================================================================
fn render_openai_tab(
    ui: &mut egui::Ui,
    cfg: &mut config::AppConfig,
    openai_request_count: &Arc<Mutex<u32>>,
) {
    settings_card(ui, "Autenticação OpenAI", |ui| {
        ui.horizontal(|ui| {
            ui.set_width(100.0);
            ui.label("API Key");
            ui.add(
                egui::TextEdit::singleline(&mut cfg.openai.api_key)
                    .password(true)
                    .desired_width(350.0),
            );
        });

        ui.add_space(8.0);
        if cfg.openai.api_key.is_empty() {
            ui.label(
                RichText::new("⚠️ Necessário para usar OpenAI como provedor")
                    .color(Color32::from_rgb(220, 100, 100))
                    .size(13.0),
            );
        } else {
            ui.label(
                RichText::new("✓ API Key configurada")
                    .color(Color32::from_rgb(80, 200, 120))
                    .size(13.0),
            );
        }
    });

    ui.add_space(20.0);

    settings_card(ui, "Modelo e Parâmetros", |ui| {
        let models = vec!["gpt-4o-mini", "gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"];
        ui.horizontal(|ui| {
            ui.set_width(100.0);
            ui.label("Modelo");
            egui::ComboBox::from_id_source("openai_model")
                .selected_text(&cfg.openai.model)
                .show_ui(ui, |ui| {
                    for m in &models {
                        ui.selectable_value(&mut cfg.openai.model, m.to_string(), *m);
                    }
                });
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.set_width(100.0);
            ui.label("Temperature");
            ui.add(egui::Slider::new(&mut cfg.openai.temperature, 0.0..=2.0).step_by(0.1));
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("0.0 = literal | 0.3 = recomendado | 1.0+ = criativo")
                .color(Color32::from_rgb(150, 150, 150))
                .size(12.0),
        );

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.set_width(100.0);
            ui.label("Max tokens");
            let mut tokens = cfg.openai.max_tokens as i32;
            if ui.add(egui::Slider::new(&mut tokens, 128..=4096)).changed() {
                cfg.openai.max_tokens = tokens as u32;
            }
        });
    });

    ui.add_space(20.0);

    settings_card(ui, "Controle de Custo", |ui| {
        let mut limit = cfg.openai.max_requests_per_session as i32;
        ui.horizontal(|ui| {
            ui.set_width(180.0);
            ui.label("Limite de requests/sessão");
            if ui
                .add(egui::Slider::new(&mut limit, 0..=2000).suffix(" req"))
                .changed()
            {
                cfg.openai.max_requests_per_session = limit as u32;
            }
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("0 = ilimitado")
                .color(Color32::from_rgb(150, 150, 150))
                .size(12.0),
        );

        ui.add_space(12.0);
        let count = *openai_request_count.lock().unwrap();
        let limit_val = cfg.openai.max_requests_per_session;
        let status_color = if limit_val > 0 && count >= limit_val {
            Color32::from_rgb(220, 100, 100)
        } else {
            Color32::from_rgb(150, 150, 150)
        };

        let status_text = if limit_val == 0 {
            format!("Requests nesta sessão: {}", count)
        } else {
            format!("Requests: {} / {} (limite)", count, limit_val)
        };

        ui.label(RichText::new(status_text).color(status_color).monospace());

        ui.add_space(8.0);
        if count > 0 {
            if ui.button("Resetar contador").clicked() {
                *openai_request_count.lock().unwrap() = 0;
            }
        }

        ui.add_space(12.0);
        let fallbacks = vec!["google", "deepl", "libretranslate"];
        ui.horizontal(|ui| {
            ui.set_width(180.0);
            ui.label("Fallback ao atingir limite");
            egui::ComboBox::from_id_source("openai_fallback")
                .selected_text(&cfg.openai.fallback_provider)
                .show_ui(ui, |ui| {
                    for f in &fallbacks {
                        ui.selectable_value(&mut cfg.openai.fallback_provider, f.to_string(), *f);
                    }
                });
        });
    });

    ui.add_space(20.0);

    settings_card(ui, "Contexto de Conversa", |ui| {
        let mut lines = cfg.openai.context_lines as i32;
        ui.horizontal(|ui| {
            ui.set_width(180.0);
            ui.label("Falas anteriores no prompt");
            if ui
                .add(egui::Slider::new(&mut lines, 0..=20).suffix(" falas"))
                .changed()
            {
                cfg.openai.context_lines = lines as u32;
            }
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("0 = desativado | Recomendado: 3-5 falas para coerência")
                .color(Color32::from_rgb(150, 150, 150))
                .size(12.0),
        );

        ui.add_space(12.0);
        ui.label("Informações do jogo (ajuda a IA com nomes e termos):");
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut cfg.openai.game_context)
                .desired_width(f32::INFINITY)
                .hint_text("Ex: Judgment - jogo de detetive yakuza em Kamurocho, Japão"),
        );
    });

    ui.add_space(20.0);

    settings_card(ui, "System Prompt", |ui| {
        ui.label(
            RichText::new("Define o tom, estilo e regras da tradução")
                .color(Color32::from_rgb(190, 190, 190))
                .size(13.0),
        );
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut cfg.openai.system_prompt)
                        .desired_width(f32::INFINITY)
                        .desired_rows(10)
                        .font(egui::TextStyle::Monospace),
                );
            });

        ui.add_space(10.0);
        if ui.button("Restaurar prompt padrão").clicked() {
            cfg.openai.system_prompt = config::default_openai_system_prompt();
        }
    });

    ui.add_space(20.0);

    settings_card(ui, "Como Usar", |ui| {
        ui.label("1. Cole sua API key da OpenAI acima");
        ui.label("2. Na aba Serviços, selecione 'openai' como provedor");
        ui.label("3. Ajuste o prompt conforme o jogo que está traduzindo");
        ui.label("4. Clique em 'Salvar Configurações' no topo da janela");
    });
}

// ============================================================================
// FUNÇÕES AUXILIARES
// ============================================================================

fn render_preprocess_controls(
    ui: &mut egui::Ui,
    preprocess: &mut config::PreprocessConfig,
    id_prefix: &str,
) {
    ui.indent(id_prefix, |ui| {
        ui.checkbox(&mut preprocess.grayscale, "Converter para escala de cinza");
        ui.checkbox(&mut preprocess.invert, "Inverter cores");

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.set_width(120.0);
            ui.label("Contraste");
            ui.add_space(8.0);
            ui.add(egui::Slider::new(&mut preprocess.contrast, 0.5..=10.0).suffix("x"));
        });

        ui.horizontal(|ui| {
            ui.set_width(120.0);
            ui.label("Threshold");
            ui.add_space(8.0);
            let mut threshold = preprocess.threshold as i32;
            if ui.add(egui::Slider::new(&mut threshold, 0..=255)).changed() {
                preprocess.threshold = threshold as u8;
            }
        });

        ui.horizontal(|ui| {
            ui.set_width(120.0);
            ui.label("Blur");
            ui.add_space(8.0);
            ui.add(egui::Slider::new(&mut preprocess.blur, 0.0..=5.0).suffix("px"));
        });

        ui.horizontal(|ui| {
            ui.set_width(120.0);
            ui.label("Dilatação");
            ui.add_space(8.0);
            let mut d = preprocess.dilate as i32;
            if ui
                .add(egui::Slider::new(&mut d, 0..=5).suffix("px"))
                .changed()
            {
                preprocess.dilate = d as u8;
            }
        });

        ui.horizontal(|ui| {
            ui.set_width(120.0);
            ui.label("Erosão");
            ui.add_space(8.0);
            let mut e = preprocess.erode as i32;
            if ui
                .add(egui::Slider::new(&mut e, 0..=5).suffix("px"))
                .changed()
            {
                preprocess.erode = e as u8;
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.set_width(120.0);
            ui.label("Edge Detection");
            ui.add_space(8.0);
            let mut ed = preprocess.edge_detection as i32;
            if ui.add(egui::Slider::new(&mut ed, 0..=150)).changed() {
                preprocess.edge_detection = ed as u8;
            }
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("0 = desativado | 30-80 = recomendado (substitui threshold)")
                .color(Color32::from_rgb(150, 150, 150))
                .size(12.0),
        );

        ui.add_space(10.0);
        ui.checkbox(
            &mut preprocess.save_debug_image,
            "Salvar imagem de debug para análise",
        );
    });
}

fn render_hotkey_combo(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    value: &mut String,
    options: &[&str],
) {
    ui.horizontal(|ui| {
        ui.set_width(140.0);
        ui.label(label);
        egui::ComboBox::from_id_source(id)
            .selected_text(value.as_str())
            .width(140.0)
            .show_ui(ui, |ui| {
                for opt in options {
                    ui.selectable_value(value, opt.to_string(), *opt);
                }
            });
    });
}
