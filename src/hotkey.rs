// ============================================================================
// MÓDULO HOTKEY - Gerenciamento de hotkeys usando device_query
// ============================================================================

use device_query::{DeviceQuery, DeviceState, Keycode};
use std::thread;
use std::time::Duration;

/// Estrutura que gerencia hotkeys
pub struct HotkeyManager {
    device_state: DeviceState,
}

/// Tipo de captura solicitada pelo usuário
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CaptureMode {
    /// Captura a tela inteira
    FullScreen,
    /// Captura apenas a região customizada
    Region,
}

impl HotkeyManager {
    /// Cria um novo gerenciador de hotkeys
    pub fn new() -> Self {
        info!("⌨️  Configurando detecção de teclas...");

        let device_state = DeviceState::new();

        info!("✅ Detecção de teclas configurada!");

        HotkeyManager { device_state }
    }

    /// Verifica se alguma hotkey foi pressionada e retorna qual
    ///
    /// # Retorna
    /// * `Some(CaptureMode)` - Se alguma hotkey foi pressionada
    /// * `None` - Se nenhuma hotkey está pressionada
    pub fn check_hotkey(&self) -> Option<CaptureMode> {
        let keys = self.device_state.get_keys();

        // Verifica Numpad + (região customizada)
        if keys.contains(&Keycode::NumpadAdd) {
            return Some(CaptureMode::Region);
        }

        // Verifica Numpad - (tela inteira)
        if keys.contains(&Keycode::NumpadSubtract) {
            return Some(CaptureMode::FullScreen);
        }

        None
    }

    /// Aguarda a tecla ser solta (para evitar múltiplos triggers)
    pub fn wait_for_key_release(&self) {
        info!("⏳ Aguardando tecla ser solta...");

        // Fica em loop enquanto QUALQUER uma das teclas estiver pressionada
        while self.check_hotkey().is_some() {
            thread::sleep(Duration::from_millis(50));
        }

        info!("✅ Tecla solta!");
    }
}
