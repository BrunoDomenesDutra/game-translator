// game-translator/src/overlay/commands.rs

// ============================================================================
// PROCESSAMENTO DE COMANDOS DO OVERLAY
// ============================================================================

use crate::app_state::{AppCommand, AppState};
use crate::config;
use crate::region_selector;
use crossbeam_channel::Receiver;

/// Processa comandos pendentes enviados para a thread principal.
#[allow(clippy::too_many_arguments)]
pub fn process_pending_commands(
    ctx: &eframe::egui::Context,
    state: &AppState,
    command_receiver: &Receiver<AppCommand>,
    settings_config: &mut Option<config::AppConfig>,
    settings_tab: &mut u8,
    settings_status: &mut Option<(String, std::time::Instant)>,
    settings_positioned: &mut bool,
) {
    while let Ok(command) = command_receiver.try_recv() {
        match command {
            AppCommand::OpenRegionSelector => {
                info!("🎯 Abrindo seletor de região...");

                // Esconde o overlay temporariamente
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                    eframe::egui::vec2(1.0, 1.0),
                ));

                // Abre o seletor de região
                match region_selector::select_region(None) {
                    Ok(Some(selected)) => {
                        info!(
                            "✅ Região selecionada: {}x{} na posição ({}, {})",
                            selected.width, selected.height, selected.x, selected.y
                        );

                        let mut config = state.config.lock().unwrap();
                        if let Err(e) = config.app_config.update_region(
                            selected.x,
                            selected.y,
                            selected.width,
                            selected.height,
                        ) {
                            error!("❌ Erro ao salvar região: {}", e);
                        } else {
                            info!("💾 Região salva no config.json!");
                            config.region_x = selected.x;
                            config.region_y = selected.y;
                            config.region_width = selected.width;
                            config.region_height = selected.height;
                        }
                    }
                    Ok(None) => info!("❌ Seleção cancelada"),
                    Err(e) => error!("❌ Erro no seletor: {}", e),
                }
            }

            AppCommand::OpenSubtitleRegionSelector => {
                info!("📺 Abrindo seletor de região de legendas...");

                // Esconde o overlay temporariamente
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                    eframe::egui::vec2(1.0, 1.0),
                ));

                // Abre o seletor de região
                match region_selector::select_region(Some("SELEÇÃO ÁREA DE LEGENDA")) {
                    Ok(Some(selected)) => {
                        info!(
                            "✅ Região de legendas selecionada: {}x{} na posição ({}, {})",
                            selected.width, selected.height, selected.x, selected.y
                        );

                        let mut config = state.config.lock().unwrap();
                        // Atualiza a região de legendas
                        config.app_config.subtitle.region.x = selected.x;
                        config.app_config.subtitle.region.y = selected.y;
                        config.app_config.subtitle.region.width = selected.width;
                        config.app_config.subtitle.region.height = selected.height;

                        // Salva no arquivo
                        if let Err(e) = config.app_config.save() {
                            error!("❌ Erro ao salvar região de legendas: {}", e);
                        } else {
                            info!("💾 Região de legendas salva no config.json!");
                        }
                    }
                    Ok(None) => info!("❌ Seleção cancelada"),
                    Err(e) => error!("❌ Erro no seletor: {}", e),
                }
            }

            AppCommand::OpenSettings => {
                info!("⚙️  Entrando no modo configurações...");

                // Copia as configurações atuais para edição
                let config = state.config.lock().unwrap();
                *settings_config = Some(config.app_config.clone());
                drop(config);

                // Ativa o modo configurações
                *state.settings_mode.lock().unwrap() = true;
                *settings_tab = 0;
                *settings_status = None;
            }

            AppCommand::CloseSettings => {
                info!("⚙️  Saindo do modo configurações...");

                *state.settings_mode.lock().unwrap() = false;
                *settings_config = None;
                *settings_positioned = false; // Reseta pra próxima abertura

                // Restaura janela para modo overlay
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Decorations(false));
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Resizable(false));
            }
        }
    }
}
