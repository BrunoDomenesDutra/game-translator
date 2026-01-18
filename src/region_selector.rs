// ============================================================================
// MÓDULO REGION SELECTOR - Seleção visual de região
// ============================================================================

use anyhow::Result;
use eframe::egui;
use screenshots::Screen;
use std::sync::{Arc, Mutex};

/// Coordenadas da região selecionada
#[derive(Debug, Clone)]
pub struct SelectedRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Aplicação de seleção de região
struct RegionSelectorApp {
    /// Screenshot de fundo
    background_texture: Option<egui::TextureHandle>,

    /// Posição inicial do clique (quando usuário começa a arrastar)
    start_pos: Option<egui::Pos2>,

    /// Posição atual do mouse
    current_pos: Option<egui::Pos2>,

    /// Região selecionada (quando usuário solta o mouse)
    selected_region: Option<SelectedRegion>,

    /// Se deve fechar a janela
    should_close: bool,

    result_holder: Option<Arc<Mutex<Option<SelectedRegion>>>>,
}

impl RegionSelectorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let background_texture = Self::capture_background(&cc.egui_ctx);

        RegionSelectorApp {
            background_texture,
            start_pos: None,
            current_pos: None,
            selected_region: None,
            should_close: false,
            result_holder: None, // <-- NOVO
        }
    }

    /// Captura a tela para usar como fundo
    fn capture_background(ctx: &egui::Context) -> Option<egui::TextureHandle> {
        info!("📸 Capturando tela de fundo para seleção...");

        // Captura a tela
        let screens = Screen::all().ok()?;
        let screen = screens.get(0)?;
        let buffer = screen.capture().ok()?;

        // Converte para imagem
        let width = buffer.width();
        let height = buffer.height();
        let rgba = buffer.rgba();

        // Cria textura para o egui
        let image =
            egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba);

        Some(ctx.load_texture("background", image, egui::TextureOptions::default()))
    }

    /// Calcula a região retangular sendo selecionada
    fn get_current_rect(&self) -> Option<egui::Rect> {
        let start = self.start_pos?;
        let current = self.current_pos?;

        Some(egui::Rect::from_two_pos(start, current))
    }
}

impl eframe::App for RegionSelectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ====================================================================
        // PAINEL CENTRAL - Seleção de região
        // ====================================================================
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                // ============================================================
                // DESENHA O FUNDO (Screenshot)
                // ============================================================
                if let Some(texture) = &self.background_texture {
                    let size = texture.size_vec2();
                    ui.image(texture);

                    // Overlay semitransparente escuro
                    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size);
                    ui.painter().rect_filled(
                        rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 100),
                    );
                }

                // ============================================================
                // INSTRUÇÕES
                // ============================================================
                ui.painter().text(
                    egui::pos2(20.0, 30.0),
                    egui::Align2::LEFT_TOP,
                    "🎯 SELEÇÃO DE REGIÃO",
                    egui::FontId::proportional(24.0),
                    egui::Color32::WHITE,
                );

                ui.painter().text(
                    egui::pos2(20.0, 60.0),
                    egui::Align2::LEFT_TOP,
                    "Clique e arraste para selecionar a área dos diálogos",
                    egui::FontId::proportional(16.0),
                    egui::Color32::LIGHT_GRAY,
                );

                ui.painter().text(
                    egui::pos2(20.0, 85.0),
                    egui::Align2::LEFT_TOP,
                    "Pressione ESC para cancelar",
                    egui::FontId::proportional(14.0),
                    egui::Color32::GRAY,
                );

                // ============================================================
                // DETECTA INTERAÇÃO DO MOUSE
                // ============================================================
                let response = ui.interact(
                    ui.max_rect(),
                    egui::Id::new("region_selector"),
                    egui::Sense::click_and_drag(),
                );

                // Mouse pressionado - inicia seleção
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.start_pos = Some(pos);
                        self.current_pos = Some(pos);
                        info!("🖱️  Início da seleção: ({:.0}, {:.0})", pos.x, pos.y);
                    }
                }

                // Mouse sendo arrastado - atualiza seleção
                if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.current_pos = Some(pos);
                    }
                }

                // Mouse solto - finaliza seleção
                if response.drag_stopped() {
                    if let Some(rect) = self.get_current_rect() {
                        let x = rect.min.x.min(rect.max.x) as u32;
                        let y = rect.min.y.min(rect.max.y) as u32;
                        let width = rect.width().abs() as u32;
                        let height = rect.height().abs() as u32;

                        info!(
                            "✅ Região selecionada: {}x{} na posição ({}, {})",
                            width, height, x, y
                        );

                        self.selected_region = Some(SelectedRegion {
                            x,
                            y,
                            width,
                            height,
                        });
                        self.should_close = true;
                    }
                }

                // ============================================================
                // DESENHA O RETÂNGULO DE SELEÇÃO
                // ============================================================
                if let Some(rect) = self.get_current_rect() {
                    // Borda do retângulo
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(3.0, egui::Color32::from_rgb(0, 200, 255)),
                    );

                    // Preenchimento semitransparente
                    ui.painter().rect_filled(
                        rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(0, 150, 255, 50),
                    );

                    // Mostra dimensões
                    let width = rect.width().abs() as u32;
                    let height = rect.height().abs() as u32;
                    let text = format!("{}x{}", width, height);

                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional(20.0),
                        egui::Color32::WHITE,
                    );
                }

                // ============================================================
                // TECLA ESC - Cancela
                // ============================================================
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    info!("❌ Seleção cancelada");
                    self.should_close = true;
                }
            });

        // ====================================================================
        // FECHA A JANELA SE NECESSÁRIO
        // ====================================================================
        if self.should_close {
            // Salva o resultado antes de fechar
            if let Some(holder) = &self.result_holder {
                *holder.lock().unwrap() = self.selected_region.clone();
            }

            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// Abre a interface de seleção de região e retorna a região selecionada
pub fn select_region() -> Result<Option<SelectedRegion>> {
    info!("🎯 Abrindo seletor de região...");

    // Captura dimensões da tela
    let screens = Screen::all()?;
    let screen = screens
        .get(0)
        .ok_or_else(|| anyhow::anyhow!("Nenhum monitor"))?;
    let width = screen.display_info.width as f32;
    let height = screen.display_info.height as f32;

    // Arc<Mutex> para compartilhar o resultado entre a app e esta função
    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width, height])
            .with_position([0.0, 0.0])
            .with_fullscreen(true)
            .with_always_on_top()
            .with_decorations(false)
            .with_resizable(false),

        ..Default::default()
    };

    let _ = eframe::run_native(
        "Seleção de Região",
        options,
        Box::new(move |cc| {
            let mut app = RegionSelectorApp::new(cc);
            app.result_holder = Some(result_clone);
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    );

    // Recupera o resultado após a janela fechar
    let final_result = result.lock().unwrap().clone();

    Ok(final_result)
}
