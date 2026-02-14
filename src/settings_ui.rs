// game-translator/src/settings_ui.rs

// ============================================================================
// MÓDULO SETTINGS UI - Interface de configurações
// ============================================================================
// Este módulo contém toda a lógica de renderização da tela de configurações.
// Foi extraído do main.rs para manter o código organizado.
// ============================================================================

use crate::config;
use crate::subtitle;
use std::sync::{Arc, Mutex};

/// Renderiza o conteúdo de uma aba específica das configurações.
///
/// # Parâmetros
/// - `ui`: referência ao egui UI para desenhar widgets
/// - `tab`: número da aba ativa (0-7)
/// - `cfg`: configurações sendo editadas (mutável)
/// - `subtitle_state`: estado das legendas (para aba Histórico)
/// - `openai_request_count`: contador de requests OpenAI (para aba OpenAI)
pub fn render_tab(
    ui: &mut eframe::egui::Ui,
    tab: u8,
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
// ABA 0 - OVERLAY
// ============================================================================
fn render_overlay_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Overlay");
    ui.add_space(10.0);
    ui.checkbox(&mut cfg.overlay.show_background, "Mostrar fundo do overlay");
    ui.label("   Se desativado, mostra apenas texto com contorno");
}

// ============================================================================
// ABA 1 - FONTE
// ============================================================================
fn render_font_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Fonte (Modo Regiao/Tela Cheia)");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Tamanho da fonte:");
        ui.add(eframe::egui::Slider::new(&mut cfg.font.size, 12.0..=72.0).suffix("px"));
    });

    ui.add_space(10.0);
    ui.checkbox(&mut cfg.font.outline.enabled, "Contorno ativado");

    if cfg.font.outline.enabled {
        ui.horizontal(|ui| {
            ui.label("   Espessura:");
            let mut width = cfg.font.outline.width as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut width, 1..=10).suffix("px"))
                .changed()
            {
                cfg.font.outline.width = width as u32;
            }
        });
    }
}

// ============================================================================
// ABA 2 - DISPLAY (Pré-processamento OCR)
// ============================================================================
fn render_display_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Display - Pre-processamento OCR");
    ui.add_space(10.0);

    ui.checkbox(
        &mut cfg.display.preprocess.enabled,
        "Pre-processamento ativado",
    );

    if cfg.display.preprocess.enabled {
        ui.add_space(10.0);
        // Renderiza os controles de pré-processamento (reutilizado na aba Legendas)
        render_preprocess_controls(ui, &mut cfg.display.preprocess, "preprocess");
    }
}

// ============================================================================
// ABA 3 - LEGENDAS
// ============================================================================
fn render_subtitle_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Legendas");
    ui.add_space(10.0);

    // --- Configurações gerais ---
    ui.horizontal(|ui| {
        ui.label("Intervalo de captura:");
        let mut interval = cfg.subtitle.capture_interval_ms as i32;
        if ui
            .add(eframe::egui::Slider::new(&mut interval, 50..=2000).suffix("ms"))
            .changed()
        {
            cfg.subtitle.capture_interval_ms = interval as u64;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Maximo de linhas:");
        let mut lines = cfg.subtitle.max_lines as i32;
        if ui
            .add(eframe::egui::Slider::new(&mut lines, 1..=10))
            .changed()
        {
            cfg.subtitle.max_lines = lines as usize;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Timeout (sem texto):");
        let mut timeout = cfg.subtitle.max_display_secs as i32;
        if ui
            .add(eframe::egui::Slider::new(&mut timeout, 1..=30).suffix("s"))
            .changed()
        {
            cfg.subtitle.max_display_secs = timeout as u64;
        }
    });

    // --- Fonte das legendas ---
    ui.add_space(15.0);
    ui.separator();
    ui.label("Fonte das legendas:");
    ui.add_space(5.0);

    ui.horizontal(|ui| {
        ui.label("   Tamanho:");
        ui.add(eframe::egui::Slider::new(&mut cfg.subtitle.font.size, 12.0..=72.0).suffix("px"));
    });

    ui.checkbox(
        &mut cfg.subtitle.font.outline.enabled,
        "   Contorno ativado",
    );

    if cfg.subtitle.font.outline.enabled {
        ui.horizontal(|ui| {
            ui.label("      Espessura:");
            let mut width = cfg.subtitle.font.outline.width as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut width, 1..=10).suffix("px"))
                .changed()
            {
                cfg.subtitle.font.outline.width = width as u32;
            }
        });
    }

    // --- Pré-processamento OCR (Legendas) ---
    ui.add_space(15.0);
    ui.separator();
    ui.label("Pre-processamento OCR (Legendas):");
    ui.add_space(5.0);

    ui.checkbox(&mut cfg.subtitle.preprocess.enabled, "   Ativado");

    if cfg.subtitle.preprocess.enabled {
        render_preprocess_controls(ui, &mut cfg.subtitle.preprocess, "sub_preprocess");
    }
}

// ============================================================================
// ABA 4 - ATALHOS
// ============================================================================
fn render_hotkeys_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Teclas de Atalho");
    ui.add_space(10.0);

    ui.label("Selecione a tecla para cada acao:");
    ui.add_space(10.0);

    // Lista de teclas disponíveis
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
        ui.label("Tela Cheia:");
        render_hotkey_combo(
            ui,
            "hotkey_fullscreen",
            "   Capturar e traduzir:",
            &mut cfg.hotkeys.translate_fullscreen,
            &teclas_disponiveis,
        );
    });

    ui.add_space(10.0);

    // Captura em Área
    ui.group(|ui| {
        ui.label("Captura em Area:");
        render_hotkey_combo(
            ui,
            "hotkey_select_region",
            "   Selecionar area:",
            &mut cfg.hotkeys.select_region,
            &teclas_disponiveis,
        );
        render_hotkey_combo(
            ui,
            "hotkey_translate_region",
            "   Traduzir area:",
            &mut cfg.hotkeys.translate_region,
            &teclas_disponiveis,
        );
    });

    ui.add_space(10.0);

    // Modo Legenda
    ui.group(|ui| {
        ui.label("Modo Legenda:");
        render_hotkey_combo(
            ui,
            "hotkey_select_subtitle",
            "   Selecionar area:",
            &mut cfg.hotkeys.select_subtitle_region,
            &teclas_disponiveis,
        );
        render_hotkey_combo(
            ui,
            "hotkey_toggle_subtitle",
            "   Ligar/Desligar:",
            &mut cfg.hotkeys.toggle_subtitle_mode,
            &teclas_disponiveis,
        );
    });

    ui.add_space(10.0);

    // Outros
    ui.group(|ui| {
        ui.label("Outros:");
        render_hotkey_combo(
            ui,
            "hotkey_hide",
            "   Esconder traducao:",
            &mut cfg.hotkeys.hide_translation,
            &teclas_disponiveis,
        );
    });

    ui.add_space(15.0);
    ui.separator();
    ui.add_space(5.0);
    ui.label("Reinicie o programa apos alterar os atalhos.");
}

// ============================================================================
// ABA 5 - SERVIÇOS
// ============================================================================
fn render_services_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Servicos de Traducao e Voz");
    ui.add_space(10.0);

    // --- Provedor de tradução ---
    ui.group(|ui| {
        ui.label("Provedor de Traducao:");
        ui.add_space(5.0);

        let providers = vec!["google", "deepl", "libretranslate", "openai"];
        ui.horizontal(|ui| {
            ui.label("   Provedor ativo:");
            eframe::egui::ComboBox::from_id_source("translation_provider")
                .selected_text(&cfg.translation.provider)
                .show_ui(ui, |ui| {
                    for p in &providers {
                        ui.selectable_value(&mut cfg.translation.provider, p.to_string(), *p);
                    }
                });
        });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("   Idioma origem:");
            ui.text_edit_singleline(&mut cfg.translation.source_language);
        });

        ui.horizontal(|ui| {
            ui.label("   Idioma destino:");
            ui.text_edit_singleline(&mut cfg.translation.target_language);
        });
    });

    ui.add_space(10.0);

    // --- DeepL ---
    ui.group(|ui| {
        ui.label("DeepL:");
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("   API Key:");
            ui.add(
                eframe::egui::TextEdit::singleline(&mut cfg.translation.deepl_api_key)
                    .password(true)
                    .desired_width(300.0),
            );
        });
        if cfg.translation.deepl_api_key.is_empty() {
            ui.label("   Necessario para usar DeepL");
        } else {
            ui.label("   Configurado");
        }
    });

    ui.add_space(10.0);

    // --- LibreTranslate ---
    ui.group(|ui| {
        ui.label("LibreTranslate:");
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("   URL:");
            ui.text_edit_singleline(&mut cfg.translation.libretranslate_url);
        });
        ui.label("   Gratuito e offline (requer servidor local)");
    });

    ui.add_space(10.0);

    // --- Google ---
    ui.group(|ui| {
        ui.label("Google Translate:");
        ui.label("   Sem API key necessaria (usa API nao oficial)");
    });

    ui.add_space(15.0);
    ui.separator();
    ui.add_space(10.0);

    // --- ElevenLabs TTS ---
    ui.group(|ui| {
        ui.label("ElevenLabs (Text-to-Speech):");
        ui.add_space(5.0);

        ui.checkbox(&mut cfg.display.tts_enabled, "   TTS ativado");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("   API Key:");
            ui.add(
                eframe::egui::TextEdit::singleline(&mut cfg.translation.elevenlabs_api_key)
                    .password(true)
                    .desired_width(300.0),
            );
        });

        ui.horizontal(|ui| {
            ui.label("   Voice ID:");
            ui.text_edit_singleline(&mut cfg.translation.elevenlabs_voice_id);
        });

        if cfg.translation.elevenlabs_api_key.is_empty() {
            ui.label("   Configure API Key e Voice ID para usar TTS");
        } else {
            ui.label("   Configurado");
        }
    });
}

// ============================================================================
// ABA 6 - HISTÓRICO
// ============================================================================
fn render_history_tab(ui: &mut eframe::egui::Ui, subtitle_state: &subtitle::SubtitleState) {
    ui.heading("Historico de Legendas");
    ui.add_space(10.0);

    // Botão de limpar
    if ui.button("Limpar historico").clicked() {
        subtitle_state.reset();
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(5.0);

    // Pega o histórico
    let history = subtitle_state.get_subtitle_history();

    if history.is_empty() {
        ui.label("Nenhuma legenda traduzida ainda.");
        ui.label("Ative o modo legenda (Numpad 0) para comecar.");
    } else {
        ui.label(format!("{} legendas no historico:", history.len()));
        ui.add_space(5.0);

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
}

// ============================================================================
// ABA 7 - OPENAI
// ============================================================================
fn render_openai_tab(
    ui: &mut eframe::egui::Ui,
    cfg: &mut config::AppConfig,
    openai_request_count: &Arc<Mutex<u32>>,
) {
    ui.heading("OpenAI - Traducao com IA");
    ui.add_space(10.0);

    // --- API Key ---
    ui.group(|ui| {
        ui.label("Autenticacao:");
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("   API Key:");
            ui.add(
                eframe::egui::TextEdit::singleline(&mut cfg.openai.api_key)
                    .password(true)
                    .desired_width(350.0),
            );
        });
        if cfg.openai.api_key.is_empty() {
            ui.label("   Necessario para usar OpenAI como provedor");
        } else {
            ui.label("   Configurado");
        }
    });

    ui.add_space(10.0);

    // --- Modelo e parâmetros ---
    ui.group(|ui| {
        ui.label("Modelo e Parametros:");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("   Modelo:");
            let models = vec!["gpt-4o-mini", "gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"];
            eframe::egui::ComboBox::from_id_source("openai_model")
                .selected_text(&cfg.openai.model)
                .show_ui(ui, |ui| {
                    for m in &models {
                        ui.selectable_value(&mut cfg.openai.model, m.to_string(), *m);
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("   Temperature:");
            ui.add(eframe::egui::Slider::new(&mut cfg.openai.temperature, 0.0..=2.0).step_by(0.1));
        });
        ui.label("      0.0 = literal, 0.3 = recomendado, 1.0+ = criativo");

        ui.horizontal(|ui| {
            ui.label("   Max tokens:");
            let mut tokens = cfg.openai.max_tokens as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut tokens, 128..=4096))
                .changed()
            {
                cfg.openai.max_tokens = tokens as u32;
            }
        });
    });

    ui.add_space(10.0);

    // --- Controle de custo ---
    ui.group(|ui| {
        ui.label("Controle de Custo:");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("   Limite de requests/sessao:");
            let mut limit = cfg.openai.max_requests_per_session as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut limit, 0..=2000).suffix(" req"))
                .changed()
            {
                cfg.openai.max_requests_per_session = limit as u32;
            }
        });
        ui.label("      0 = ilimitado");

        ui.add_space(5.0);
        let count = *openai_request_count.lock().unwrap();
        let limit = cfg.openai.max_requests_per_session;
        let status_text = if limit == 0 {
            format!("   Requests nesta sessao: {} (sem limite)", count)
        } else {
            format!("   Requests nesta sessao: {} / {}", count, limit)
        };
        ui.label(status_text);

        if count > 0 {
            if ui.button("Resetar contador").clicked() {
                *openai_request_count.lock().unwrap() = 0;
            }
        }

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("   Fallback quando atingir limite:");
            let fallbacks = vec!["google", "deepl", "libretranslate"];
            eframe::egui::ComboBox::from_id_source("openai_fallback")
                .selected_text(&cfg.openai.fallback_provider)
                .show_ui(ui, |ui| {
                    for f in &fallbacks {
                        ui.selectable_value(&mut cfg.openai.fallback_provider, f.to_string(), *f);
                    }
                });
        });
    });

    ui.add_space(10.0);

    // --- Contexto de conversa ---
    ui.group(|ui| {
        ui.label("Contexto de Conversa:");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("   Falas anteriores no prompt:");
            let mut lines = cfg.openai.context_lines as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut lines, 0..=20).suffix(" falas"))
                .changed()
            {
                cfg.openai.context_lines = lines as u32;
            }
        });
        ui.label("      0 = desativado. Recomendado: 3-5 falas");
        ui.label("      Envia legendas anteriores pra IA manter coerencia no dialogo");

        ui.add_space(5.0);
        ui.label("   Informacoes do jogo (ajuda a IA com nomes e termos):");
        ui.add(
            eframe::egui::TextEdit::singleline(&mut cfg.openai.game_context)
                .desired_width(f32::INFINITY)
                .hint_text("Ex: Judgment - jogo de detetive yakuza em Kamurocho, Japao"),
        );
    });

    ui.add_space(10.0);

    // --- System Prompt ---
    ui.group(|ui| {
        ui.label("System Prompt (instrucao para a IA):");
        ui.add_space(5.0);
        ui.label("   Define o tom, estilo e regras da traducao:");
        ui.add_space(5.0);

        eframe::egui::ScrollArea::vertical()
            .max_height(250.0)
            .show(ui, |ui| {
                ui.add(
                    eframe::egui::TextEdit::multiline(&mut cfg.openai.system_prompt)
                        .desired_width(f32::INFINITY)
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
    ui.group(|ui| {
        ui.label("Como usar:");
        ui.label("   1. Cole sua API key da OpenAI acima");
        ui.label("   2. Na aba Servicos, selecione 'openai' como provedor");
        ui.label("   3. Ajuste o prompt conforme o jogo que esta traduzindo");
        ui.label("   4. Salve as configuracoes");
    });
}

// ============================================================================
// FUNÇÕES AUXILIARES (reutilizadas em múltiplas abas)
// ============================================================================

/// Renderiza controles de pré-processamento OCR.
/// Reutilizado nas abas Display e Legendas.
fn render_preprocess_controls(
    ui: &mut eframe::egui::Ui,
    preprocess: &mut config::PreprocessConfig,
    id_prefix: &str,
) {
    ui.indent(id_prefix, |ui| {
        ui.checkbox(&mut preprocess.grayscale, "Escala de cinza");
        ui.checkbox(&mut preprocess.invert, "Inverter cores");

        ui.horizontal(|ui| {
            ui.label("Contraste:");
            ui.add(eframe::egui::Slider::new(&mut preprocess.contrast, 0.5..=10.0).suffix("x"));
        });

        ui.horizontal(|ui| {
            ui.label("Threshold:");
            let mut threshold = preprocess.threshold as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut threshold, 0..=255))
                .changed()
            {
                preprocess.threshold = threshold as u8;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Blur:");
            ui.add(eframe::egui::Slider::new(&mut preprocess.blur, 0.0..=5.0).suffix("x"));
        });

        ui.horizontal(|ui| {
            ui.label("Dilatacao:");
            let mut d = preprocess.dilate as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut d, 0..=5).suffix("px"))
                .changed()
            {
                preprocess.dilate = d as u8;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Erosao:");
            let mut e = preprocess.erode as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut e, 0..=5).suffix("px"))
                .changed()
            {
                preprocess.erode = e as u8;
            }
        });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Edge Detection:");
            let mut ed = preprocess.edge_detection as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut ed, 0..=150))
                .changed()
            {
                preprocess.edge_detection = ed as u8;
            }
        });
        ui.label("   0=desativado, 30-80=recomendado (substitui threshold)");

        ui.checkbox(&mut preprocess.save_debug_image, "Salvar imagem debug");
    });
}

/// Renderiza um combo box de hotkey com label.
/// Reutilizado na aba Atalhos para evitar repetição.
fn render_hotkey_combo(
    ui: &mut eframe::egui::Ui,
    id: &str,
    label: &str,
    value: &mut String,
    options: &[&str],
) {
    ui.horizontal(|ui| {
        ui.label(label);
        eframe::egui::ComboBox::from_id_source(id)
            .selected_text(value.as_str())
            .show_ui(ui, |ui: &mut eframe::egui::Ui| {
                for opt in options {
                    ui.selectable_value(value, opt.to_string(), *opt);
                }
            });
    });
}
