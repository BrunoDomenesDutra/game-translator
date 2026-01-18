// game-translator/src/ocr.rs

// ============================================================================
// MÓDULO OCR - Extração de texto de imagens usando Tesseract
// ============================================================================

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Extrai texto de uma imagem usando Tesseract OCR via linha de comando
///
/// # Argumentos
/// * `image_path` - Caminho para a imagem a ser processada
///
/// # Retorna
/// * `Result<String>` - Texto extraído ou erro
pub fn extract_text(image_path: &Path) -> Result<String> {
    info!("🔍 Executando OCR na imagem: {:?}", image_path);

    // Executa o Tesseract via linha de comando
    // Equivalente a: tesseract imagem.png stdout -l eng
    let output = Command::new("tesseract")
        .arg(image_path) // Arquivo de entrada
        .arg("stdout") // Saída para stdout (em vez de arquivo)
        .arg("-l") // Idioma
        .arg("eng") // Inglês
        .output()
        .context("Falha ao executar Tesseract. Está instalado e no PATH?")?;

    // Verifica se houve erro
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Tesseract retornou erro: {}", error);
    }

    // Converte a saída para String
    let text = String::from_utf8(output.stdout).context("Falha ao ler saída do Tesseract")?;

    // Remove espaços em branco extras
    let text = text.trim().to_string();

    if text.is_empty() {
        info!("⚠️  Nenhum texto detectado na imagem");
    } else {
        info!("✅ Texto extraído ({} caracteres)", text.len());
    }

    Ok(text)
}
