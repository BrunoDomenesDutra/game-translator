// game-translator/src/runtime/config_watcher.rs

// ============================================================================
// THREAD DE CONFIG WATCHER (monitora mudancas no config.json)
// ============================================================================

use crate::app_state::AppState;
use crate::config::Config;
use notify::{RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

pub fn start_config_watcher(state: AppState) {
    thread::spawn(move || {
        info!("???  Thread de monitoramento do config.json iniciada");

        let (tx, rx) = channel();

        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                error!("? Erro ao criar watcher: {}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(Path::new("config.json"), RecursiveMode::NonRecursive) {
            error!("? Erro ao monitorar config.json: {}", e);
            return;
        }

        info!("? Monitorando config.json para mudanças...");

        let mut last_reload = std::time::Instant::now();
        let debounce_duration = Duration::from_millis(500);

        loop {
            match rx.recv() {
                Ok(event_result) => {
                    if let Ok(event) = event_result {
                        if matches!(event.kind, notify::EventKind::Modify(_)) {
                            if last_reload.elapsed() < debounce_duration {
                                continue;
                            }

                            last_reload = std::time::Instant::now();

                            info!("");
                            info!("?? CONFIG.JSON MODIFICADO - RECARREGANDO");

                            thread::sleep(Duration::from_millis(100));

                            match Config::load() {
                                Ok(new_config) => {
                                    let mut config = state.config.lock().unwrap();
                                    *config = new_config;
                                    info!("? Configurações recarregadas!");
                                }
                                Err(e) => {
                                    error!("? Erro ao recarregar config: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("? Erro ao receber evento: {}", e);
                    break;
                }
            }
        }
    });
}

