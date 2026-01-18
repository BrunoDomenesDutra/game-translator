// game-translator/src/tts.rs

// ============================================================================
// MÓDULO TTS - Text-to-Speech usando ElevenLabs
// ============================================================================

use anyhow::Result;

/// Converte texto em áudio usando ElevenLabs e reproduz
///
/// # Argumentos
/// * `text` - Texto a ser falado
/// * `api_key` - Chave da API do ElevenLabs
/// * `voice_id` - ID da voz personalizada
///
/// # Retorna
/// * `Result<()>` - Sucesso ou erro
pub async fn speak(text: &str, api_key: &str, voice_id: &str) -> Result<()> {
    info!("🔊 Sintetizando voz...");

    // TODO: Implementar chamada real à API do ElevenLabs
    // Por enquanto, apenas loga
    info!("⚠️  TTS ainda não implementado (placeholder)");
    info!("   Texto: {}", text);

    Ok(())
}
