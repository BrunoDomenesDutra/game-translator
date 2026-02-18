// game-translator/src/settings_ui/lab_tab.rs

use crate::config;
use crate::screenshot;

/// Aba de teste de pre-processamento.
/// Carrega uma imagem da pasta images/ e aplica os filtros em tempo real.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_lab_tab(
    ui: &mut eframe::egui::Ui,
    cfg: &mut config::AppConfig,
    original_texture: &mut Option<eframe::egui::TextureHandle>,
    processed_texture: &mut Option<eframe::egui::TextureHandle>,
    preprocess: &mut Option<config::PreprocessConfig>,
    selected_file: &mut Option<String>,
    original_image: &mut Option<image::DynamicImage>,
    needs_reprocess: &mut bool,
) {
    ui.heading("Laboratorio de Pre-processamento");
    ui.add_space(10.0);

    // Inicializa config de pre-processamento se ainda nao existe
    if preprocess.is_none() {
        *preprocess = Some(config::PreprocessConfig::default());
    }

    // --- Selecao de imagem ---
    super::full_width_group(ui, |ui| {
        ui.label("Imagem de teste:");
        ui.add_space(5.0);
        ui.label("Coloque imagens PNG/JPG na pasta 'images/' ao lado do executavel.");
        ui.add_space(5.0);

        // Lista arquivos da pasta images/
        let images_dir = std::path::Path::new("images");
        let mut files: Vec<String> = Vec::new();

        if images_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(images_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        let ext_lower = ext.to_string_lossy().to_lowercase();
                        if ext_lower == "png" || ext_lower == "jpg" || ext_lower == "jpeg" {
                            if let Some(name) = path.file_name() {
                                files.push(name.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        files.sort();

        if files.is_empty() {
            ui.label("Nenhuma imagem encontrada na pasta 'images/'.");
            ui.label("Crie a pasta e coloque screenshots de legendas para testar.");
        } else {
            // Combo box pra selecionar o arquivo
            let current = selected_file.clone().unwrap_or_default();

            eframe::egui::ComboBox::from_id_source("lab_image_selector")
                .selected_text(if current.is_empty() {
                    "Selecione uma imagem..."
                } else {
                    &current
                })
                .show_ui(ui, |ui| {
                    for file in &files {
                        if ui
                            .selectable_value(selected_file, Some(file.clone()), file)
                            .clicked()
                        {
                            // Quando seleciona um arquivo novo, carrega a imagem
                            let path = images_dir.join(file);
                            match image::open(&path) {
                                Ok(img) => {
                                    // Cria textura da imagem original
                                    let rgba = img.to_rgba8();
                                    let size = [rgba.width() as usize, rgba.height() as usize];
                                    let pixels = rgba.into_raw();
                                    let color_image =
                                        eframe::egui::ColorImage::from_rgba_unmultiplied(
                                            size, &pixels,
                                        );

                                    *original_texture = Some(ui.ctx().load_texture(
                                        "lab_original",
                                        color_image,
                                        eframe::egui::TextureOptions::LINEAR,
                                    ));

                                    *original_image = Some(img);
                                    *needs_reprocess = true;

                                    info!("🔬 Imagem carregada: {}", file);
                                }
                                Err(e) => {
                                    error!("❌ Erro ao carregar {}: {}", file, e);
                                }
                            }
                        }
                    }
                });
        }
    });

    // Se nao tem imagem carregada, para aqui
    if original_image.is_none() {
        return;
    }

    ui.add_space(10.0);

    // --- Controles de pre-processamento ---
    let mut changed = false;

    super::full_width_group(ui, |ui| {
        ui.label("Parametros de pre-processamento:");
        ui.add_space(5.0);

        if let Some(ref mut pp) = preprocess {
            // Checkboxes
            if ui.checkbox(&mut pp.grayscale, "Escala de cinza").changed() {
                changed = true;
            }
            if ui.checkbox(&mut pp.invert, "Inverter cores").changed() {
                changed = true;
            }

            ui.add_space(5.0);

            // Grid com sliders
            eframe::egui::Grid::new("lab_preprocess_grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Contraste:");
                    if ui
                        .add(eframe::egui::Slider::new(&mut pp.contrast, 0.5..=10.0).suffix("x"))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Threshold:");
                    let mut threshold = pp.threshold as i32;
                    if ui
                        .add(eframe::egui::Slider::new(&mut threshold, 0..=255))
                        .changed()
                    {
                        pp.threshold = threshold as u8;
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Upscale:");
                    if ui
                        .add(eframe::egui::Slider::new(&mut pp.upscale, 1.0..=4.0).suffix("x"))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Blur:");
                    if ui
                        .add(eframe::egui::Slider::new(&mut pp.blur, 0.0..=5.0).suffix("x"))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Dilatacao:");
                    let mut d = pp.dilate as i32;
                    if ui
                        .add(eframe::egui::Slider::new(&mut d, 0..=5).suffix("px"))
                        .changed()
                    {
                        pp.dilate = d as u8;
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Erosao:");
                    let mut e_val = pp.erode as i32;
                    if ui
                        .add(eframe::egui::Slider::new(&mut e_val, 0..=5).suffix("px"))
                        .changed()
                    {
                        pp.erode = e_val as u8;
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Edge Detection:");
                    let mut ed = pp.edge_detection as i32;
                    if ui
                        .add(eframe::egui::Slider::new(&mut ed, 0..=150))
                        .changed()
                    {
                        pp.edge_detection = ed as u8;
                        changed = true;
                    }
                    ui.end_row();
                });

            ui.add_space(3.0);
            ui.label("Edge Detection: 0=desativado, 30-80=recomendado (substitui threshold)");
        }
    });

    // Se algum parametro mudou, reprocessa a imagem
    if changed {
        *needs_reprocess = true;
    }

    // --- Botoes para copiar parametros ---
    if preprocess.is_some() {
        ui.add_space(10.0);
        super::full_width_group(ui, |ui| {
            ui.label("Copiar parametros para:");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("Aplicar em Display").clicked() {
                    if let Some(ref pp) = preprocess {
                        cfg.display.preprocess = pp.clone();
                        cfg.display.preprocess.enabled = true;
                        info!("🔬 Parametros copiados para Display");
                    }
                }

                if ui.button("Aplicar em Legendas").clicked() {
                    if let Some(ref pp) = preprocess {
                        cfg.subtitle.preprocess = pp.clone();
                        cfg.subtitle.preprocess.enabled = true;
                        info!("🔬 Parametros copiados para Legendas");
                    }
                }
            });

            ui.add_space(3.0);
            ui.label("Lembre de salvar as configuracoes depois!");
        });
    }

    // Reprocessa se necessario
    if *needs_reprocess {
        if let (Some(ref img), Some(ref pp)) = (original_image, preprocess) {
            // Aplica pre-processamento usando a mesma funcao do pipeline real
            let processed = screenshot::preprocess_image(
                img,
                pp.grayscale,
                pp.invert,
                pp.contrast,
                pp.threshold,
                false, // nao salva debug
                pp.upscale,
                pp.blur,
                pp.dilate,
                pp.erode,
                pp.edge_detection,
            );

            // Converte pra textura do egui
            let rgba = processed.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels = rgba.into_raw();
            let color_image = eframe::egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

            match processed_texture {
                Some(ref mut tex) => {
                    tex.set(color_image, eframe::egui::TextureOptions::LINEAR);
                }
                None => {
                    *processed_texture = Some(ui.ctx().load_texture(
                        "lab_processed",
                        color_image,
                        eframe::egui::TextureOptions::LINEAR,
                    ));
                }
            }

            *needs_reprocess = false;
        }
    }

    ui.add_space(10.0);

    // --- Imagens: Original e Processada ---
    let available_w = ui.available_width();

    // Imagem original
    super::full_width_group(ui, |ui| {
        ui.label("Imagem original:");
        ui.add_space(3.0);

        if let Some(ref texture) = original_texture {
            let tex_size = texture.size_vec2();
            let scale = ((available_w - 20.0) / tex_size.x).min(1.0);
            let display_size = eframe::egui::vec2(tex_size.x * scale, tex_size.y * scale);

            ui.image(eframe::egui::load::SizedTexture::new(
                texture.id(),
                display_size,
            ));

            ui.add_space(3.0);
            ui.label(format!(
                "{}x{} pixels",
                tex_size.x as u32, tex_size.y as u32
            ));
        }
    });

    ui.add_space(10.0);

    // Imagem processada
    super::full_width_group(ui, |ui| {
        ui.label("Imagem processada:");
        ui.add_space(3.0);

        if let Some(ref texture) = processed_texture {
            let tex_size = texture.size_vec2();
            let scale = ((available_w - 20.0) / tex_size.x).min(1.0);
            let display_size = eframe::egui::vec2(tex_size.x * scale, tex_size.y * scale);

            ui.image(eframe::egui::load::SizedTexture::new(
                texture.id(),
                display_size,
            ));

            ui.add_space(3.0);
            ui.label(format!(
                "{}x{} pixels",
                tex_size.x as u32, tex_size.y as u32
            ));
        }
    });
}
