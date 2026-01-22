// game-translator/src/hotkey.rs

// ============================================================================
// MÓDULO HOTKEY - Gerenciamento de hotkeys usando device_query
// ============================================================================

use device_query::{DeviceQuery, DeviceState, Keycode};
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Estrutura que gerencia hotkeys
pub struct HotkeyManager {
    device_state: DeviceState,
    pressed_keys: HashSet<Keycode>, // Teclas que já foram processadas
    last_action_time: Instant,      // Para evitar spam global
}

/// Tipo de captura solicitada pelo usuário
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HotkeyAction {
    /// Captura a tela inteira
    TranslateFullScreen,
    /// Captura apenas a região customizada
    TranslateRegion,
    /// Abre o seletor de região
    SelectRegion,
    /// Abre o seletor de região de legendas
    SelectSubtitleRegion,
    /// Liga/desliga o modo de legendas em tempo real
    ToggleSubtitleMode,
}

impl HotkeyManager {
    pub fn new() -> Self {
        info!("⌨️  Configurando detecção de teclas...");
        let device_state = DeviceState::new();
        info!("✅ Detecção de teclas configurada!");
        HotkeyManager {
            device_state,
            pressed_keys: HashSet::new(),
            last_action_time: Instant::now(),
        }
    }

    /// Verifica se alguma hotkey foi pressionada e retorna qual
    ///
    /// # Retorna
    /// * `Some(HotkeyAction)` - Se alguma hotkey foi pressionada
    /// * `None` - Se nenhuma hotkey está pressionada
    pub fn check_hotkey(&mut self) -> Option<HotkeyAction> {
        // Evita múltiplas execuções rápidas (ex: <200ms)
        if self.last_action_time.elapsed() < Duration::from_millis(200) {
            return None;
        }

        let keys = self.device_state.get_keys();

        // Lista de teclas e suas ações correspondentes
        let key_actions = [
            (Keycode::NumpadMultiply, HotkeyAction::SelectRegion),
            (Keycode::NumpadAdd, HotkeyAction::TranslateRegion),
            (Keycode::NumpadSubtract, HotkeyAction::TranslateFullScreen),
            (Keycode::NumpadDivide, HotkeyAction::SelectSubtitleRegion),
            (Keycode::Numpad0, HotkeyAction::ToggleSubtitleMode),
        ];

        for &(key, action) in &key_actions {
            if keys.contains(&key) {
                if !self.pressed_keys.contains(&key) {
                    self.pressed_keys.insert(key);
                    self.last_action_time = Instant::now();
                    return Some(action);
                }
                // Já pressionada — não faz nada
                return None;
            } else {
                // Tecla foi solta — remove do conjunto
                self.pressed_keys.remove(&key);
            }
        }

        None
    }
}
