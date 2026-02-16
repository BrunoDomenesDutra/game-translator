// game-translator/src/overlay/render.rs

// ============================================================================
// RENDERIZACAO DE ELEMENTOS DO OVERLAY
// ============================================================================

use crate::app_state::{AppState, CaptureMode};
use std::time::Duration;

/// Estima a area onde o overlay de legendas sera exibido.
/// Retorna (x, y, width, height) em coordenadas logicas (ja com DPI aplicado).
fn estimate_subtitle_overlay_area(
    subtitle_region_y: f32,
    max_lines: usize,
    font_size: f32,
    dpi_scale: f32,
) -> (f32, f32, f32, f32) {
    let screen_width_px =
        unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN) } as f32;

    // Mantem a mesma margem lateral usada no modo legenda
    let side_margin_px = 50.0;
    let overlay_width = (screen_width_px - side_margin_px * 2.0) / dpi_scale;
    let overlay_x = side_margin_px / dpi_scale;

    // Estimativa de altura baseada em numero de linhas + fonte
    // (ajuda a visualizar mesmo sem legendas ativas)
    let estimated_line_height = (font_size * 1.35).max(18.0);
    let estimated_height = (max_lines.max(1) as f32 * estimated_line_height + 20.0).max(50.0);

    let overlay_y = (subtitle_region_y / dpi_scale - estimated_height - 10.0).max(0.0);

    (overlay_x, overlay_y, overlay_width, estimated_height)
}

/// Cria um FontId usando a familia "translation" (fonte configurada pelo usuario).
fn translation_font_id(size: f32) -> eframe::egui::FontId {
    eframe::egui::FontId::new(size, eframe::egui::FontFamily::Name("translation".into()))
}

/// Renderiza o preview de areas de legenda (captura + exibicao).
/// Retorna true quando o preview foi renderizado e o frame deve encerrar.
pub fn render_subtitle_areas_preview(
    ctx: &eframe::egui::Context,
    state: &AppState,
) -> bool {
    let subtitle_areas_preview_active = *state.subtitle_areas_preview_active.lock().unwrap();
    if !subtitle_areas_preview_active {
        return false;
    }

    let (subtitle_region, subtitle_font_size, subtitle_max_lines) = {
        let config = state.config.lock().unwrap();
        (
            config.app_config.subtitle.region.clone(),
            config.app_config.subtitle.font.size,
            config.app_config.subtitle.max_lines,
        )
    };

    let scale = state.dpi_scale;
    let screen_width =
        unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN) } as f32;
    let screen_height =
        unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN) } as f32;

    let capture_rect_screen = eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(
            subtitle_region.x as f32 / scale,
            subtitle_region.y as f32 / scale,
        ),
        eframe::egui::vec2(
            subtitle_region.width as f32 / scale,
            subtitle_region.height as f32 / scale,
        ),
    );

    let (overlay_x, overlay_y, overlay_width, overlay_height) = estimate_subtitle_overlay_area(
        subtitle_region.y as f32,
        subtitle_max_lines,
        subtitle_font_size,
        scale,
    );

    let display_rect_screen = eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(overlay_x, overlay_y),
        eframe::egui::vec2(overlay_width, overlay_height),
    );

    // Janela cobre apenas a uniao das duas areas (evita ocupar tela inteira)
    let preview_padding = 40.0;
    let screen_w_logical = screen_width / scale;
    let screen_h_logical = screen_height / scale;

    let mut window_x = (capture_rect_screen.min.x.min(display_rect_screen.min.x) - preview_padding)
        .max(0.0);
    let mut window_y = (capture_rect_screen.min.y.min(display_rect_screen.min.y) - preview_padding)
        .max(0.0);
    let mut window_right = (capture_rect_screen.max.x.max(display_rect_screen.max.x)
        + preview_padding)
        .min(screen_w_logical);
    let mut window_bottom = (capture_rect_screen.max.y.max(display_rect_screen.max.y)
        + preview_padding)
        .min(screen_h_logical);

    // Garante tamanho minimo para evitar viewport invalida
    if window_right <= window_x {
        window_right = (window_x + 1.0).min(screen_w_logical);
    }
    if window_bottom <= window_y {
        window_bottom = (window_y + 1.0).min(screen_h_logical);
    }

    // Ajusta quando o clamp no limite da tela deixou origem fora do intervalo
    if window_x >= window_right {
        window_x = (window_right - 1.0).max(0.0);
    }
    if window_y >= window_bottom {
        window_y = (window_bottom - 1.0).max(0.0);
    }

    let window_width = window_right - window_x;
    let window_height = window_bottom - window_y;

    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(eframe::egui::pos2(
        window_x, window_y,
    )));
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(eframe::egui::vec2(
        window_width,
        window_height,
    )));

    let capture_rect = eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(
            capture_rect_screen.min.x - window_x,
            capture_rect_screen.min.y - window_y,
        ),
        capture_rect_screen.size(),
    );
    let display_rect = eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(
            display_rect_screen.min.x - window_x,
            display_rect_screen.min.y - window_y,
        ),
        display_rect_screen.size(),
    );

    eframe::egui::CentralPanel::default()
        .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
        .show(ctx, |ui| {
            let stroke_capture = eframe::egui::Stroke::new(
                3.0,
                eframe::egui::Color32::from_rgb(255, 92, 92),
            );
            let stroke_display = eframe::egui::Stroke::new(
                3.0,
                eframe::egui::Color32::from_rgb(80, 200, 255),
            );

            ui.painter().rect_stroke(capture_rect, 0.0, stroke_capture);
            ui.painter().rect_stroke(display_rect, 0.0, stroke_display);

            ui.painter().text(
                capture_rect.left_top() + eframe::egui::vec2(8.0, 8.0),
                eframe::egui::Align2::LEFT_TOP,
                "CAPTURA DE LEGENDA",
                eframe::egui::FontId::proportional(18.0),
                stroke_capture.color,
            );

            ui.painter().text(
                display_rect.left_top() + eframe::egui::vec2(8.0, 8.0),
                eframe::egui::Align2::LEFT_TOP,
                "ÁREA DE EXIBIÇÃO DA TRADUÇÃO",
                eframe::egui::FontId::proportional(18.0),
                stroke_display.color,
            );
        });

    ctx.request_repaint();
    true
}

/// Renderiza o historico de legendas acima da regiao de captura.
/// Retorna true quando o modo legenda foi renderizado neste frame.
pub fn render_subtitle_history_overlay(
    ctx: &eframe::egui::Context,
    state: &AppState,
) -> bool {
    let subtitle_mode_active = *state.subtitle_mode_active.lock().unwrap();
    let has_subtitles = state.subtitle_state.has_subtitles();

    if !(subtitle_mode_active && has_subtitles) {
        return false;
    }

    // Pega a regiao de legenda do config
    let (_sub_x, sub_y, _sub_w, _sub_h) = {
        let config = state.config.lock().unwrap();
        (
            config.app_config.subtitle.region.x as f32,
            config.app_config.subtitle.region.y as f32,
            config.app_config.subtitle.region.width as f32,
            config.app_config.subtitle.region.height as f32,
        )
    };

    // Pega configuracoes de fonte (especifica de legendas) e fundo
    let (
        font_size,
        font_color,
        show_background,
        bg_color,
        outline_enabled,
        outline_width,
        outline_color,
    ) = {
        let config = state.config.lock().unwrap();
        (
            config.app_config.subtitle.font.size,
            config.app_config.subtitle.font.color,
            config.app_config.overlay.show_background,
            config.app_config.overlay.background_color,
            config.app_config.subtitle.font.outline.enabled,
            config.app_config.subtitle.font.outline.width,
            config.app_config.subtitle.font.outline.color,
        )
    };

    // Pega o historico de legendas
    let history = state.subtitle_state.get_subtitle_history();

    // Pega numero maximo de legendas do config
    let max_lines = {
        let config = state.config.lock().unwrap();
        config.app_config.subtitle.max_lines
    };

    // Pega apenas as ultimas N legendas
    let visible_history: Vec<_> = if history.len() > max_lines {
        history[(history.len() - max_lines)..].to_vec()
    } else {
        history.clone()
    };

    // Calcula altura dinamica baseada no conteudo real
    let font_id_calc = translation_font_id(font_size);
    let screen_width_calc =
        unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN) } as f32;
    let max_width_calc = (screen_width_calc - 100.0) / state.dpi_scale - 100.0;

    let mut calculated_height = 10.0; // Margem superior
    for entry in &visible_history {
        let text = format!("- {}", entry.translated);
        let galley = ctx.fonts(|f| {
            f.layout(
                text,
                font_id_calc.clone(),
                eframe::egui::Color32::WHITE,
                max_width_calc,
            )
        });
        calculated_height += galley.rect.height() + 5.0;
    }
    calculated_height += 10.0; // Margem inferior

    let overlay_height = calculated_height.max(50.0); // Minimo de 50px

    // Posiciona o overlay ACIMA da regiao de legenda
    // Usa largura TOTAL da tela para a caixa de traducao
    let scale = state.dpi_scale;
    let screen_width =
        unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN) } as f32;
    // Margem lateral (pixels fisicos) - ajuste se quiser mais/menos borda
    let side_margin = 50.0;
    let overlay_width = (screen_width - side_margin * 2.0) / scale;
    let overlay_x = side_margin / scale;
    // overlay_height ja esta em logico (calculado pelo galley)
    let overlay_y = sub_y / scale - overlay_height - 10.0;

    // Posiciona e redimensiona a janela
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
        eframe::egui::pos2(overlay_x, overlay_y),
    ));
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(eframe::egui::vec2(
        overlay_width,
        overlay_height,
    )));

    // Renderiza o historico de legendas
    eframe::egui::CentralPanel::default()
        .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
        .show(ctx, |ui| {
            // Se show_background = true, desenha o fundo
            if show_background {
                let rect = ui.max_rect();
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    eframe::egui::Color32::from_rgba_unmultiplied(
                        bg_color[0], bg_color[1], bg_color[2], bg_color[3],
                    ),
                );
            }

            // Configura renderizacao
            let font_id = translation_font_id(font_size);
            let max_width = overlay_width - 100.0; // Mais margem lateral pro texto

            let text_color = eframe::egui::Color32::from_rgba_unmultiplied(
                font_color[0],
                font_color[1],
                font_color[2],
                font_color[3],
            );

            // Renderiza cada legenda do historico
            let mut y_offset = 5.0;

            for entry in &visible_history {
                let text = format!("- {}", entry.translated);

                // Centraliza horizontalmente: calcula largura do texto
                // e posiciona no centro do overlay
                let galley_measure = ui
                    .painter()
                    .layout(text.clone(), font_id.clone(), text_color, max_width);
                let text_w = galley_measure.rect.width();
                let center_x = (overlay_width - text_w) / 2.0;
                let text_pos = eframe::egui::pos2(center_x.max(10.0), y_offset);

                // Calcula o galley para obter a altura real
                let galley = ui
                    .painter()
                    .layout(text.clone(), font_id.clone(), text_color, max_width);
                let text_height = galley.rect.height();

                // Desenha contorno se habilitado OU se nao tem fundo
                if outline_enabled || !show_background {
                    let size = outline_width as f32;
                    let color = eframe::egui::Color32::from_rgba_unmultiplied(
                        outline_color[0],
                        outline_color[1],
                        outline_color[2],
                        outline_color[3],
                    );

                    // Gera pontos em circulo para contorno suave
                    // Quanto maior o size, mais pontos precisamos
                    let num_points = (size * 8.0).max(16.0) as i32;

                    for layer in 1..=(size.ceil() as i32) {
                        let radius = layer as f32;

                        for i in 0..num_points {
                            let angle =
                                (i as f32 / num_points as f32) * std::f32::consts::PI * 2.0;
                            let dx = angle.cos() * radius;
                            let dy = angle.sin() * radius;

                            let offset_pos = text_pos + eframe::egui::vec2(dx, dy);
                            let outline_galley = ui.painter().layout(
                                text.clone(),
                                font_id.clone(),
                                color,
                                max_width,
                            );
                            ui.painter().galley(offset_pos, outline_galley, color);
                        }
                    }
                }

                // Desenha o texto principal
                ui.painter().galley(text_pos, galley, text_color);

                // Avanca Y pela altura real do texto + espacamento
                y_offset += text_height + 5.0;
            }
        });

    true
}

/// Renderiza traducoes no modo fullscreen/regiao quando ainda estao dentro da duracao configurada.
/// Retorna true quando o frame foi tratado como "com traducao ativa".
pub fn render_translations_overlay(
    ctx: &eframe::egui::Context,
    state: &AppState,
    display_duration: Duration,
) -> bool {
    let should_display = if let Some((_, _, _, timestamp)) = state.get_translations() {
        timestamp.elapsed() < display_duration
    } else {
        false
    };

    if !should_display {
        return false;
    }

    if let Some((items, region, mode, _timestamp)) = state.get_translations() {
        // Pega tamanho da fonte do config
        let font_size = state.config.lock().unwrap().app_config.font.size;

        // Pega configuracao de fundo e outline
        let (show_background, bg_color, outline_enabled, outline_width, outline_color) = {
            let config = state.config.lock().unwrap();
            (
                config.app_config.overlay.show_background,
                config.app_config.overlay.background_color,
                config.app_config.font.outline.enabled,
                config.app_config.font.outline.width,
                config.app_config.font.outline.color,
            )
        };

        // Usa o modo de captura para decidir como renderizar
        let is_fullscreen_mode = mode == CaptureMode::FullScreen;

        if is_fullscreen_mode {
            // Calcula bounding box de todos os textos
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = 0.0f64;
            let mut max_y = 0.0f64;

            for item in &items {
                if item.translated.is_empty() || item.original == item.translated {
                    continue;
                }
                min_x = min_x.min(item.screen_x);
                min_y = min_y.min(item.screen_y);
                max_x = max_x.max(item.screen_x + item.width);
                max_y = max_y.max(item.screen_y + item.height);
            }

            // Se nao ha textos validos, esconde
            if min_x == f64::MAX {
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(eframe::egui::vec2(
                    1.0, 1.0,
                )));
            } else {
                // Adiciona margem
                let margin = 20.0;
                let scale = state.dpi_scale;
                let overlay_x = (min_x - margin).max(0.0) as f32 / scale;
                let overlay_y = (min_y - margin).max(0.0) as f32 / scale;
                let overlay_width = (max_x - min_x + margin * 2.0) as f32 / scale;
                let overlay_height = (max_y - min_y + margin * 2.0 + 50.0) as f32 / scale;

                // Posiciona e redimensiona a janela
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                    eframe::egui::pos2(overlay_x, overlay_y),
                ));
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                    eframe::egui::vec2(overlay_width, overlay_height),
                ));

                eframe::egui::CentralPanel::default()
                    .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
                    .show(ctx, |ui| {
                        let font_id = translation_font_id(font_size);

                        for item in &items {
                            if item.translated.is_empty() || item.original == item.translated {
                                continue;
                            }

                            // Posicao relativa ao overlay (ajustada por DPI)
                            let text_x = (item.screen_x - min_x + margin) as f32 / scale;
                            let text_y = (item.screen_y - min_y + margin) as f32 / scale;
                            let text_pos = eframe::egui::pos2(text_x, text_y);

                            // Largura maxima baseada na largura original do texto
                            let max_width = (item.width as f32 / scale * 1.5).max(200.0);

                            if show_background {
                                let galley = ui.painter().layout(
                                    item.translated.clone(),
                                    font_id.clone(),
                                    eframe::egui::Color32::WHITE,
                                    max_width,
                                );
                                let text_rect = eframe::egui::Rect::from_min_size(
                                    text_pos,
                                    galley.rect.size() + eframe::egui::vec2(10.0, 6.0),
                                );
                                ui.painter().rect_filled(
                                    text_rect,
                                    4.0,
                                    eframe::egui::Color32::from_rgba_unmultiplied(
                                        bg_color[0],
                                        bg_color[1],
                                        bg_color[2],
                                        bg_color[3],
                                    ),
                                );
                            }

                            if outline_enabled || !show_background {
                                let size = outline_width as f32;
                                let color = eframe::egui::Color32::from_rgba_unmultiplied(
                                    outline_color[0],
                                    outline_color[1],
                                    outline_color[2],
                                    outline_color[3],
                                );

                                let num_points = (size * 8.0).max(16.0) as i32;

                                for layer in 1..=(size.ceil() as i32) {
                                    let radius = layer as f32;

                                    for i in 0..num_points {
                                        let angle = (i as f32 / num_points as f32)
                                            * std::f32::consts::PI
                                            * 2.0;
                                        let dx = angle.cos() * radius;
                                        let dy = angle.sin() * radius;

                                        let offset_pos = text_pos + eframe::egui::vec2(dx, dy);
                                        let outline_galley = ui.painter().layout(
                                            item.translated.clone(),
                                            font_id.clone(),
                                            color,
                                            max_width,
                                        );
                                        ui.painter().galley(offset_pos, outline_galley, color);
                                    }
                                }
                            }

                            let galley = ui.painter().layout(
                                item.translated.clone(),
                                font_id.clone(),
                                eframe::egui::Color32::WHITE,
                                max_width,
                            );
                            ui.painter()
                                .galley(text_pos, galley, eframe::egui::Color32::WHITE);
                        }
                    });
            }
        } else {
            let scale = state.dpi_scale;
            let overlay_x = region.x as f32 / scale;
            let overlay_y = region.y as f32 / scale;
            let overlay_width = region.width as f32 / scale;
            let overlay_height = region.height as f32 / scale;

            // Posiciona e redimensiona a janela
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                eframe::egui::pos2(overlay_x, overlay_y),
            ));
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(eframe::egui::vec2(
                overlay_width,
                overlay_height,
            )));

            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
                .show(ctx, |ui| {
                    // Junta todas as traducoes em um texto so
                    let combined_text: String = items
                        .iter()
                        .filter(|item| item.original != item.translated)
                        .map(|item| item.translated.as_str())
                        .collect::<Vec<&str>>()
                        .join(" ");

                    if !combined_text.is_empty() {
                        if show_background {
                            let rect = ui.max_rect();
                            ui.painter().rect_filled(
                                rect,
                                0.0,
                                eframe::egui::Color32::from_rgba_unmultiplied(
                                    bg_color[0],
                                    bg_color[1],
                                    bg_color[2],
                                    bg_color[3],
                                ),
                            );
                        }

                        let text_pos = eframe::egui::pos2(20.0, 15.0);
                        let font_id = translation_font_id(font_size);
                        let max_width = overlay_width - 40.0;

                        if outline_enabled || !show_background {
                            let size = outline_width as f32;
                            let color = eframe::egui::Color32::from_rgba_unmultiplied(
                                outline_color[0],
                                outline_color[1],
                                outline_color[2],
                                outline_color[3],
                            );

                            let num_points = (size * 8.0).max(16.0) as i32;

                            for layer in 1..=(size.ceil() as i32) {
                                let radius = layer as f32;

                                for i in 0..num_points {
                                    let angle = (i as f32 / num_points as f32)
                                        * std::f32::consts::PI
                                        * 2.0;
                                    let dx = angle.cos() * radius;
                                    let dy = angle.sin() * radius;

                                    let offset_pos = text_pos + eframe::egui::vec2(dx, dy);
                                    let outline_galley = ui.painter().layout(
                                        combined_text.clone(),
                                        font_id.clone(),
                                        color,
                                        max_width,
                                    );
                                    ui.painter().galley(offset_pos, outline_galley, color);
                                }
                            }
                        }

                        let galley = ui.painter().layout(
                            combined_text.clone(),
                            font_id.clone(),
                            eframe::egui::Color32::WHITE,
                            max_width,
                        );
                        ui.painter()
                            .galley(text_pos, galley, eframe::egui::Color32::WHITE);
                    }
                });
        }
    }

    true
}
