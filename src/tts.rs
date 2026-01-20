// game-translator/src/tts.rs

// ============================================================================
// MÓDULO TTS - Text-to-Speech usando ElevenLabs
// ============================================================================
//
// Este módulo converte texto em áudio usando a API do ElevenLabs.
// O ElevenLabs retorna um arquivo MP3 que tocamos usando a biblioteca rodio.
//
// ============================================================================

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

// ============================================================================
// ESTRUTURAS DE DADOS
// ============================================================================

/// Configurações da requisição para ElevenLabs
#[derive(Debug, serde::Serialize)]
struct ElevenLabsRequest {
    /// Texto a ser convertido em áudio
    text: String,

    /// Configurações do modelo de voz
    model_id: String,

    /// Configurações de voz (estabilidade, similaridade, etc)
    voice_settings: VoiceSettings,
}

/// Configurações de voz do ElevenLabs
#[derive(Debug, serde::Serialize)]
struct VoiceSettings {
    /// Estabilidade da voz (0.0 a 1.0)
    /// Maior = mais consistente, Menor = mais expressivo
    stability: f32,

    /// Similaridade com a voz original (0.0 a 1.0)
    similarity_boost: f32,

    /// Estilo (0.0 a 1.0) - apenas para alguns modelos
    style: f32,

    /// Usar boost de alto-falante
    use_speaker_boost: bool,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        VoiceSettings {
            stability: 0.5,
            similarity_boost: 0.75,
            style: 0.0,
            use_speaker_boost: true,
        }
    }
}

// ============================================================================
// FUNÇÃO PRINCIPAL DE TTS
// ============================================================================

/// Converte texto em áudio usando ElevenLabs e reproduz
///
/// # Argumentos
/// * `text` - Texto a ser falado
/// * `api_key` - Chave da API do ElevenLabs
/// * `voice_id` - ID da voz personalizada
///
/// # Retorna
/// * `Result<()>` - Sucesso ou erro
///
/// # Exemplo
/// ```
/// speak("Olá, mundo!", "api-key", "voice-id").await?;
/// ```
pub async fn speak(text: &str, api_key: &str, voice_id: &str) -> Result<()> {
    info!("🔊 Iniciando síntese de voz...");
    info!("   📝 Texto: {} caracteres", text.len());

    // ========================================================================
    // VERIFICAÇÃO: Se não há API key ou voice_id, pula TTS
    // ========================================================================
    if api_key.is_empty() {
        info!("⚠️  ElevenLabs API key não configurada, pulando TTS");
        return Ok(());
    }

    if voice_id.is_empty() {
        info!("⚠️  ElevenLabs Voice ID não configurado, pulando TTS");
        return Ok(());
    }

    // ========================================================================
    // PASSO 1: Fazer requisição para a API do ElevenLabs
    // ========================================================================
    let audio_data = request_tts(text, api_key, voice_id).await?;

    // ========================================================================
    // PASSO 2: Tocar o áudio
    // ========================================================================
    play_audio(&audio_data)?;

    info!("✅ TTS concluído!");

    Ok(())
}

/// Converte texto em áudio mas NÃO toca (retorna os bytes do áudio)
///
/// Útil se você quiser salvar o áudio em arquivo ou processar depois.
pub async fn synthesize(text: &str, api_key: &str, voice_id: &str) -> Result<Vec<u8>> {
    request_tts(text, api_key, voice_id).await
}

// ============================================================================
// FUNÇÕES INTERNAS
// ============================================================================

/// Faz a requisição para a API do ElevenLabs
async fn request_tts(text: &str, api_key: &str, voice_id: &str) -> Result<Vec<u8>> {
    info!("   🌐 Enviando texto para ElevenLabs...");

    // URL da API do ElevenLabs para text-to-speech
    let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);

    // Monta o corpo da requisição
    let request_body = ElevenLabsRequest {
        text: text.to_string(),
        model_id: "eleven_multilingual_v2".to_string(), // Modelo multilíngue (suporta PT-BR)
        voice_settings: VoiceSettings::default(),
    };

    // Cria cliente HTTP
    let client = reqwest::Client::new();

    // Faz a requisição POST
    let response = client
        .post(&url)
        .header("xi-api-key", api_key)
        .header("Content-Type", "application/json")
        .header("Accept", "audio/mpeg") // Queremos MP3
        .json(&request_body)
        .send()
        .await
        .context("Falha ao enviar requisição para ElevenLabs")?;

    // Verifica se foi sucesso
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        error!(
            "❌ ElevenLabs API retornou erro: {} - {}",
            status, error_text
        );
        anyhow::bail!("ElevenLabs API erro {}: {}", status, error_text);
    }

    // Pega os bytes do áudio (MP3)
    let audio_bytes = response
        .bytes()
        .await
        .context("Falha ao receber áudio do ElevenLabs")?;

    info!("   ✅ Áudio recebido: {} bytes", audio_bytes.len());

    Ok(audio_bytes.to_vec())
}

/// Toca o áudio MP3 usando rodio
fn play_audio(audio_data: &[u8]) -> Result<()> {
    info!("   🔈 Tocando áudio...");

    // Cria um cursor para ler os bytes como se fosse um arquivo
    let cursor = Cursor::new(audio_data.to_vec());

    // Inicializa o sistema de áudio
    let (_stream, stream_handle) =
        OutputStream::try_default().context("Falha ao inicializar sistema de áudio")?;

    // Cria um sink (controla a reprodução)
    let sink = Sink::try_new(&stream_handle).context("Falha ao criar sink de áudio")?;

    // Decodifica o MP3
    let source = Decoder::new(cursor).context("Falha ao decodificar áudio MP3")?;

    // Adiciona ao sink e toca
    sink.append(source);

    // Aguarda terminar de tocar
    sink.sleep_until_end();

    info!("   ✅ Áudio reproduzido!");

    Ok(())
}

// ============================================================================
// FUNÇÃO AUXILIAR PARA TOCAR SEM BLOQUEAR
// ============================================================================

/// Toca o áudio em uma thread separada (não bloqueia)
pub fn play_audio_async(audio_data: Vec<u8>) {
    std::thread::spawn(move || {
        if let Err(e) = play_audio(&audio_data) {
            error!("❌ Erro ao tocar áudio: {}", e);
        }
    });
}
