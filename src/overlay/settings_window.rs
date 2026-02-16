// game-translator/src/overlay/settings_window.rs

// ============================================================================
// JANELA DE CONFIGURACOES DO OVERLAY
// ============================================================================

use crate::app_state::AppState;
use crate::config;
use crate::settings_ui;

/// Renderiza a janela de configuracoes quando o modo settings estiver ativo.
/// Retorna true quando a janela de configuracoes foi renderizada neste frame.
#[allow(clippy::too_many_arguments)]
pub fn render_settings_window(
    ctx: &eframe::egui::Context,
    state: &AppState,
    settings_config: &mut Option<config::AppConfig>,
    settings_tab: &mut u8,
    settings_status: &mut Option<(String, std::time::Instant)>,
    settings_positioned: &mut bool,
    debug_texture: &mut Option<eframe::egui::TextureHandle>,
    debug_texture_last_update: &mut std::time::Instant,
    lab_original_texture: &mut Option<eframe::egui::TextureHandle>,
    lab_processed_texture: &mut Option<eframe::egui::TextureHandle>,
    lab_preprocess: &mut Option<crate::config::PreprocessConfig>,
    lab_selected_file: &mut Option<String>,
    lab_original_image: &mut Option<image::DynamicImage>,
    lab_needs_reprocess: &mut bool,
) -> bool {
    let is_settings_mode = *state.settings_mode.lock().unwrap();
    if !is_settings_mode {
        return false;
    }

    // Na primeira vez que abre as configs, centraliza e define tamanho inicial.
    // Depois disso, não força mais posição/tamanho - deixa o usuário mover e redimensionar.
    {
        if !*settings_positioned {
            *settings_positioned = true;

            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Decorations(true));
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Resizable(true));
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(eframe::egui::vec2(
                650.0, 900.0,
            )));
            // Tamanho mínimo da janela de configurações
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::MinInnerSize(eframe::egui::vec2(
                650.0, 400.0,
            )));

            let screen_w = unsafe {
                winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN)
            } as f32
                / state.dpi_scale;
            let screen_h = unsafe {
                winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN)
            } as f32
                / state.dpi_scale;

            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(eframe::egui::pos2(
                (screen_w - 650.0) / 2.0,
                (screen_h - 900.0) / 2.0,
            )));
        }
    }

    // Remove transparência temporariamente
    let visuals = eframe::egui::Visuals::dark();
    ctx.set_visuals(visuals);

    // ================================================================
    // HEADER FIXO - Título + Abas (sempre visível no topo)
    // ================================================================
    eframe::egui::TopBottomPanel::top("settings_header").show(ctx, |ui| {
        ui.add_space(5.0);

        // Abas de navegação
        ui.horizontal(|ui| {
            if ui.selectable_label(*settings_tab == 0, "Overlay").clicked() {
                *settings_tab = 0;
            }
            if ui.selectable_label(*settings_tab == 1, "Fonte").clicked() {
                *settings_tab = 1;
            }
            if ui.selectable_label(*settings_tab == 2, "Display").clicked() {
                *settings_tab = 2;
            }
            if ui.selectable_label(*settings_tab == 3, "Legendas").clicked() {
                *settings_tab = 3;
            }
            if ui.selectable_label(*settings_tab == 4, "Atalhos").clicked() {
                *settings_tab = 4;
            }
            if ui.selectable_label(*settings_tab == 5, "Servicos").clicked() {
                *settings_tab = 5;
            }
            if ui.selectable_label(*settings_tab == 6, "Historico").clicked() {
                *settings_tab = 6;
            }
            if ui.selectable_label(*settings_tab == 7, "OpenAI").clicked() {
                *settings_tab = 7;
            }
            if ui.selectable_label(*settings_tab == 8, "Laboratorio").clicked() {
                *settings_tab = 8;
            }
        });

        ui.add_space(3.0);
    });

    // ================================================================
    // FOOTER FIXO - Botões de ação (sempre visível embaixo)
    // ================================================================
    eframe::egui::TopBottomPanel::bottom("settings_footer").show(ctx, |ui| {
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            // Botão Salvar
            if ui.button("Salvar").clicked() {
                if let Some(ref cfg) = settings_config {
                    match cfg.save() {
                        Ok(_) => {
                            let mut config = state.config.lock().unwrap();
                            config.app_config = cfg.clone();
                            // Sinaliza pra thread de hotkeys recarregar
                            *state.hotkeys_need_reload.lock().unwrap() = true;
                            // Sinaliza pra recarregar a fonte de tradução
                            *state.font_need_reload.lock().unwrap() = true;
                            *settings_status =
                                Some(("Salvo!".to_string(), std::time::Instant::now()));
                            info!("Configuracoes salvas!");
                        }
                        Err(e) => {
                            *settings_status =
                                Some((format!("Erro: {}", e), std::time::Instant::now()));
                            error!("Erro ao salvar: {}", e);
                        }
                    }
                }
            }

            // Botão Recarregar
            if ui.button("Recarregar").clicked() {
                match config::AppConfig::load() {
                    Ok(cfg) => {
                        *settings_config = Some(cfg);
                        *settings_status =
                            Some(("Recarregado!".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        *settings_status = Some((format!("Erro: {}", e), std::time::Instant::now()));
                    }
                }
            }

            // Mostra status temporário (3 segundos)
            if let Some((ref msg, time)) = settings_status {
                if time.elapsed() < std::time::Duration::from_secs(3) {
                    ui.label(msg);
                }
            }

            // Botões do lado direito
            ui.with_layout(
                eframe::egui::Layout::right_to_left(eframe::egui::Align::Center),
                |ui| {
                    if ui.button("Sair do Programa").clicked() {
                        std::process::exit(0);
                    }
                    ui.add_space(5.0);
                    if ui.button("Fechar").clicked() {
                        *state.settings_mode.lock().unwrap() = false;
                        *settings_config = None;
                        *settings_positioned = false;
                        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Decorations(false));
                        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Resizable(false));
                    }
                },
            );
        });

        ui.add_space(5.0);
    });

    // ================================================================
    // CONTEÚDO COM SCROLL (entre header e footer)
    // ================================================================
    eframe::egui::CentralPanel::default().show(ctx, |ui| {
        eframe::egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(5.0);
            if let Some(ref mut cfg) = settings_config {
                settings_ui::render_tab(
                    ui,
                    *settings_tab,
                    cfg,
                    &state.subtitle_state,
                    &state.openai_request_count,
                    debug_texture,
                    debug_texture_last_update,
                    lab_original_texture,
                    lab_processed_texture,
                    lab_preprocess,
                    lab_selected_file,
                    lab_original_image,
                    lab_needs_reprocess,
                );
            }

            // Espaço extra no final pra não colar no footer
            ui.add_space(5.0);
        });
    });

    ctx.request_repaint();
    true
}
