// game-translator/src/settings_ui.rs

// ============================================================================
// MÓDULO SETTINGS UI - Interface de configurações
// ============================================================================
// Este módulo contém toda a lógica de renderização da tela de configurações.
// Todas as abas seguem o mesmo padrão visual:
// - Grupos (ui.group) para agrupar seções
// - Grid para alinhar labels e sliders
// - Espaçamento consistente entre seções
// ============================================================================

use crate::config;
use crate::subtitle;
use std::sync::{Arc, Mutex};

// ============================================================================
// FUNÇÃO PÚBLICA - Renderiza o conteúdo da aba ativa
// ============================================================================

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

    // --- Fundo ---
    full_width_group(ui, |ui| {
        ui.label("Aparencia:");
        ui.add_space(5.0);
        ui.checkbox(&mut cfg.overlay.show_background, "Mostrar fundo do overlay");
        ui.label("Se desativado, mostra apenas texto com contorno");
    });

    ui.add_space(10.0);

    // --- Duração ---
    full_width_group(ui, |ui| {
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

// ============================================================================
// ABA 1 - FONTE
// ============================================================================
fn render_font_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Fonte (Modo Regiao/Tela Cheia)");
    ui.add_space(10.0);

    // --- Tamanho e tipo ---
    full_width_group(ui, |ui| {
        ui.label("Configuracao da fonte:");
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
    full_width_group(ui, |ui| {
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

// ============================================================================
// ABA 2 - DISPLAY (Pré-processamento OCR)
// ============================================================================
fn render_display_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Pre-processamento OCR");
    ui.add_space(10.0);

    // --- Pré-processamento ---
    full_width_group(ui, |ui| {
        ui.label("Pre-processamento (Regiao/Tela Cheia):");
        ui.add_space(5.0);

        ui.checkbox(&mut cfg.display.preprocess.enabled, "Ativado");

        if cfg.display.preprocess.enabled {
            ui.add_space(5.0);
            render_preprocess_controls(ui, &mut cfg.display.preprocess, "preprocess");
        }
    });
}

// ============================================================================
// ABA 3 - LEGENDAS
// ============================================================================
fn render_subtitle_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Legendas");
    ui.add_space(10.0);

    // --- Configurações gerais ---
    full_width_group(ui, |ui| {
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
    full_width_group(ui, |ui| {
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
    full_width_group(ui, |ui| {
        ui.label("Pre-processamento OCR (Legendas):");
        ui.add_space(5.0);

        ui.checkbox(&mut cfg.subtitle.preprocess.enabled, "Ativado");

        if cfg.subtitle.preprocess.enabled {
            ui.add_space(5.0);
            render_preprocess_controls(ui, &mut cfg.subtitle.preprocess, "sub_preprocess");
        }
    });
}

// ============================================================================
// ABA 4 - ATALHOS
// ============================================================================
fn render_hotkeys_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Teclas de Atalho");
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

    // --- Tela Cheia ---
    full_width_group(ui, |ui| {
        ui.label("Tela Cheia:");
        ui.add_space(5.0);
        render_hotkey_combo(
            ui,
            "hotkey_fullscreen",
            "Capturar e traduzir:",
            &mut cfg.hotkeys.translate_fullscreen,
            &teclas_disponiveis,
        );
    });

    ui.add_space(10.0);

    // --- Captura em Área ---
    full_width_group(ui, |ui| {
        ui.label("Captura em Area:");
        ui.add_space(5.0);
        render_hotkey_combo(
            ui,
            "hotkey_select_region",
            "Selecionar area:",
            &mut cfg.hotkeys.select_region,
            &teclas_disponiveis,
        );
        render_hotkey_combo(
            ui,
            "hotkey_translate_region",
            "Traduzir area:",
            &mut cfg.hotkeys.translate_region,
            &teclas_disponiveis,
        );
    });

    ui.add_space(10.0);

    // --- Modo Legenda ---
    full_width_group(ui, |ui| {
        ui.label("Modo Legenda:");
        ui.add_space(5.0);
        render_hotkey_combo(
            ui,
            "hotkey_select_subtitle",
            "Selecionar area:",
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

    // --- Outros ---
    full_width_group(ui, |ui| {
        ui.label("Outros:");
        ui.add_space(5.0);
        render_hotkey_combo(
            ui,
            "hotkey_hide",
            "Esconder traducao:",
            &mut cfg.hotkeys.hide_translation,
            &teclas_disponiveis,
        );
    });

    ui.add_space(10.0);

    // --- Aviso ---
    full_width_group(ui, |ui| {
        ui.label("Reinicie o programa apos alterar os atalhos.");
    });
}

// ============================================================================
// ABA 5 - SERVIÇOS
// ============================================================================
fn render_services_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Servicos de Traducao e Voz");
    ui.add_space(10.0);

    // --- Provedor de tradução ---
    full_width_group(ui, |ui| {
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
    full_width_group(ui, |ui| {
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
    full_width_group(ui, |ui| {
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
    full_width_group(ui, |ui| {
        ui.label("Google Translate:");
        ui.add_space(5.0);
        ui.label("Sem API key necessaria (usa API nao oficial)");
    });

    ui.add_space(10.0);

    // --- ElevenLabs TTS ---
    full_width_group(ui, |ui| {
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

// ============================================================================
// ABA 6 - HISTÓRICO
// ============================================================================
fn render_history_tab(ui: &mut eframe::egui::Ui, subtitle_state: &subtitle::SubtitleState) {
    ui.heading("Historico de Legendas");
    ui.add_space(10.0);

    // --- Controles ---
    full_width_group(ui, |ui| {
        let history = subtitle_state.get_subtitle_history();
        ui.label(format!("{} legendas no historico", history.len()));

        ui.add_space(5.0);
        if ui.button("Limpar historico").clicked() {
            subtitle_state.reset();
        }
    });

    ui.add_space(10.0);

    // --- Lista de legendas ---
    full_width_group(ui, |ui| {
        ui.label("Legendas traduzidas:");
        ui.add_space(5.0);

        let history = subtitle_state.get_subtitle_history();

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
    full_width_group(ui, |ui| {
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

    // --- Modelo e parâmetros ---
    full_width_group(ui, |ui| {
        ui.label("Modelo e Parametros:");
        ui.add_space(5.0);

        let models = vec!["gpt-4o-mini", "gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"];

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
    full_width_group(ui, |ui| {
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
    full_width_group(ui, |ui| {
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
    full_width_group(ui, |ui| {
        ui.label("System Prompt (instrucao para a IA):");
        ui.add_space(5.0);
        ui.label("Define o tom, estilo e regras da traducao:");
        ui.add_space(5.0);

        // ScrollArea dentro do grupo pro prompt não ficar gigante
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
    full_width_group(ui, |ui| {
        ui.label("Como usar:");
        ui.add_space(3.0);
        ui.label("1. Cole sua API key da OpenAI acima");
        ui.label("2. Na aba Servicos, selecione 'openai' como provedor");
        ui.label("3. Ajuste o prompt conforme o jogo que esta traduzindo");
        ui.label("4. Salve as configuracoes");
    });
}

// ============================================================================
// FUNÇÕES AUXILIARES (reutilizadas em múltiplas abas)
// ============================================================================

/// Renderiza um grupo (caixa com borda) que sempre ocupa a largura total disponível.
/// Substitui ui.group() pra manter visual consistente em todas as abas.
fn full_width_group(ui: &mut eframe::egui::Ui, add_contents: impl FnOnce(&mut eframe::egui::Ui)) {
    let available_width = ui.available_width();

    // Frame com visual idêntico ao ui.group() mas com largura forçada
    // .max(0.0) evita panic quando a janela ainda não redimensionou
    eframe::egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width((available_width - 12.0).max(0.0));
        add_contents(ui);
    });
}

/// Renderiza controles de pré-processamento OCR usando Grid para alinhamento.
/// Reutilizado nas abas Display e Legendas.
fn render_preprocess_controls(
    ui: &mut eframe::egui::Ui,
    preprocess: &mut config::PreprocessConfig,
    id_prefix: &str,
) {
    // Checkboxes ficam fora do grid (não precisam de alinhamento com sliders)
    ui.checkbox(&mut preprocess.grayscale, "Escala de cinza");
    ui.checkbox(&mut preprocess.invert, "Inverter cores");

    ui.add_space(5.0);

    // Grid alinha todos os labels na mesma coluna e sliders na segunda coluna
    eframe::egui::Grid::new(format!("{}_grid", id_prefix))
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            ui.label("Contraste:");
            ui.add(eframe::egui::Slider::new(&mut preprocess.contrast, 0.5..=10.0).suffix("x"));
            ui.end_row();

            ui.label("Threshold:");
            let mut threshold = preprocess.threshold as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut threshold, 0..=255))
                .changed()
            {
                preprocess.threshold = threshold as u8;
            }
            ui.end_row();

            ui.label("Upscale:");
            ui.add(eframe::egui::Slider::new(&mut preprocess.upscale, 1.0..=4.0).suffix("x"));
            ui.end_row();

            ui.label("Blur:");
            ui.add(eframe::egui::Slider::new(&mut preprocess.blur, 0.0..=5.0).suffix("x"));
            ui.end_row();

            ui.label("Dilatacao:");
            let mut d = preprocess.dilate as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut d, 0..=5).suffix("px"))
                .changed()
            {
                preprocess.dilate = d as u8;
            }
            ui.end_row();

            ui.label("Erosao:");
            let mut e_val = preprocess.erode as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut e_val, 0..=5).suffix("px"))
                .changed()
            {
                preprocess.erode = e_val as u8;
            }
            ui.end_row();

            ui.label("Edge Detection:");
            let mut ed = preprocess.edge_detection as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut ed, 0..=150))
                .changed()
            {
                preprocess.edge_detection = ed as u8;
            }
            ui.end_row();
        });

    ui.add_space(3.0);
    ui.label("Edge Detection: 0=desativado, 30-80=recomendado (substitui threshold)");

    ui.add_space(5.0);
    ui.checkbox(&mut preprocess.save_debug_image, "Salvar imagem debug");
}

/// Renderiza um combo box de hotkey com label, usando layout horizontal simples.
/// O alinhamento entre combos é garantido pelo min_label_width fixo.
fn render_hotkey_combo(
    ui: &mut eframe::egui::Ui,
    id: &str,
    label: &str,
    value: &mut String,
    options: &[&str],
) {
    ui.horizontal(|ui| {
        // Largura fixa pro label garante que todos os combos alinhem
        let label_response = ui.label(label);
        let used = label_response.rect.width();
        let min_label = 150.0; // Largura mínima pra todos os labels de hotkey
        if used < min_label {
            ui.add_space(min_label - used);
        }

        eframe::egui::ComboBox::from_id_source(format!("{}_combo", id))
            .selected_text(value.as_str())
            .show_ui(ui, |ui: &mut eframe::egui::Ui| {
                for opt in options {
                    ui.selectable_value(value, opt.to_string(), *opt);
                }
            });
    });
}
