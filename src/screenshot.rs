// game-translator/src/screenshot.rs

// ============================================================================
// MÓDULO SCREENSHOT - Captura de tela
// ============================================================================

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use screenshots::Screen;
use std::path::Path;

// ============================================================================
// CAPTURA DE TELA INTEIRA
// ============================================================================

/// Captura a tela inteira do monitor principal e salva em um arquivo
///
/// # Argumentos
/// * `output_path` - Caminho onde a imagem será salva
///
/// # Retorna
/// * `Result<DynamicImage>` - Imagem capturada ou erro
pub fn capture_screen(output_path: &Path) -> Result<DynamicImage> {
    info!("📸 Capturando tela inteira...");

    // Pega a lista de todos os monitores
    let screens = Screen::all().context("Falha ao listar monitores")?;

    // Pega o monitor principal (índice 0)
    let screen = screens.get(0).context("Nenhum monitor encontrado")?;

    info!(
        "   Monitor: {}x{}",
        screen.display_info.width, screen.display_info.height
    );

    // Captura a imagem do monitor
    let buffer = screen.capture().context("Falha ao capturar tela")?;

    // Converte o buffer para DynamicImage (formato padrão do crate image)
    let img = buffer_to_image(&buffer);

    // Salva a imagem em disco
    img.save(output_path)
        .context("Falha ao salvar screenshot")?;

    info!("✅ Screenshot salva em: {:?}", output_path);

    Ok(img)
}

// ============================================================================
// CAPTURA DE REGIÃO ESPECÍFICA (NOVO!)
// ============================================================================

/// Captura apenas uma região específica da tela
///
/// # Argumentos
/// * `output_path` - Caminho onde a imagem será salva
/// * `x` - Posição X do canto superior esquerdo (em pixels)
/// * `y` - Posição Y do canto superior esquerdo (em pixels)
/// * `width` - Largura da região (em pixels)
/// * `height` - Altura da região (em pixels)
///
/// # Retorna
/// * `Result<DynamicImage>` - Imagem capturada (apenas a região) ou erro
pub fn capture_region(
    output_path: &Path,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<DynamicImage> {
    info!("📸 Capturando região da tela...");
    info!("   Posição: ({}, {})", x, y);
    info!("   Tamanho: {}x{}", width, height);

    // ========================================================================
    // PASSO 1: Capturar a tela inteira primeiro
    // ========================================================================
    let screens = Screen::all().context("Falha ao listar monitores")?;

    let screen = screens.get(0).context("Nenhum monitor encontrado")?;

    let buffer = screen.capture().context("Falha ao capturar tela")?;

    let full_img = buffer_to_image(&buffer);

    // ========================================================================
    // PASSO 2: Validar se a região está dentro da tela
    // ========================================================================
    let screen_width = full_img.width();
    let screen_height = full_img.height();

    // Verifica se a região está completamente dentro da tela
    if x + width > screen_width || y + height > screen_height {
        anyhow::bail!(
            "Região ({},{} {}x{}) está fora dos limites da tela ({}x{})",
            x,
            y,
            width,
            height,
            screen_width,
            screen_height
        );
    }

    // ========================================================================
    // PASSO 3: Recortar apenas a região desejada
    // ========================================================================
    // O método crop() cria uma "view" da região sem copiar dados
    // Mas precisamos converter para DynamicImage para salvar
    let cropped = full_img.crop_imm(x, y, width, height);

    // ========================================================================
    // PASSO 4: Salvar a imagem recortada
    // ========================================================================
    cropped
        .save(output_path)
        .context("Falha ao salvar screenshot da região")?;

    info!("✅ Screenshot da região salva em: {:?}", output_path);

    Ok(cropped)
}

// ============================================================================
// FUNÇÃO AUXILIAR - Converte buffer para imagem
// ============================================================================

/// Converte o buffer da screenshot para DynamicImage
/// (função auxiliar interna)
fn buffer_to_image(buffer: &screenshots::Image) -> DynamicImage {
    let width = buffer.width();
    let height = buffer.height();
    let rgba = buffer.rgba();

    // Cria um ImageBuffer a partir dos bytes RGBA
    let img_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width as u32, height as u32, rgba.to_vec())
            .expect("Falha ao criar ImageBuffer");

    DynamicImage::ImageRgba8(img_buffer)
}
