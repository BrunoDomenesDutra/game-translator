// ============================================================================
// MÓDULO REGION SELECTOR - Seleção visual usando minifb
// ============================================================================

use anyhow::Result;
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
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

/// Estado do mouse durante a seleção
#[derive(Debug, Clone, Copy)]
struct MouseState {
    start_x: Option<i32>,
    start_y: Option<i32>,
    current_x: i32,
    current_y: i32,
    is_dragging: bool,
}

impl MouseState {
    fn new() -> Self {
        MouseState {
            start_x: None,
            start_y: None,
            current_x: 0,
            current_y: 0,
            is_dragging: false,
        }
    }
}

/// Abre a interface de seleção de região e retorna a região selecionada
pub fn select_region() -> Result<Option<SelectedRegion>> {
    info!("🎯 Iniciando seletor de região...");

    // Captura screenshot da tela
    let screens = Screen::all()?;
    let screen = screens
        .first()
        .ok_or_else(|| anyhow::anyhow!("Nenhum monitor encontrado"))?;
    let image = screen.capture()?;

    let width = image.width() as usize;
    let height = image.height() as usize;

    info!("📸 Screenshot capturado: {}x{}", width, height);

    // Converte para buffer do minifb (ARGB)
    let mut buffer: Vec<u32> = image
        .rgba()
        .chunks(4)
        .map(|rgba| {
            let r = rgba[0] as u32;
            let g = rgba[1] as u32;
            let b = rgba[2] as u32;
            // Escurece a imagem (overlay semitransparente)
            let r = (r * 6 / 10) & 0xFF;
            let g = (g * 6 / 10) & 0xFF;
            let b = (b * 6 / 10) & 0xFF;
            (r << 16) | (g << 8) | b
        })
        .collect();

    // Cria janela fullscreen borderless
    let mut window = Window::new(
        "Seleção de Região - Clique e Arraste | ESC para Cancelar",
        width,
        height,
        WindowOptions {
            borderless: true,
            title: false,
            resize: false,
            scale_mode: minifb::ScaleMode::UpperLeft,
            topmost: true,
            none: false,
            ..WindowOptions::default()
        },
    )?;

    window.set_position(0, 0);

    let mut mouse_state = MouseState::new();
    let result = Arc::new(Mutex::new(None));

    info!("✅ Janela de seleção aberta. Aguardando interação...");

    // Loop principal
    while window.is_open() {
        // ESC para cancelar
        if window.is_key_down(Key::Escape) {
            info!("❌ Seleção cancelada pelo usuário");
            break;
        }

        // Pega posição do mouse
        if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Clamp) {
            mouse_state.current_x = mx as i32;
            mouse_state.current_y = my as i32;

            // Botão pressionado - inicia seleção
            if window.get_mouse_down(MouseButton::Left) {
                if !mouse_state.is_dragging {
                    mouse_state.start_x = Some(mx as i32);
                    mouse_state.start_y = Some(my as i32);
                    mouse_state.is_dragging = true;
                    info!("🖱️  Início da seleção: ({}, {})", mx as i32, my as i32);
                }
            } else if mouse_state.is_dragging {
                // Botão solto - finaliza seleção
                if let (Some(start_x), Some(start_y)) = (mouse_state.start_x, mouse_state.start_y) {
                    let x1 = start_x.min(mouse_state.current_x).max(0);
                    let y1 = start_y.min(mouse_state.current_y).max(0);
                    let x2 = start_x.max(mouse_state.current_x).min(width as i32 - 1);
                    let y2 = start_y.max(mouse_state.current_y).min(height as i32 - 1);

                    let selected = SelectedRegion {
                        x: x1 as u32,
                        y: y1 as u32,
                        width: (x2 - x1) as u32,
                        height: (y2 - y1) as u32,
                    };

                    info!(
                        "✅ Região selecionada: {}x{} na posição ({}, {})",
                        selected.width, selected.height, selected.x, selected.y
                    );

                    *result.lock().unwrap() = Some(selected);
                    break;
                }
            }
        }

        // Desenha retângulo de seleção se estiver arrastando
        if mouse_state.is_dragging {
            if let (Some(start_x), Some(start_y)) = (mouse_state.start_x, mouse_state.start_y) {
                draw_selection_rect(
                    &mut buffer,
                    width,
                    height,
                    start_x,
                    start_y,
                    mouse_state.current_x,
                    mouse_state.current_y,
                );
            }
        }

        // Atualiza janela
        window.update_with_buffer(&buffer, width, height)?;

        // Restaura buffer (remove retângulo para próximo frame)
        if mouse_state.is_dragging {
            buffer = image
                .rgba()
                .chunks(4)
                .map(|rgba| {
                    let r = ((rgba[0] as u32) * 6 / 10) & 0xFF;
                    let g = ((rgba[1] as u32) * 6 / 10) & 0xFF;
                    let b = ((rgba[2] as u32) * 6 / 10) & 0xFF;
                    (r << 16) | (g << 8) | b
                })
                .collect();
        }
    }

    let final_result = result.lock().unwrap().clone();
    Ok(final_result)
}

/// Desenha retângulo de seleção no buffer
fn draw_selection_rect(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
) {
    let x_min = x1.min(x2).max(0) as usize;
    let x_max = x1.max(x2).min(width as i32 - 1) as usize;
    let y_min = y1.min(y2).max(0) as usize;
    let y_max = y1.max(y2).min(height as i32 - 1) as usize;

    let blue = 0x00_66_FF; // Azul brilhante
    let cyan = 0x00_FF_FF; // Ciano (preenchimento)

    // Preenchimento semitransparente
    for y in y_min..=y_max {
        for x in x_min..=x_max {
            let idx = y * width + x;
            if idx < buffer.len() {
                // Mistura com a cor original (efeito transparente)
                let original = buffer[idx];
                let r = ((original >> 16) & 0xFF) * 7 / 10 + ((cyan >> 16) & 0xFF) * 3 / 10;
                let g = ((original >> 8) & 0xFF) * 7 / 10 + ((cyan >> 8) & 0xFF) * 3 / 10;
                let b = (original & 0xFF) * 7 / 10 + (cyan & 0xFF) * 3 / 10;
                buffer[idx] = (r << 16) | (g << 8) | b;
            }
        }
    }

    // Borda (3 pixels de espessura)
    for thickness in 0..3 {
        // Linha superior
        for x in x_min..=x_max {
            if y_min + thickness < height {
                let idx = (y_min + thickness) * width + x;
                if idx < buffer.len() {
                    buffer[idx] = blue;
                }
            }
        }

        // Linha inferior
        for x in x_min..=x_max {
            if y_max >= thickness && y_max - thickness < height {
                let idx = (y_max - thickness) * width + x;
                if idx < buffer.len() {
                    buffer[idx] = blue;
                }
            }
        }

        // Linha esquerda
        for y in y_min..=y_max {
            if x_min + thickness < width {
                let idx = y * width + (x_min + thickness);
                if idx < buffer.len() {
                    buffer[idx] = blue;
                }
            }
        }

        // Linha direita
        for y in y_min..=y_max {
            if x_max >= thickness && x_max - thickness < width {
                let idx = y * width + (x_max - thickness);
                if idx < buffer.len() {
                    buffer[idx] = blue;
                }
            }
        }
    }
}
