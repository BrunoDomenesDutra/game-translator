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
use crate::screenshot;
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
#[allow(clippy::too_many_arguments)]
pub fn render_tab(
    ui: &mut eframe::egui::Ui,
    tab: u8,
    cfg: &mut config::AppConfig,
    subtitle_state: &subtitle::SubtitleState,
    openai_request_count: &Arc<Mutex<u32>>,
    debug_texture: &mut Option<eframe::egui::TextureHandle>,
    debug_texture_last_update: &mut std::time::Instant,
    lab_original_texture: &mut Option<eframe::egui::TextureHandle>,
    lab_processed_texture: &mut Option<eframe::egui::TextureHandle>,
    lab_preprocess: &mut Option<config::PreprocessConfig>,
    lab_selected_file: &mut Option<String>,
    lab_original_image: &mut Option<image::DynamicImage>,
    lab_needs_reprocess: &mut bool,
) {
    match tab {
        0 => render_overlay_tab(ui, cfg),
        1 => render_font_tab(ui, cfg),
        2 => render_display_tab(
            ui,
            cfg,
            debug_texture,
            debug_texture_last_update,
            subtitle_state,
        ),
        3 => render_subtitle_tab(
            ui,
            cfg,
            debug_texture,
            debug_texture_last_update,
            subtitle_state,
        ),
        4 => render_hotkeys_tab(ui, cfg),
        5 => render_services_tab(ui, cfg),
        6 => render_history_tab(ui, subtitle_state),
        7 => render_openai_tab(ui, cfg, openai_request_count),
        8 => render_lab_tab(
            ui,
            cfg,
            lab_original_texture,
            lab_processed_texture,
            lab_preprocess,
            lab_selected_file,
            lab_original_image,
            lab_needs_reprocess,
        ),
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
    ui.heading("Fonte das Traducoes");
    ui.add_space(10.0);

    // --- Seleção de fonte ---
    full_width_group(ui, |ui| {
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
    full_width_group(ui, |ui| {
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
fn render_display_tab(
    ui: &mut eframe::egui::Ui,
    cfg: &mut config::AppConfig,
    debug_texture: &mut Option<eframe::egui::TextureHandle>,
    debug_texture_last_update: &mut std::time::Instant,
    subtitle_state: &subtitle::SubtitleState,
) {
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

    ui.add_space(10.0);

    // --- Preview da imagem debug ---
    if cfg.display.preprocess.save_debug_image {
        render_debug_preview(ui, debug_texture, debug_texture_last_update, subtitle_state);
    }
}

// ============================================================================
// ABA 3 - LEGENDAS
// ============================================================================
fn render_subtitle_tab(
    ui: &mut eframe::egui::Ui,
    cfg: &mut config::AppConfig,
    debug_texture: &mut Option<eframe::egui::TextureHandle>,
    debug_texture_last_update: &mut std::time::Instant,
    subtitle_state: &subtitle::SubtitleState,
) {
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

    ui.add_space(10.0);

    // --- Preview da imagem debug ---
    if cfg.subtitle.preprocess.save_debug_image {
        render_debug_preview(ui, debug_texture, debug_texture_last_update, subtitle_state);
    }
}

// ============================================================================
// ABA 4 - ATALHOS
// ============================================================================
fn render_hotkeys_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Teclas de Atalho");
    ui.add_space(10.0);

    // Lista de modificadores disponíveis
    let modificadores = vec!["", "Ctrl", "Shift", "Alt"];

    // Lista de teclas disponíveis
    let teclas_disponiveis = vec![
        // Numpad
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
        // Teclas de função
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
        // Letras
        "A",
        "B",
        "C",
        "D",
        "E",
        "F",
        "G",
        "H",
        "I",
        "J",
        "K",
        "L",
        "M",
        "N",
        "O",
        "P",
        "Q",
        "R",
        "S",
        "T",
        "U",
        "V",
        "W",
        "X",
        "Y",
        "Z",
        // Números
        "0",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        // Especiais
        "Space",
        "Enter",
        "Escape",
        "Tab",
        "Insert",
        "Delete",
        "Home",
        "End",
        "PageUp",
        "PageDown",
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
            &modificadores,
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
            &modificadores,
            &teclas_disponiveis,
        );
        render_hotkey_combo(
            ui,
            "hotkey_translate_region",
            "Traduzir area:",
            &mut cfg.hotkeys.translate_region,
            &modificadores,
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
            &modificadores,
            &teclas_disponiveis,
        );
        render_hotkey_combo(
            ui,
            "hotkey_toggle_subtitle_areas_preview",
            "Mostrar areas:",
            &mut cfg.hotkeys.toggle_subtitle_areas_preview,
            &modificadores,
            &teclas_disponiveis,
        );
        render_hotkey_combo(
            ui,
            "hotkey_toggle_subtitle",
            "Ligar/Desligar:",
            &mut cfg.hotkeys.toggle_subtitle_mode,
            &modificadores,
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
            &modificadores,
            &teclas_disponiveis,
        );
        render_hotkey_combo(
            ui,
            "hotkey_settings",
            "Abrir configuracoes:",
            &mut cfg.hotkeys.open_settings,
            &modificadores,
            &teclas_disponiveis,
        );
    });

    ui.add_space(10.0);
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
        let history = subtitle_state.get_full_history();
        ui.label(format!("{} legendas no historico", history.len()));

        ui.add_space(5.0);
        if ui.button("Limpar historico").clicked() {
            subtitle_state.clear_full_history();
        }
    });

    ui.add_space(10.0);

    // --- Lista de legendas ---
    full_width_group(ui, |ui| {
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
// ABA 8 - LABORATÓRIO DE PRÉ-PROCESSAMENTO
// ============================================================================

/// Aba de teste de pré-processamento.
/// Carrega uma imagem da pasta images/ e aplica os filtros em tempo real.
#[allow(clippy::too_many_arguments)]
fn render_lab_tab(
    ui: &mut eframe::egui::Ui,
    cfg: &mut config::AppConfig,
    original_texture: &mut Option<eframe::egui::TextureHandle>,
    processed_texture: &mut Option<eframe::egui::TextureHandle>,
    preprocess: &mut Option<config::PreprocessConfig>,
    selected_file: &mut Option<String>,
    original_image: &mut Option<image::DynamicImage>,
    needs_reprocess: &mut bool,
) {
    ui.heading("Laboratorio de Pre-processamento");
    ui.add_space(10.0);

    // Inicializa config de pré-processamento se ainda não existe
    if preprocess.is_none() {
        *preprocess = Some(config::PreprocessConfig::default());
    }

    // --- Seleção de imagem ---
    full_width_group(ui, |ui| {
        ui.label("Imagem de teste:");
        ui.add_space(5.0);
        ui.label("Coloque imagens PNG/JPG na pasta 'images/' ao lado do executavel.");
        ui.add_space(5.0);

        // Lista arquivos da pasta images/
        let images_dir = std::path::Path::new("images");
        let mut files: Vec<String> = Vec::new();

        if images_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(images_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        let ext_lower = ext.to_string_lossy().to_lowercase();
                        if ext_lower == "png" || ext_lower == "jpg" || ext_lower == "jpeg" {
                            if let Some(name) = path.file_name() {
                                files.push(name.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        files.sort();

        if files.is_empty() {
            ui.label("Nenhuma imagem encontrada na pasta 'images/'.");
            ui.label("Crie a pasta e coloque screenshots de legendas para testar.");
        } else {
            // Combo box pra selecionar o arquivo
            let current = selected_file.clone().unwrap_or_default();

            eframe::egui::ComboBox::from_id_source("lab_image_selector")
                .selected_text(if current.is_empty() {
                    "Selecione uma imagem..."
                } else {
                    &current
                })
                .show_ui(ui, |ui| {
                    for file in &files {
                        if ui
                            .selectable_value(selected_file, Some(file.clone()), file)
                            .clicked()
                        {
                            // Quando seleciona um arquivo novo, carrega a imagem
                            let path = images_dir.join(file);
                            match image::open(&path) {
                                Ok(img) => {
                                    // Cria textura da imagem original
                                    let rgba = img.to_rgba8();
                                    let size = [rgba.width() as usize, rgba.height() as usize];
                                    let pixels = rgba.into_raw();
                                    let color_image =
                                        eframe::egui::ColorImage::from_rgba_unmultiplied(
                                            size, &pixels,
                                        );

                                    *original_texture = Some(ui.ctx().load_texture(
                                        "lab_original",
                                        color_image,
                                        eframe::egui::TextureOptions::LINEAR,
                                    ));

                                    *original_image = Some(img);
                                    *needs_reprocess = true;

                                    info!("🔬 Imagem carregada: {}", file);
                                }
                                Err(e) => {
                                    error!("❌ Erro ao carregar {}: {}", file, e);
                                }
                            }
                        }
                    }
                });
        }
    });

    // Se não tem imagem carregada, para aqui
    if original_image.is_none() {
        return;
    }

    ui.add_space(10.0);

    // --- Controles de pré-processamento ---
    let mut changed = false;

    full_width_group(ui, |ui| {
        ui.label("Parametros de pre-processamento:");
        ui.add_space(5.0);

        if let Some(ref mut pp) = preprocess {
            // Checkboxes
            if ui.checkbox(&mut pp.grayscale, "Escala de cinza").changed() {
                changed = true;
            }
            if ui.checkbox(&mut pp.invert, "Inverter cores").changed() {
                changed = true;
            }

            ui.add_space(5.0);

            // Grid com sliders
            eframe::egui::Grid::new("lab_preprocess_grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Contraste:");
                    if ui
                        .add(eframe::egui::Slider::new(&mut pp.contrast, 0.5..=10.0).suffix("x"))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Threshold:");
                    let mut threshold = pp.threshold as i32;
                    if ui
                        .add(eframe::egui::Slider::new(&mut threshold, 0..=255))
                        .changed()
                    {
                        pp.threshold = threshold as u8;
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Upscale:");
                    if ui
                        .add(eframe::egui::Slider::new(&mut pp.upscale, 1.0..=4.0).suffix("x"))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Blur:");
                    if ui
                        .add(eframe::egui::Slider::new(&mut pp.blur, 0.0..=5.0).suffix("x"))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Dilatacao:");
                    let mut d = pp.dilate as i32;
                    if ui
                        .add(eframe::egui::Slider::new(&mut d, 0..=5).suffix("px"))
                        .changed()
                    {
                        pp.dilate = d as u8;
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Erosao:");
                    let mut e_val = pp.erode as i32;
                    if ui
                        .add(eframe::egui::Slider::new(&mut e_val, 0..=5).suffix("px"))
                        .changed()
                    {
                        pp.erode = e_val as u8;
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Edge Detection:");
                    let mut ed = pp.edge_detection as i32;
                    if ui
                        .add(eframe::egui::Slider::new(&mut ed, 0..=150))
                        .changed()
                    {
                        pp.edge_detection = ed as u8;
                        changed = true;
                    }
                    ui.end_row();
                });

            ui.add_space(3.0);
            ui.label("Edge Detection: 0=desativado, 30-80=recomendado (substitui threshold)");
        }
    });

    // Se algum parâmetro mudou, reprocessa a imagem
    if changed {
        *needs_reprocess = true;
    }

    // --- Botões para copiar parâmetros ---
    if preprocess.is_some() {
        ui.add_space(10.0);
        full_width_group(ui, |ui| {
            ui.label("Copiar parametros para:");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("Aplicar em Display").clicked() {
                    if let Some(ref pp) = preprocess {
                        cfg.display.preprocess = pp.clone();
                        cfg.display.preprocess.enabled = true;
                        info!("🔬 Parametros copiados para Display");
                    }
                }

                if ui.button("Aplicar em Legendas").clicked() {
                    if let Some(ref pp) = preprocess {
                        cfg.subtitle.preprocess = pp.clone();
                        cfg.subtitle.preprocess.enabled = true;
                        info!("🔬 Parametros copiados para Legendas");
                    }
                }
            });

            ui.add_space(3.0);
            ui.label("Lembre de salvar as configuracoes depois!");
        });
    }

    // Reprocessa se necessário
    if *needs_reprocess {
        if let (Some(ref img), Some(ref pp)) = (original_image, preprocess) {
            // Aplica pré-processamento usando a mesma função do pipeline real
            let processed = screenshot::preprocess_image(
                img,
                pp.grayscale,
                pp.invert,
                pp.contrast,
                pp.threshold,
                false, // não salva debug
                pp.upscale,
                pp.blur,
                pp.dilate,
                pp.erode,
                pp.edge_detection,
            );

            // Converte pra textura do egui
            let rgba = processed.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels = rgba.into_raw();
            let color_image = eframe::egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

            match processed_texture {
                Some(ref mut tex) => {
                    tex.set(color_image, eframe::egui::TextureOptions::LINEAR);
                }
                None => {
                    *processed_texture = Some(ui.ctx().load_texture(
                        "lab_processed",
                        color_image,
                        eframe::egui::TextureOptions::LINEAR,
                    ));
                }
            }

            *needs_reprocess = false;
        }
    }

    ui.add_space(10.0);

    // --- Imagens: Original e Processada ---
    let available_w = ui.available_width();

    // Imagem original
    full_width_group(ui, |ui| {
        ui.label("Imagem original:");
        ui.add_space(3.0);

        if let Some(ref texture) = original_texture {
            let tex_size = texture.size_vec2();
            let scale = ((available_w - 20.0) / tex_size.x).min(1.0);
            let display_size = eframe::egui::vec2(tex_size.x * scale, tex_size.y * scale);

            ui.image(eframe::egui::load::SizedTexture::new(
                texture.id(),
                display_size,
            ));

            ui.add_space(3.0);
            ui.label(format!(
                "{}x{} pixels",
                tex_size.x as u32, tex_size.y as u32
            ));
        }
    });

    ui.add_space(10.0);

    // Imagem processada
    full_width_group(ui, |ui| {
        ui.label("Imagem processada:");
        ui.add_space(3.0);

        if let Some(ref texture) = processed_texture {
            let tex_size = texture.size_vec2();
            let scale = ((available_w - 20.0) / tex_size.x).min(1.0);
            let display_size = eframe::egui::vec2(tex_size.x * scale, tex_size.y * scale);

            ui.image(eframe::egui::load::SizedTexture::new(
                texture.id(),
                display_size,
            ));

            ui.add_space(3.0);
            ui.label(format!(
                "{}x{} pixels",
                tex_size.x as u32, tex_size.y as u32
            ));
        }
    });
}

// ============================================================================
// FUNÇÕES AUXILIARES (reutilizadas em múltiplas abas)
// ============================================================================

/// Intervalo de refresh da imagem debug (em milissegundos)
const DEBUG_PREVIEW_REFRESH_MS: u128 = 500;

/// Renderiza a preview da imagem debug de pré-processamento.
/// Lê o arquivo debug_preprocessed.png do disco e mostra na tela.
/// Atualiza automaticamente a cada 500ms.
fn render_debug_preview(
    ui: &mut eframe::egui::Ui,
    debug_texture: &mut Option<eframe::egui::TextureHandle>,
    last_update: &mut std::time::Instant,
    subtitle_state: &subtitle::SubtitleState,
) {
    full_width_group(ui, |ui| {
        ui.label("Preview do pre-processamento (auto-refresh):");
        ui.add_space(5.0);

        // Verifica se precisa atualizar a textura (a cada 500ms)
        let needs_update = last_update.elapsed().as_millis() >= DEBUG_PREVIEW_REFRESH_MS;

        if needs_update {
            // Tenta ler o arquivo de debug do disco
            let path = std::path::Path::new("debug_preprocessed.png");

            if path.exists() {
                match image::open(path) {
                    Ok(img) => {
                        // Converte a imagem pra RGBA8
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();

                        // Cria ColorImage pro egui
                        let color_image =
                            eframe::egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

                        // Cria ou atualiza a textura
                        match debug_texture {
                            Some(ref mut tex) => {
                                // Atualiza textura existente
                                tex.set(color_image, eframe::egui::TextureOptions::LINEAR);
                            }
                            None => {
                                // Cria nova textura
                                *debug_texture = Some(ui.ctx().load_texture(
                                    "debug_preview",
                                    color_image,
                                    eframe::egui::TextureOptions::LINEAR,
                                ));
                            }
                        }

                        *last_update = std::time::Instant::now();
                    }
                    Err(e) => {
                        ui.label(format!("Erro ao ler imagem: {}", e));
                    }
                }
            } else {
                ui.label("Arquivo debug_preprocessed.png nao encontrado.");
                ui.label("Faca uma captura primeiro para gerar a imagem.");
            }
        }

        // Renderiza a textura se existir
        if let Some(ref texture) = debug_texture {
            let tex_size = texture.size_vec2();

            // Escala pra caber na largura disponível, mantendo proporção
            let available_w = ui.available_width();
            let scale = (available_w / tex_size.x).min(1.0); // Não amplia, só reduz
            let display_size = eframe::egui::vec2(tex_size.x * scale, tex_size.y * scale);

            ui.image(eframe::egui::load::SizedTexture::new(
                texture.id(),
                display_size,
            ));

            // Info sobre a imagem
            ui.add_space(3.0);
            ui.label(format!(
                "{}x{} pixels (atualiza a cada {}ms)",
                tex_size.x as u32, tex_size.y as u32, DEBUG_PREVIEW_REFRESH_MS,
            ));
        }

        // --- Última tradução ---
        let history = subtitle_state.get_full_history();
        if let Some(last) = history.last() {
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(3.0);
            ui.label("Ultima traducao:");
            ui.label(
                eframe::egui::RichText::new(&last.translated)
                    .size(16.0)
                    .color(eframe::egui::Color32::from_rgb(100, 200, 255)),
            );
        }

        // Força repaint pra manter o auto-refresh funcionando
        ui.ctx().request_repaint();
    });
}

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

/// Renderiza um combo de hotkey com modificador + tecla.
/// Mostra dois ComboBox lado a lado: [Modificador] [Tecla]
fn render_hotkey_combo(
    ui: &mut eframe::egui::Ui,
    id: &str,
    label: &str,
    binding: &mut config::HotkeyBinding,
    modifiers: &[&str],
    keys: &[&str],
) {
    ui.horizontal(|ui| {
        // Largura fixa pro label garante que todos os combos alinhem
        let label_response = ui.label(label);
        let used = label_response.rect.width();
        let min_label = 150.0;
        if used < min_label {
            ui.add_space(min_label - used);
        }

        // ComboBox do modificador
        let mod_display = if binding.modifier.is_empty() {
            "Nenhum"
        } else {
            &binding.modifier
        };

        eframe::egui::ComboBox::from_id_source(format!("{}_mod", id))
            .selected_text(mod_display)
            .width(70.0)
            .show_ui(ui, |ui| {
                for m in modifiers {
                    let display = if m.is_empty() { "Nenhum" } else { m };
                    ui.selectable_value(&mut binding.modifier, m.to_string(), display);
                }
            });

        ui.label("+");

        // ComboBox da tecla principal
        eframe::egui::ComboBox::from_id_source(format!("{}_key", id))
            .selected_text(&binding.key)
            .width(130.0)
            .show_ui(ui, |ui| {
                for k in keys {
                    ui.selectable_value(&mut binding.key, k.to_string(), *k);
                }
            });
    });
}
