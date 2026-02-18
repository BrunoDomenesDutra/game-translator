// game-translator/src/settings_ui/common.rs

use crate::config;
use crate::subtitle;

const DEBUG_PREVIEW_REFRESH_MS: u128 = 500;

/// Renderiza um grupo (caixa com borda) que sempre ocupa a largura total disponível.
/// Substitui ui.group() pra manter visual consistente em todas as abas.
pub(super) fn full_width_group(
    ui: &mut eframe::egui::Ui,
    add_contents: impl FnOnce(&mut eframe::egui::Ui),
) {
    let available_width = ui.available_width();

    // Frame com visual idêntico ao ui.group() mas com largura forçada
    // .max(0.0) evita panic quando a janela ainda não redimensionou
    eframe::egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width((available_width - 12.0).max(0.0));
        add_contents(ui);
    });
}

/// Renderiza controles de pré-processamento OCR usando Grid para alinhamento.
/// Reutilizado nas abas Display e Legendas.
pub(super) fn render_preprocess_controls(
    ui: &mut eframe::egui::Ui,
    preprocess: &mut config::PreprocessConfig,
    id_prefix: &str,
) {
    // Checkboxes ficam fora do grid (não precisam de alinhamento com sliders)
    ui.checkbox(&mut preprocess.grayscale, "Escala de cinza");
    ui.checkbox(&mut preprocess.invert, "Inverter cores");

    ui.add_space(5.0);

    // Grid alinha todos os labels na mesma coluna e sliders na segunda coluna
    eframe::egui::Grid::new(format!("{}_grid", id_prefix))
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            ui.label("Contraste:");
            ui.add(eframe::egui::Slider::new(&mut preprocess.contrast, 0.5..=10.0).suffix("x"));
            ui.end_row();

            ui.label("Threshold:");
            let mut threshold = preprocess.threshold as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut threshold, 0..=255))
                .changed()
            {
                preprocess.threshold = threshold as u8;
            }
            ui.end_row();

            ui.label("Upscale:");
            ui.add(eframe::egui::Slider::new(&mut preprocess.upscale, 1.0..=4.0).suffix("x"));
            ui.end_row();

            ui.label("Blur:");
            ui.add(eframe::egui::Slider::new(&mut preprocess.blur, 0.0..=5.0).suffix("x"));
            ui.end_row();

            ui.label("Dilatacao:");
            let mut d = preprocess.dilate as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut d, 0..=5).suffix("px"))
                .changed()
            {
                preprocess.dilate = d as u8;
            }
            ui.end_row();

            ui.label("Erosao:");
            let mut e_val = preprocess.erode as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut e_val, 0..=5).suffix("px"))
                .changed()
            {
                preprocess.erode = e_val as u8;
            }
            ui.end_row();

            ui.label("Edge Detection:");
            let mut ed = preprocess.edge_detection as i32;
            if ui
                .add(eframe::egui::Slider::new(&mut ed, 0..=150))
                .changed()
            {
                preprocess.edge_detection = ed as u8;
            }
            ui.end_row();
        });

    ui.add_space(3.0);
    ui.label("Edge Detection: 0=desativado, 30-80=recomendado (substitui threshold)");

    ui.add_space(5.0);
    ui.checkbox(&mut preprocess.save_debug_image, "Salvar imagem debug");
}

/// Renderiza um combo de hotkey com modificador + tecla.
/// Mostra dois ComboBox lado a lado: [Modificador] [Tecla]
pub(super) fn render_hotkey_combo(
    ui: &mut eframe::egui::Ui,
    id: &str,
    label: &str,
    binding: &mut config::HotkeyBinding,
    modifiers: &[&str],
    keys: &[&str],
) {
    ui.horizontal(|ui| {
        // Largura fixa pro label garante que todos os combos alinhem
        let label_response = ui.label(label);
        let used = label_response.rect.width();
        let min_label = 150.0;
        if used < min_label {
            ui.add_space(min_label - used);
        }

        // ComboBox do modificador
        let mod_display = if binding.modifier.is_empty() {
            "Nenhum"
        } else {
            &binding.modifier
        };

        eframe::egui::ComboBox::from_id_source(format!("{}_mod", id))
            .selected_text(mod_display)
            .width(70.0)
            .show_ui(ui, |ui| {
                for m in modifiers {
                    let display = if m.is_empty() { "Nenhum" } else { m };
                    ui.selectable_value(&mut binding.modifier, m.to_string(), display);
                }
            });

        ui.label("+");

        // ComboBox da tecla principal
        eframe::egui::ComboBox::from_id_source(format!("{}_key", id))
            .selected_text(&binding.key)
            .width(130.0)
            .show_ui(ui, |ui| {
                for k in keys {
                    ui.selectable_value(&mut binding.key, k.to_string(), *k);
                }
            });
    });
}

pub(super) fn render_debug_preview(
    ui: &mut eframe::egui::Ui,
    debug_texture: &mut Option<eframe::egui::TextureHandle>,
    last_update: &mut std::time::Instant,
    subtitle_state: &subtitle::SubtitleState,
) {
    full_width_group(ui, |ui| {
        ui.label("Preview do pre-processamento (auto-refresh):");
        ui.add_space(5.0);

        // Verifica se precisa atualizar a textura (a cada 500ms)
        let needs_update = last_update.elapsed().as_millis() >= DEBUG_PREVIEW_REFRESH_MS;

        if needs_update {
            // Tenta ler o arquivo de debug do disco
            let path = std::path::Path::new("debug_preprocessed.png");

            if path.exists() {
                match image::open(path) {
                    Ok(img) => {
                        // Converte a imagem pra RGBA8
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();

                        // Cria ColorImage pro egui
                        let color_image =
                            eframe::egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

                        // Cria ou atualiza a textura
                        match debug_texture {
                            Some(ref mut tex) => {
                                // Atualiza textura existente
                                tex.set(color_image, eframe::egui::TextureOptions::LINEAR);
                            }
                            None => {
                                // Cria nova textura
                                *debug_texture = Some(ui.ctx().load_texture(
                                    "debug_preview",
                                    color_image,
                                    eframe::egui::TextureOptions::LINEAR,
                                ));
                            }
                        }

                        *last_update = std::time::Instant::now();
                    }
                    Err(e) => {
                        ui.label(format!("Erro ao ler imagem: {}", e));
                    }
                }
            } else {
                ui.label("Arquivo debug_preprocessed.png nao encontrado.");
                ui.label("Faca uma captura primeiro para gerar a imagem.");
            }
        }

        // Renderiza a textura se existir
        if let Some(ref texture) = debug_texture {
            let tex_size = texture.size_vec2();

            // Escala pra caber na largura disponível, mantendo proporção
            let available_w = ui.available_width();
            let scale = (available_w / tex_size.x).min(1.0); // Não amplia, só reduz
            let display_size = eframe::egui::vec2(tex_size.x * scale, tex_size.y * scale);

            ui.image(eframe::egui::load::SizedTexture::new(
                texture.id(),
                display_size,
            ));

            // Info sobre a imagem
            ui.add_space(3.0);
            ui.label(format!(
                "{}x{} pixels (atualiza a cada {}ms)",
                tex_size.x as u32, tex_size.y as u32, DEBUG_PREVIEW_REFRESH_MS,
            ));
        }

        // --- Última tradução ---
        let history = subtitle_state.get_full_history();
        if let Some(last) = history.last() {
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(3.0);
            ui.label("Ultima traducao:");
            ui.label(
                eframe::egui::RichText::new(&last.translated)
                    .size(16.0)
                    .color(eframe::egui::Color32::from_rgb(100, 200, 255)),
            );
        }

        // Força repaint pra manter o auto-refresh funcionando
        ui.ctx().request_repaint();
    });
}
