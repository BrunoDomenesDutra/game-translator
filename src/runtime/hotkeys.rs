// game-translator/src/runtime/hotkeys.rs

// ============================================================================
// THREAD DE HOTKEYS (roda em background)
// ============================================================================

use crate::app_state::{AppCommand, AppState};
use crate::hotkey;
use crate::processing;
use std::thread;
use std::time::Duration;

pub fn start_hotkey_thread(state: AppState) {
    thread::spawn(move || {
        info!("⌨️  Thread de hotkeys iniciada");

        // Pega as configurações de hotkeys
        let hotkeys = state.config.lock().unwrap().app_config.hotkeys.clone();
        let mut hotkey_manager = hotkey::HotkeyManager::new(&hotkeys);

        loop {
            // Verifica se precisa recarregar as hotkeys (config foi salvo)
            {
                let mut needs_reload = state.hotkeys_need_reload.lock().unwrap();
                if *needs_reload {
                    let hotkeys = state.config.lock().unwrap().app_config.hotkeys.clone();
                    hotkey_manager = hotkey::HotkeyManager::new(&hotkeys);
                    *needs_reload = false;
                    info!("⌨️  Hotkeys recarregadas!");
                }
            }

            if let Some(action) = hotkey_manager.check_hotkey() {
                // Se está no modo configurações, ignora TODAS as hotkeys
                // exceto OpenSettings (pra poder fechar a janela)
                let is_settings = *state.settings_mode.lock().unwrap();
                if is_settings && action != hotkey::HotkeyAction::OpenSettings {
                    continue;
                }

                match action {
                    hotkey::HotkeyAction::SelectRegion => {
                        info!("");
                        info!("🎯 ============================================");
                        info!("🎯 SOLICITANDO ABERTURA DO SELETOR DE REGIÃO");
                        info!("🎯 ============================================");

                        if let Err(e) = state.command_sender.send(AppCommand::OpenRegionSelector) {
                            error!("❌ Erro ao enviar comando: {}", e);
                        }
                    }

                    hotkey::HotkeyAction::SelectSubtitleRegion => {
                        info!("");
                        info!("📺 ============================================");
                        info!("📺 SOLICITANDO ABERTURA DO SELETOR DE LEGENDA");
                        info!("📺 ============================================");

                        if let Err(e) = state
                            .command_sender
                            .send(AppCommand::OpenSubtitleRegionSelector)
                        {
                            error!("❌ Erro ao enviar comando: {}", e);
                        }
                    }

                    hotkey::HotkeyAction::ToggleSubtitleAreasPreview => {
                        let mut preview_active =
                            state.subtitle_areas_preview_active.lock().unwrap();
                        *preview_active = !*preview_active;

                        info!("");
                        if *preview_active {
                            info!("🧭 ============================================");
                            info!("🧭 PREVIEW DE ÁREAS DE LEGENDA: ✅ ATIVADO");
                            info!("🧭 ============================================");
                        } else {
                            info!("🧭 ============================================");
                            info!("🧭 PREVIEW DE ÁREAS DE LEGENDA: ❌ DESATIVADO");
                            info!("🧭 ============================================");
                        }
                    }

                    hotkey::HotkeyAction::HideTranslation => {
                        info!("");
                        info!("🙈 ============================================");
                        info!("🙈 ESCONDENDO TRADUÇÃO");
                        info!("🙈 ============================================");

                        state.clear_translations();
                    }

                    hotkey::HotkeyAction::ToggleSubtitleMode => {
                        let mut active = state.subtitle_mode_active.lock().unwrap();
                        *active = !*active;

                        info!("");
                        if *active {
                            info!("📺 ============================================");
                            info!("📺 MODO LEGENDA: ✅ ATIVADO");
                            info!("📺 ============================================");
                        } else {
                            info!("📺 ============================================");
                            info!("📺 MODO LEGENDA: ❌ DESATIVADO");
                            info!("📺 ============================================");
                        }
                    }

                    hotkey::HotkeyAction::TranslateFullScreen => {
                        info!("");
                        info!("▶️  ============================================");
                        info!("▶️  MODO: 🖥️  TELA INTEIRA");
                        info!("▶️  ============================================");

                        let state_clone = state.clone();
                        thread::spawn(move || {
                            if let Err(e) = processing::process_translation_blocking(
                                &state_clone,
                                hotkey::HotkeyAction::TranslateFullScreen,
                            ) {
                                error!("❌ Erro: {}", e);
                            }
                        });
                    }

                    hotkey::HotkeyAction::TranslateRegion => {
                        info!("");
                        info!("▶️  ============================================");
                        info!("▶️  MODO: 🎯 REGIÃO CUSTOMIZADA");
                        info!("▶️  ============================================");

                        let state_clone = state.clone();
                        thread::spawn(move || {
                            if let Err(e) = processing::process_translation_blocking(
                                &state_clone,
                                hotkey::HotkeyAction::TranslateRegion,
                            ) {
                                error!("❌ Erro: {}", e);
                            }
                        });
                    }

                    hotkey::HotkeyAction::OpenSettings => {
                        // Verifica se já está no modo configurações
                        let is_settings = *state.settings_mode.lock().unwrap();

                        if is_settings {
                            info!("");
                            info!("⚙️  ============================================");
                            info!("⚙️  FECHANDO JANELA DE CONFIGURAÇÕES");
                            info!("⚙️  ============================================");

                            if let Err(e) = state.command_sender.send(AppCommand::CloseSettings) {
                                error!("❌ Erro ao enviar comando: {}", e);
                            }
                        } else {
                            info!("");
                            info!("⚙️  ============================================");
                            info!("⚙️  ABRINDO JANELA DE CONFIGURAÇÕES");
                            info!("⚙️  ============================================");

                            if let Err(e) = state.command_sender.send(AppCommand::OpenSettings) {
                                error!("❌ Erro ao enviar comando: {}", e);
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(50));
        }
    });
}

