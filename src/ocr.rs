// game-translator/src/ocr.rs

// ============================================================================
// MÓDULO OCR - Extração de texto usando Windows OCR Nativo
// ============================================================================
//
// Este módulo usa a API de OCR nativa do Windows 10/11 para extrair texto
// de imagens. Suporta dois modos:
//
// 1. MODO MEMÓRIA (rápido) - Processa imagem direto da RAM
// 2. MODO ARQUIVO (debug) - Lê imagem de arquivo em disco
//
// ============================================================================

use anyhow::{Context, Result};
use image::DynamicImage;
use std::io::Cursor;
use std::path::Path;
use windows::{
    core::HSTRING,
    Graphics::Imaging::BitmapDecoder,
    Media::Ocr::OcrEngine,
    Storage::Streams::{DataWriter, InMemoryRandomAccessStream},
    Storage::{FileAccessMode, StorageFile},
};

// ============================================================================
// ESTRUTURAS DE DADOS
// ============================================================================

/// Representa um bloco de texto detectado com sua posição na imagem
#[derive(Debug, Clone)]
pub struct DetectedText {
    /// O texto detectado
    pub text: String,
    /// Posição X (pixels a partir da esquerda da imagem)
    pub x: f64,
    /// Posição Y (pixels a partir do topo da imagem)
    pub y: f64,
    /// Largura do bloco de texto
    pub width: f64,
    /// Altura do bloco de texto
    pub height: f64,
}

/// Representa um texto traduzido com sua posição na TELA (coordenadas absolutas)
#[derive(Debug, Clone)]
pub struct TranslatedText {
    /// Texto original (inglês)
    pub original: String,
    /// Texto traduzido (português)
    pub translated: String,
    /// Posição X na tela (coordenadas absolutas do monitor)
    pub screen_x: f64,
    /// Posição Y na tela (coordenadas absolutas do monitor)
    pub screen_y: f64,
    /// Largura do bloco original
    pub width: f64,
    /// Altura do bloco original
    pub height: f64,
}

/// Resultado completo do OCR com posições
#[derive(Debug, Clone)]
pub struct OcrResultWithPositions {
    /// Texto completo (todas as linhas juntas)
    pub full_text: String,
    /// Lista de linhas detectadas com suas posições
    pub lines: Vec<DetectedText>,
}

// ============================================================================
// OCR DA MEMÓRIA (MODO RÁPIDO)
// ============================================================================

/// Extrai texto COM posições de uma imagem em memória (não usa disco)
///
/// Este é o modo mais rápido pois não precisa salvar/ler arquivo.
///
/// # Argumentos
/// * `image` - Imagem em memória (DynamicImage)
///
/// # Retorna
/// * `Result<OcrResultWithPositions>` - Texto extraído com posições
pub fn extract_text_from_memory(image: &DynamicImage) -> Result<OcrResultWithPositions> {
    info!("🔍 Executando Windows OCR (memória)...");

    // ========================================================================
    // PASSO 1: Converter imagem para PNG em memória (bytes)
    // ========================================================================
    let mut png_bytes: Vec<u8> = Vec::new();
    {
        let mut cursor = Cursor::new(&mut png_bytes);
        image
            .write_to(&mut cursor, image::ImageFormat::Png)
            .context("Falha ao converter imagem para PNG")?;
    }

    info!("   📊 Imagem: {} bytes", png_bytes.len());

    // ========================================================================
    // PASSO 2: Criar stream de memória para o Windows
    // ========================================================================
    let stream = InMemoryRandomAccessStream::new().context("Falha ao criar stream em memória")?;

    // Escreve os bytes PNG no stream
    {
        let writer = DataWriter::CreateDataWriter(&stream).context("Falha ao criar DataWriter")?;

        writer
            .WriteBytes(&png_bytes)
            .context("Falha ao escrever bytes")?;

        writer
            .StoreAsync()
            .context("Falha ao iniciar store")?
            .get()
            .context("Falha ao armazenar bytes")?;

        writer.DetachStream().context("Falha ao desanexar stream")?;
    }

    // Volta para o início do stream
    stream
        .Seek(0)
        .context("Falha ao voltar ao início do stream")?;

    // ========================================================================
    // PASSO 3: Decodificar a imagem e executar OCR
    // ========================================================================
    let decoder = BitmapDecoder::CreateAsync(&stream)
        .context("Falha ao criar decoder")?
        .get()
        .context("Falha ao decodificar imagem")?;

    let bitmap = decoder
        .GetSoftwareBitmapAsync()
        .context("Falha ao criar bitmap")?
        .get()
        .context("Falha ao obter bitmap")?;

    let engine =
        OcrEngine::TryCreateFromUserProfileLanguages().context("Falha ao criar engine OCR")?;

    let result = engine
        .RecognizeAsync(&bitmap)
        .context("Falha ao executar OCR")?
        .get()
        .context("Falha ao obter resultado OCR")?;

    // ========================================================================
    // PASSO 4: Extrair texto e posições
    // ========================================================================
    extract_lines_from_result(&result)
}

// ============================================================================
// OCR DE ARQUIVO (MODO DEBUG)
// ============================================================================

/// Extrai texto COM posições de um arquivo de imagem em disco
///
/// Mais lento que o modo memória, mas útil para debug (pode ver o screenshot.png)
///
/// # Argumentos
/// * `image_path` - Caminho para a imagem
///
/// # Retorna
/// * `Result<OcrResultWithPositions>` - Texto extraído com posições
pub fn extract_text_with_positions(image_path: &Path) -> Result<OcrResultWithPositions> {
    info!("🔍 Executando Windows OCR (arquivo): {:?}", image_path);

    // ========================================================================
    // PASSO 1: Abrir arquivo de imagem
    // ========================================================================
    let absolute_path = image_path
        .canonicalize()
        .context("Falha ao obter caminho absoluto")?;

    let path_str = absolute_path
        .to_string_lossy()
        .to_string()
        .trim_start_matches(r"\\?\")
        .to_string();

    let path_hstring = HSTRING::from(&path_str);

    let file = StorageFile::GetFileFromPathAsync(&path_hstring)
        .context("Falha ao abrir arquivo")?
        .get()
        .context("Falha ao obter arquivo")?;

    let stream = file
        .OpenAsync(FileAccessMode::Read)
        .context("Falha ao abrir stream")?
        .get()
        .context("Falha ao obter stream")?;

    // ========================================================================
    // PASSO 2: Decodificar e executar OCR
    // ========================================================================
    let decoder = BitmapDecoder::CreateAsync(&stream)
        .context("Falha ao criar decoder")?
        .get()
        .context("Falha ao decodificar")?;

    let bitmap = decoder
        .GetSoftwareBitmapAsync()
        .context("Falha ao criar bitmap")?
        .get()
        .context("Falha ao obter bitmap")?;

    let engine =
        OcrEngine::TryCreateFromUserProfileLanguages().context("Falha ao criar engine OCR")?;

    let result = engine
        .RecognizeAsync(&bitmap)
        .context("Falha ao executar OCR")?
        .get()
        .context("Falha ao obter resultado")?;

    // ========================================================================
    // PASSO 3: Extrair texto e posições
    // ========================================================================
    extract_lines_from_result(&result)
}

// ============================================================================
// FUNÇÃO AUXILIAR - Extrai linhas do resultado do OCR
// ============================================================================

/// Extrai as linhas de texto e suas posições do resultado do Windows OCR
fn extract_lines_from_result(
    result: &windows::Media::Ocr::OcrResult,
) -> Result<OcrResultWithPositions> {
    let full_text = result
        .Text()
        .context("Falha ao obter texto")?
        .to_string_lossy();

    let ocr_lines = result.Lines().context("Falha ao obter linhas")?;

    let mut lines: Vec<DetectedText> = Vec::new();

    for i in 0..ocr_lines.Size()? {
        let line = ocr_lines.GetAt(i)?;
        let line_text = line.Text()?.to_string_lossy();
        let words = line.Words()?;

        if words.Size()? > 0 {
            // Calcula bounding box da linha usando primeira e última palavra
            let first_word = words.GetAt(0)?;
            let first_rect = first_word.BoundingRect()?;

            let last_word = words.GetAt(words.Size()? - 1)?;
            let last_rect = last_word.BoundingRect()?;

            let x = first_rect.X as f64;
            let y = first_rect.Y as f64;
            let width = (last_rect.X + last_rect.Width - first_rect.X) as f64;
            let height = first_rect.Height as f64;

            info!(
                "   📍 Linha {}: \"{}\" em ({:.0}, {:.0}) {}x{}",
                i, line_text, x, y, width as i32, height as i32
            );

            lines.push(DetectedText {
                text: line_text,
                x,
                y,
                width,
                height,
            });
        }
    }

    info!("✅ OCR completo: {} linhas detectadas", lines.len());

    Ok(OcrResultWithPositions { full_text, lines })
}

/// Limpa texto do OCR corrigindo erros comuns de reconhecimento
pub fn clean_ocr_text(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Substituições de padrões comuns de erro do OCR
    let replacements = [
        // Letra K confundida
        ("|<", "K"),
        ("l<", "K"),
        ("|{", "K"),
        // Letra I confundida
        ("|", "I"), // Cuidado: só aplicar em contextos específicos
        // Letra O e zero
        // ("0", "O"),  // Perigoso, pode ter números reais
        // Outros
        ("@", "a"),
        ("}{", "H"),
        ("][", "I"),
        ("|-|", "H"),
        ("/\\", "A"),
        ("\\/", "V"),
    ];

    for (wrong, correct) in replacements {
        cleaned = cleaned.replace(wrong, correct);
    }

    // Remove caracteres estranhos que não deveriam estar em legendas
    // Mantém letras, números, espaços, pontuação básica
    cleaned = cleaned
        .chars()
        .filter(|c| {
            c.is_alphanumeric()
                || c.is_whitespace()
                || matches!(
                    c,
                    '.' | ',' | '!' | '?' | '\'' | '"' | '-' | ':' | ';' | '(' | ')' | '…'
                )
        })
        .collect();

    // Remove espaços duplicados
    while cleaned.contains("  ") {
        cleaned = cleaned.replace("  ", " ");
    }

    cleaned.trim().to_string()
}
