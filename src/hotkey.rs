// game-translator/src/hotkey.rs

// ============================================================================
// MÓDULO HOTKEY - Gerenciamento de hotkeys com suporte a modificadores
// ============================================================================
// Suporta combinações como Ctrl+T, Shift+F1, Alt+Numpad0
// ou teclas simples como Numpad0, F5, etc.
// ============================================================================

use device_query::{DeviceQuery, DeviceState, Keycode};
use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::config::HotkeyBinding;

/// Estrutura interna que representa um atalho já convertido pra Keycode
struct ParsedBinding {
    /// Tecla modificadora (None = sem modificador)
    modifier: Option<Keycode>,
    /// Tecla principal
    key: Keycode,
    /// Ação associada
    action: HotkeyAction,
}

/// Estrutura que gerencia hotkeys
pub struct HotkeyManager {
    device_state: DeviceState,
    /// Teclas que já foram processadas (evita repetição enquanto segura)
    pressed_keys: HashSet<Keycode>,
    /// Para evitar múltiplas ações rápidas
    last_action_time: Instant,
    /// Lista de atalhos configurados
    bindings: Vec<ParsedBinding>,
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
    /// Mostra/oculta o preview visual das áreas de legenda
    ToggleSubtitleAreasPreview,
    /// Liga/desliga o modo de legendas em tempo real
    ToggleSubtitleMode,
    /// Esconde a tradução atual
    HideTranslation,
    /// Abre a janela de configurações
    OpenSettings,
}

/// Converte uma string de modificador para Keycode
fn modifier_to_keycode(s: &str) -> Option<Keycode> {
    match s.to_lowercase().as_str() {
        "ctrl" | "control" | "lcontrol" => Some(Keycode::LControl),
        "shift" | "lshift" => Some(Keycode::LShift),
        "alt" | "lalt" => Some(Keycode::LAlt),
        "" => None,
        other => {
            warn!("⚠️  Modificador desconhecido: '{}', ignorando", other);
            None
        }
    }
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
        // Letras A-Z
        "A" => Keycode::A,
        "B" => Keycode::B,
        "C" => Keycode::C,
        "D" => Keycode::D,
        "E" => Keycode::E,
        "F" => Keycode::F,
        "G" => Keycode::G,
        "H" => Keycode::H,
        "I" => Keycode::I,
        "J" => Keycode::J,
        "K" => Keycode::K,
        "L" => Keycode::L,
        "M" => Keycode::M,
        "N" => Keycode::N,
        "O" => Keycode::O,
        "P" => Keycode::P,
        "Q" => Keycode::Q,
        "R" => Keycode::R,
        "S" => Keycode::S,
        "T" => Keycode::T,
        "U" => Keycode::U,
        "V" => Keycode::V,
        "W" => Keycode::W,
        "X" => Keycode::X,
        "Y" => Keycode::Y,
        "Z" => Keycode::Z,
        // Números do teclado principal
        "0" => Keycode::Key0,
        "1" => Keycode::Key1,
        "2" => Keycode::Key2,
        "3" => Keycode::Key3,
        "4" => Keycode::Key4,
        "5" => Keycode::Key5,
        "6" => Keycode::Key6,
        "7" => Keycode::Key7,
        "8" => Keycode::Key8,
        "9" => Keycode::Key9,
        // Teclas especiais
        "Space" => Keycode::Space,
        "Enter" => Keycode::Enter,
        "Escape" => Keycode::Escape,
        "Tab" => Keycode::Tab,
        "Backspace" => Keycode::Backspace,
        "Insert" => Keycode::Insert,
        "Delete" => Keycode::Delete,
        "Home" => Keycode::Home,
        "End" => Keycode::End,
        "PageUp" => Keycode::PageUp,
        "PageDown" => Keycode::PageDown,
        // Padrão
        _ => {
            warn!("⚠️  Tecla desconhecida: '{}', usando Numpad0", s);
            Keycode::Numpad0
        }
    }
}

/// Converte um HotkeyBinding do config em um ParsedBinding interno
fn parse_binding(binding: &HotkeyBinding, action: HotkeyAction) -> ParsedBinding {
    ParsedBinding {
        modifier: modifier_to_keycode(&binding.modifier),
        key: string_to_keycode(&binding.key),
        action,
    }
}

impl HotkeyManager {
    pub fn new(hotkeys: &crate::config::HotkeyConfig) -> Self {
        info!("⌨️  Configurando detecção de teclas...");
        let device_state = DeviceState::new();

        // Converte todos os bindings do config
        let bindings = vec![
            parse_binding(&hotkeys.select_region, HotkeyAction::SelectRegion),
            parse_binding(&hotkeys.translate_region, HotkeyAction::TranslateRegion),
            parse_binding(
                &hotkeys.translate_fullscreen,
                HotkeyAction::TranslateFullScreen,
            ),
            parse_binding(
                &hotkeys.select_subtitle_region,
                HotkeyAction::SelectSubtitleRegion,
            ),
            parse_binding(
                &hotkeys.toggle_subtitle_areas_preview,
                HotkeyAction::ToggleSubtitleAreasPreview,
            ),
            parse_binding(
                &hotkeys.toggle_subtitle_mode,
                HotkeyAction::ToggleSubtitleMode,
            ),
            parse_binding(&hotkeys.hide_translation, HotkeyAction::HideTranslation),
            parse_binding(&hotkeys.open_settings, HotkeyAction::OpenSettings),
        ];

        // Log dos atalhos configurados
        for b in &bindings {
            let mod_str = match &b.modifier {
                Some(m) => format!("{:?} + ", m),
                None => String::new(),
            };
            info!("   {:?}: {}{:?}", b.action, mod_str, b.key);
        }

        info!("✅ Detecção de teclas configurada!");

        HotkeyManager {
            device_state,
            pressed_keys: HashSet::new(),
            last_action_time: Instant::now(),
            bindings,
        }
    }

    /// Verifica se alguma hotkey foi pressionada e retorna qual ação
    pub fn check_hotkey(&mut self) -> Option<HotkeyAction> {
        // Evita múltiplas execuções rápidas
        if self.last_action_time.elapsed() < Duration::from_millis(200) {
            return None;
        }

        let keys = self.device_state.get_keys();

        for binding in &self.bindings {
            // Verifica se a tecla principal está pressionada
            if keys.contains(&binding.key) {
                // Verifica se o modificador está correto
                let modifier_ok = match &binding.modifier {
                    // Se precisa de modificador, verifica se está pressionado
                    // Aceita tanto Left quanto Right (Ctrl, Shift, Alt)
                    Some(modifier) => {
                        keys.contains(modifier)
                            || match modifier {
                                Keycode::LControl => keys.contains(&Keycode::RControl),
                                Keycode::LShift => keys.contains(&Keycode::RShift),
                                Keycode::LAlt => keys.contains(&Keycode::RAlt),
                                _ => false,
                            }
                    }
                    // Se NÃO precisa de modificador, verifica que NENHUM está pressionado
                    // Isso evita que Ctrl+T ative o atalho configurado pra só "T"
                    None => {
                        !keys.contains(&Keycode::LControl)
                            && !keys.contains(&Keycode::RControl)
                            && !keys.contains(&Keycode::LShift)
                            && !keys.contains(&Keycode::RShift)
                            && !keys.contains(&Keycode::LAlt)
                            && !keys.contains(&Keycode::RAlt)
                    }
                };

                if modifier_ok {
                    // Sistema de "press once" — só dispara na primeira detecção
                    if !self.pressed_keys.contains(&binding.key) {
                        self.pressed_keys.insert(binding.key);
                        self.last_action_time = Instant::now();
                        return Some(binding.action);
                    }
                    return None;
                }
            } else {
                // Tecla foi solta — remove do conjunto
                self.pressed_keys.remove(&binding.key);
            }
        }

        None
    }
}
