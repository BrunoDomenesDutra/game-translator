// game-translator/src/settings_ui/hotkeys_tab.rs

use crate::config;

pub(super) fn render_hotkeys_tab(ui: &mut eframe::egui::Ui, cfg: &mut config::AppConfig) {
    ui.heading("Teclas de Atalho");
    ui.add_space(10.0);

    // Lista de modificadores disponiveis
    let modificadores = vec!["", "Ctrl", "Shift", "Alt"];

    // Lista de teclas disponiveis
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
        // Teclas de funcao
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
        // Numeros
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
    super::full_width_group(ui, |ui| {
        ui.label("Tela Cheia:");
        ui.add_space(5.0);
        super::render_hotkey_combo(
            ui,
            "hotkey_fullscreen",
            "Capturar e traduzir:",
            &mut cfg.hotkeys.translate_fullscreen,
            &modificadores,
            &teclas_disponiveis,
        );
    });

    ui.add_space(10.0);

    // --- Captura em Area ---
    super::full_width_group(ui, |ui| {
        ui.label("Captura em Area:");
        ui.add_space(5.0);
        super::render_hotkey_combo(
            ui,
            "hotkey_select_region",
            "Selecionar area:",
            &mut cfg.hotkeys.select_region,
            &modificadores,
            &teclas_disponiveis,
        );
        super::render_hotkey_combo(
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
    super::full_width_group(ui, |ui| {
        ui.label("Modo Legenda:");
        ui.add_space(5.0);
        super::render_hotkey_combo(
            ui,
            "hotkey_select_subtitle",
            "Selecionar area:",
            &mut cfg.hotkeys.select_subtitle_region,
            &modificadores,
            &teclas_disponiveis,
        );
        super::render_hotkey_combo(
            ui,
            "hotkey_toggle_subtitle_areas_preview",
            "Mostrar areas:",
            &mut cfg.hotkeys.toggle_subtitle_areas_preview,
            &modificadores,
            &teclas_disponiveis,
        );
        super::render_hotkey_combo(
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
    super::full_width_group(ui, |ui| {
        ui.label("Outros:");
        ui.add_space(5.0);
        super::render_hotkey_combo(
            ui,
            "hotkey_hide",
            "Esconder traducao:",
            &mut cfg.hotkeys.hide_translation,
            &modificadores,
            &teclas_disponiveis,
        );
        super::render_hotkey_combo(
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
