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
    // Mapeamento de ações para teclas (lido do config)
    hotkey_translate_fullscreen: Keycode,
    hotkey_translate_region: Keycode,
    hotkey_select_region: Keycode,
    hotkey_select_subtitle_region: Keycode,
    hotkey_toggle_subtitle_mode: Keycode,
    hotkey_hide_translation: Keycode,
    hotkey_open_settings: Keycode,
}

/// Tipo de ação solicitada pelo usuário
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
    /// Esconde a tradução atual
    HideTranslation,
    /// Abre a janela de configurações
    OpenSettings,
}

/// Converte uma string do config para Keycode
fn string_to_keycode(s: &str) -> Keycode {
    match s {
        // Numpad
        "Numpad0" => Keycode::Numpad0,
        "Numpad1" => Keycode::Numpad1,
        "Numpad2" => Keycode::Numpad2,
        "Numpad3" => Keycode::Numpad3,
        "Numpad4" => Keycode::Numpad4,
        "Numpad5" => Keycode::Numpad5,
        "Numpad6" => Keycode::Numpad6,
        "Numpad7" => Keycode::Numpad7,
        "Numpad8" => Keycode::Numpad8,
        "Numpad9" => Keycode::Numpad9,
        "NumpadAdd" => Keycode::NumpadAdd,
        "NumpadSubtract" => Keycode::NumpadSubtract,
        "NumpadMultiply" => Keycode::NumpadMultiply,
        "NumpadDivide" => Keycode::NumpadDivide,
        "NumpadDecimal" => Keycode::NumpadDecimal,
        // Teclas de função
        "F1" => Keycode::F1,
        "F2" => Keycode::F2,
        "F3" => Keycode::F3,
        "F4" => Keycode::F4,
        "F5" => Keycode::F5,
        "F6" => Keycode::F6,
        "F7" => Keycode::F7,
        "F8" => Keycode::F8,
        "F9" => Keycode::F9,
        "F10" => Keycode::F10,
        "F11" => Keycode::F11,
        "F12" => Keycode::F12,
        // Padrão
        _ => {
            warn!("⚠️  Tecla desconhecida: {}, usando Numpad0", s);
            Keycode::Numpad0
        }
    }
}

impl HotkeyManager {
    pub fn new(hotkeys: &crate::config::HotkeyConfig) -> Self {
        info!("⌨️  Configurando detecção de teclas...");
        let device_state = DeviceState::new();

        // Converte as strings do config para Keycode
        let hotkey_translate_fullscreen = string_to_keycode(&hotkeys.translate_fullscreen);
        let hotkey_translate_region = string_to_keycode(&hotkeys.translate_region);
        let hotkey_select_region = string_to_keycode(&hotkeys.select_region);
        let hotkey_select_subtitle_region = string_to_keycode(&hotkeys.select_subtitle_region);
        let hotkey_toggle_subtitle_mode = string_to_keycode(&hotkeys.toggle_subtitle_mode);
        let hotkey_hide_translation = string_to_keycode(&hotkeys.hide_translation);
        let hotkey_open_settings = Keycode::Numpad5; // Fixo por enquanto

        info!("✅ Detecção de teclas configurada!");
        info!("   Tela cheia: {:?}", hotkey_translate_fullscreen);
        info!("   Região: {:?}", hotkey_translate_region);
        info!("   Selecionar região: {:?}", hotkey_select_region);

        HotkeyManager {
            device_state,
            pressed_keys: HashSet::new(),
            last_action_time: Instant::now(),
            hotkey_translate_fullscreen,
            hotkey_translate_region,
            hotkey_select_region,
            hotkey_select_subtitle_region,
            hotkey_toggle_subtitle_mode,
            hotkey_hide_translation,
            hotkey_open_settings,
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

        // Lista de teclas e suas ações correspondentes (lidas do config)
        let key_actions = [
            (self.hotkey_select_region, HotkeyAction::SelectRegion),
            (self.hotkey_translate_region, HotkeyAction::TranslateRegion),
            (
                self.hotkey_translate_fullscreen,
                HotkeyAction::TranslateFullScreen,
            ),
            (
                self.hotkey_select_subtitle_region,
                HotkeyAction::SelectSubtitleRegion,
            ),
            (
                self.hotkey_toggle_subtitle_mode,
                HotkeyAction::ToggleSubtitleMode,
            ),
            (self.hotkey_hide_translation, HotkeyAction::HideTranslation),
            (self.hotkey_open_settings, HotkeyAction::OpenSettings),
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
