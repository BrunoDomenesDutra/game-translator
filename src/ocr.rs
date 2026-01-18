// game-translator/src/ocr.rs

// ============================================================================
// MÓDULO OCR - Extração de texto usando Windows OCR Nativo
// ============================================================================

use anyhow::{Context, Result};
use std::path::Path;
use windows::{
    core::HSTRING,
    Graphics::Imaging::BitmapDecoder,
    Media::Ocr::{OcrEngine, OcrResult as WindowsOcrResult},
    Storage::{FileAccessMode, StorageFile},
};

// ============================================================================
// ESTRUTURAS DE DADOS
// ============================================================================

/// Representa um bloco de texto detectado com sua posição na tela
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

/// Resultado completo do OCR com posições
#[derive(Debug, Clone)]
pub struct OcrResultWithPositions {
    /// Texto completo (todas as linhas juntas)
    pub full_text: String,
    /// Lista de linhas detectadas com suas posições
    pub lines: Vec<DetectedText>,
}

/// Extrai texto de uma imagem usando Windows OCR nativo
///
/// # Argumentos
/// * `image_path` - Caminho para a imagem a ser processada
///
/// # Retorna
/// * `Result<String>` - Texto extraído ou erro
///
/// # Requisitos
/// * Windows 10 ou 11
/// * Pacote de idioma inglês instalado no Windows
pub fn extract_text(image_path: &Path) -> Result<String> {
    info!("🔍 Executando Windows OCR na imagem: {:?}", image_path);

    // ========================================================================
    // PASSO 1: Converter o caminho para caminho absoluto
    // ========================================================================
    // O Windows OCR precisa de caminho absoluto (completo)
    // Exemplo: "screenshot.png" vira "C:\Users\...\screenshot.png"
    let absolute_path = image_path
        .canonicalize()
        .context("Falha ao obter caminho absoluto da imagem")?;

    // Remove o prefixo "\\?\" que o Windows não gosta
    // canonicalize() retorna "\\?\C:\..." mas a API quer "C:\..."
    let path_str = absolute_path
        .to_string_lossy()
        .to_string()
        .trim_start_matches(r"\\?\")
        .to_string();

    info!("   📁 Caminho: {}", path_str);

    // ========================================================================
    // PASSO 2: Abrir o arquivo de imagem via API do Windows
    // ========================================================================
    // HSTRING é o tipo de string que o Windows usa internamente
    let path_hstring = HSTRING::from(&path_str);

    // Abre o arquivo usando a API de Storage do Windows
    // .get() bloqueia até a operação completar (síncrono)
    let file = StorageFile::GetFileFromPathAsync(&path_hstring)
        .context("Falha ao criar requisição de abertura do arquivo")?
        .get()
        .context("Falha ao abrir arquivo de imagem")?;

    info!("   📂 Arquivo aberto com sucesso");

    // ========================================================================
    // PASSO 3: Criar um decoder para ler a imagem
    // ========================================================================
    // Abre o arquivo em modo leitura
    let stream = file
        .OpenAsync(FileAccessMode::Read)
        .context("Falha ao criar requisição de stream")?
        .get()
        .context("Falha ao abrir stream do arquivo")?;

    // BitmapDecoder consegue ler PNG, JPG, BMP, etc.
    let decoder = BitmapDecoder::CreateAsync(&stream)
        .context("Falha ao criar requisição do decoder")?
        .get()
        .context("Falha ao criar decoder de imagem")?;

    info!("   🖼️  Imagem decodificada");

    // ========================================================================
    // PASSO 4: Converter para SoftwareBitmap (formato que o OCR aceita)
    // ========================================================================
    // GetSoftwareBitmapAsync extrai os pixels da imagem
    let bitmap = decoder
        .GetSoftwareBitmapAsync()
        .context("Falha ao criar requisição de bitmap")?
        .get()
        .context("Falha ao obter bitmap da imagem")?;

    info!("   📊 Bitmap extraído");

    // ========================================================================
    // PASSO 5: Criar o engine de OCR
    // ========================================================================
    // TryCreateFromUserProfileLanguages() usa os idiomas instalados no Windows
    // Se você tem inglês instalado, ele vai reconhecer inglês
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .context("Falha ao criar engine OCR. Verifique se há idiomas instalados no Windows.")?;

    info!("   ⚙️  Engine OCR criado");

    // ========================================================================
    // PASSO 6: Executar o OCR!
    // ========================================================================
    let result = engine
        .RecognizeAsync(&bitmap)
        .context("Falha ao criar requisição de OCR")?
        .get()
        .context("Falha ao executar OCR")?;

    // ========================================================================
    // PASSO 7: Extrair o texto do resultado
    // ========================================================================
    // result.Text() retorna todo o texto encontrado
    let text = result.Text().context("Falha ao obter texto do resultado")?;

    // Converte de HSTRING para String do Rust
    let text = text.to_string_lossy();

    if text.is_empty() {
        info!("⚠️  Nenhum texto detectado na imagem");
    } else {
        info!("✅ Texto extraído ({} caracteres)", text.len());
    }

    Ok(text)
}

/// Extrai texto COM posições usando Windows OCR nativo
///
/// # Argumentos
/// * `image_path` - Caminho para a imagem a ser processada
///
/// # Retorna
/// * `Result<OcrResult>` - Texto extraído com posições ou erro
pub fn extract_text_with_positions(image_path: &Path) -> Result<OcrResultWithPositions> {
    info!(
        "🔍 Executando Windows OCR (com posições) na imagem: {:?}",
        image_path
    );

    // ========================================================================
    // PASSO 1-6: Igual ao extract_text (abre arquivo, decodifica, roda OCR)
    // ========================================================================
    let absolute_path = image_path
        .canonicalize()
        .context("Falha ao obter caminho absoluto da imagem")?;

    let path_str = absolute_path
        .to_string_lossy()
        .to_string()
        .trim_start_matches(r"\\?\")
        .to_string();

    let path_hstring = HSTRING::from(&path_str);

    let file = StorageFile::GetFileFromPathAsync(&path_hstring)
        .context("Falha ao criar requisição de abertura do arquivo")?
        .get()
        .context("Falha ao abrir arquivo de imagem")?;

    let stream = file
        .OpenAsync(FileAccessMode::Read)
        .context("Falha ao criar requisição de stream")?
        .get()
        .context("Falha ao abrir stream do arquivo")?;

    let decoder = BitmapDecoder::CreateAsync(&stream)
        .context("Falha ao criar requisição do decoder")?
        .get()
        .context("Falha ao criar decoder de imagem")?;

    let bitmap = decoder
        .GetSoftwareBitmapAsync()
        .context("Falha ao criar requisição de bitmap")?
        .get()
        .context("Falha ao obter bitmap da imagem")?;

    let engine =
        OcrEngine::TryCreateFromUserProfileLanguages().context("Falha ao criar engine OCR")?;

    let result: WindowsOcrResult = engine
        .RecognizeAsync(&bitmap)
        .context("Falha ao criar requisição de OCR")?
        .get()
        .context("Falha ao executar OCR")?;

    // ========================================================================
    // PASSO 7: Extrair texto E posições
    // ========================================================================
    let full_text = result
        .Text()
        .context("Falha ao obter texto")?
        .to_string_lossy();

    // Pega todas as linhas detectadas
    let ocr_lines = result.Lines().context("Falha ao obter linhas do OCR")?;

    let mut lines: Vec<DetectedText> = Vec::new();

    // Itera sobre cada linha
    for i in 0..ocr_lines.Size()? {
        let line = ocr_lines.GetAt(i)?;

        // Texto da linha
        let line_text = line.Text()?.to_string_lossy();

        // Bounding box da linha (posição e tamanho)
        // Precisamos calcular a partir das palavras
        let words = line.Words()?;

        if words.Size()? > 0 {
            // Pega o bounding rect da primeira e última palavra
            // para calcular o rect da linha toda
            let first_word = words.GetAt(0)?;
            let first_rect = first_word.BoundingRect()?;

            let last_word = words.GetAt(words.Size()? - 1)?;
            let last_rect = last_word.BoundingRect()?;

            // Calcula o bounding box da linha completa
            let x = first_rect.X as f64;
            let y = first_rect.Y as f64;
            let width = (last_rect.X + last_rect.Width - first_rect.X) as f64;
            let height = first_rect.Height as f64; // Altura da primeira palavra

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
