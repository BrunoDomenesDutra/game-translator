// game-translator/src/overlay/fonts.rs

// ============================================================================
// FONTS DO OVERLAY
// ============================================================================

use crate::app_state::AppState;

/// Recarrega a fonte de traducao quando sinalizado em `font_need_reload`.
pub fn reload_translation_font_if_needed(ctx: &eframe::egui::Context, state: &AppState) {
    let mut needs_reload = state.font_need_reload.lock().unwrap();
    if !*needs_reload {
        return;
    }
    *needs_reload = false;

    let translation_font_name = {
        let config = state.config.lock().unwrap();
        config.app_config.font.translation_font.clone()
    };

    let translation_font_path = std::path::Path::new("fonts").join(&translation_font_name);

    let mut fonts = eframe::egui::FontDefinitions::default();

    // Roboto embutida (UI)
    let roboto_data = include_bytes!("../../fonts/Roboto-Regular.ttf");
    fonts.font_data.insert(
        "roboto".to_owned(),
        eframe::egui::FontData::from_static(roboto_data),
    );
    fonts
        .families
        .get_mut(&eframe::egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "roboto".to_owned());

    // Fonte de traducao
    let translation_family = eframe::egui::FontFamily::Name("translation".into());

    if !translation_font_name.is_empty() && translation_font_path.exists() {
        match std::fs::read(&translation_font_path) {
            Ok(font_data) => {
                fonts.font_data.insert(
                    "translation".to_owned(),
                    eframe::egui::FontData::from_owned(font_data),
                );
                fonts.families.insert(
                    translation_family,
                    vec!["translation".to_owned(), "roboto".to_owned()],
                );
                info!(
                    "🔤 Fonte de tradução recarregada: {}",
                    translation_font_name
                );
            }
            Err(e) => {
                error!("❌ Erro ao recarregar fonte: {}", e);
                fonts
                    .families
                    .insert(translation_family, vec!["roboto".to_owned()]);
            }
        }
    } else {
        fonts
            .families
            .insert(translation_family, vec!["roboto".to_owned()]);
        info!("🔤 Fonte de tradução: Roboto (fallback)");
    }

    ctx.set_fonts(fonts);
}
