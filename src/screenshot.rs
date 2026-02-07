// game-translator/src/screenshot.rs

// ============================================================================
// MÓDULO SCREENSHOT - Captura de tela
// ============================================================================

use anyhow::{Context, Result};
use image::{DynamicImage, ImageBuffer, Rgba};
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
// CAPTURA EM MEMÓRIA (SEM SALVAR EM DISCO) - MAIS RÁPIDO!
// ============================================================================

/// Captura a tela inteira e retorna a imagem em memória (não salva em disco)
///
/// # Retorna
/// * `Result<DynamicImage>` - Imagem capturada em memória
pub fn capture_screen_to_memory() -> Result<DynamicImage> {
    info!("📸 Capturando tela inteira (memória)...");

    let screens = Screen::all().context("Falha ao listar monitores")?;
    let screen = screens.get(0).context("Nenhum monitor encontrado")?;

    info!(
        "   Monitor: {}x{}",
        screen.display_info.width, screen.display_info.height
    );

    let buffer = screen.capture().context("Falha ao capturar tela")?;
    let img = buffer_to_image(&buffer);

    info!("✅ Screenshot capturada em memória!");

    Ok(img)
}

/// Captura uma região específica e retorna a imagem em memória (não salva em disco)
///
/// # Argumentos
/// * `x` - Posição X do canto superior esquerdo
/// * `y` - Posição Y do canto superior esquerdo
/// * `width` - Largura da região
/// * `height` - Altura da região
///
/// # Retorna
/// * `Result<DynamicImage>` - Imagem capturada em memória
pub fn capture_region_to_memory(x: u32, y: u32, width: u32, height: u32) -> Result<DynamicImage> {
    info!("📸 Capturando região (memória)...");
    info!("   Posição: ({}, {})", x, y);
    info!("   Tamanho: {}x{}", width, height);

    let screens = Screen::all().context("Falha ao listar monitores")?;
    let screen = screens.get(0).context("Nenhum monitor encontrado")?;

    let buffer = screen.capture().context("Falha ao capturar tela")?;
    let full_img = buffer_to_image(&buffer);

    // Valida se a região está dentro da tela
    let screen_width = full_img.width();
    let screen_height = full_img.height();

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

    // Recorta a região
    let cropped = full_img.crop_imm(x, y, width, height);

    info!("✅ Screenshot da região capturada em memória!");

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

/// Pré-processa uma imagem para melhorar o OCR
/// - Converte para escala de cinza
/// - Aumenta contraste
/// - Aplica threshold (binarização)
/// - Inverte cores (texto branco -> texto preto)
pub fn preprocess_image(
    image: &image::DynamicImage,
    grayscale: bool,
    invert: bool,
    contrast: f32,
    threshold: u8,
    save_debug: bool,
    upscale: f32,
    blur: f32,
) -> image::DynamicImage {
    let mut processed = image.clone();

    // 0. Upscale — redimensiona a imagem ANTES de qualquer processamento
    // O Windows OCR funciona MUITO melhor com texto grande (>30px).
    // Se o texto no jogo é pequeno, upscale 2x ou 3x melhora bastante.
    // Fazemos isso PRIMEIRO porque os outros filtros (threshold, contraste)
    // funcionam melhor em imagens maiores com mais detalhe nos pixels.
    if upscale > 1.0 {
        let (w, h) = (processed.width(), processed.height());
        let new_w = (w as f32 * upscale) as u32;
        let new_h = (h as f32 * upscale) as u32;

        // image::imageops::FilterType::Lanczos3 é o melhor filtro para upscale
        // Ele preserva bordas nítidas (perfeito para texto)
        // Outros filtros disponíveis:
        //   Nearest  = mais rápido, mas pixelado (ruim para OCR)
        //   Triangle = ok, mas borra um pouco
        //   Lanczos3 = mais lento, mas bordas nítidas (melhor para texto!)
        processed = processed.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);

        info!(
            "   🔍 Upscale: {}x{} → {}x{} (fator {:.1}x)",
            w, h, new_w, new_h, upscale
        );
    }

    // 0.5. Blur gaussiano — suaviza sombras e artefatos visuais
    // Aplicado ANTES do grayscale/threshold para que a suavização
    // elimine bordas duras de sombras e efeitos do jogo.
    // O threshold depois "limpa" o resultado borrado.
    if blur > 0.0 {
        let sigma = blur; // sigma controla a intensidade do blur
        processed = processed.blur(sigma);
        info!("   🌫️ Blur aplicado: sigma={:.1}", sigma);
    }

    // 1. Converte para escala de cinza
    if grayscale {
        processed = image::DynamicImage::ImageLuma8(processed.to_luma8());
        // Converte de volta para RGB para manter compatibilidade
        processed = image::DynamicImage::ImageRgb8(processed.to_rgb8());
    }

    // 2. Aumenta contraste
    if contrast != 1.0 {
        processed = processed.adjust_contrast(contrast);
    }

    // 3. Aplica threshold (binarização) se > 0
    if threshold > 0 {
        let rgb = processed.to_rgb8();
        let (width, height) = rgb.dimensions();

        let mut binary = image::RgbImage::new(width, height);

        for (x, y, pixel) in rgb.enumerate_pixels() {
            // Calcula luminância do pixel
            let luma =
                (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) as u8;

            // Aplica threshold: acima = branco, abaixo = preto
            let value = if luma > threshold { 255 } else { 0 };
            binary.put_pixel(x, y, image::Rgb([value, value, value]));
        }

        processed = image::DynamicImage::ImageRgb8(binary);
    }

    // 4. Inverte cores
    if invert {
        processed.invert();
    }

    // 5. Salva imagem de debug se solicitado
    if save_debug {
        if let Err(e) = processed.save("debug_preprocessed.png") {
            error!("❌ Erro ao salvar imagem de debug: {}", e);
        } else {
            trace!("📸 Imagem de debug salva: debug_preprocessed.png");
        }
    }

    processed
}
