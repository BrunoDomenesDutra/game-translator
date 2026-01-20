// game-translator/src/translator.rs

// ============================================================================
// MÓDULO TRANSLATOR - Tradução usando múltiplos provedores
// ============================================================================
//
// Provedores suportados:
// - DeepL (requer API key, melhor qualidade)
// - Google Translate (grátis, sem API key)
//
// ============================================================================

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ============================================================================
// ESTRUTURAS DE DADOS - DeepL
// ============================================================================

#[derive(Debug, Serialize)]
struct DeepLRequest {
    text: Vec<String>,
    target_lang: String,
    source_lang: String,
}

#[derive(Debug, Deserialize)]
struct DeepLResponse {
    translations: Vec<DeepLTranslation>,
}

#[derive(Debug, Deserialize)]
struct DeepLTranslation {
    text: String,
}

// ============================================================================
// ESTRUTURAS DE DADOS - Google Translate
// ============================================================================

// Google retorna um array aninhado complexo, vamos parsear manualmente

// ============================================================================
// FUNÇÃO PRINCIPAL - TRADUÇÃO EM BATCH
// ============================================================================

/// Traduz múltiplos textos usando o provedor configurado
///
/// # Argumentos
/// * `texts` - Lista de textos a traduzir
/// * `provider` - Provedor: "deepl" ou "google"
/// * `api_key` - API key (só necessário para DeepL)
/// * `source_lang` - Idioma de origem (ex: "EN", "auto")
/// * `target_lang` - Idioma de destino (ex: "PT-BR")
///
/// # Retorna
/// * `Result<Vec<String>>` - Lista de textos traduzidos
pub async fn translate_batch_with_provider(
    texts: &[String],
    provider: &str,
    api_key: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<Vec<String>> {
    match provider.to_lowercase().as_str() {
        "deepl" => translate_batch_deepl(texts, api_key, source_lang, target_lang).await,
        "google" => translate_batch_google(texts, source_lang, target_lang).await,
        _ => {
            warn!("⚠️  Provedor '{}' não reconhecido, usando Google", provider);
            translate_batch_google(texts, source_lang, target_lang).await
        }
    }
}

/// Função de compatibilidade (usa DeepL por padrão)
pub async fn translate_batch(texts: &[String], api_key: &str) -> Result<Vec<String>> {
    translate_batch_deepl(texts, api_key, "EN", "PT-BR").await
}

// ============================================================================
// DeepL TRADUTOR
// ============================================================================

async fn translate_batch_deepl(
    texts: &[String],
    api_key: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<Vec<String>> {
    info!("🌐 [DeepL] Iniciando tradução em batch...");
    info!("   📝 {} textos para traduzir", texts.len());

    if texts.is_empty() {
        return Ok(Vec::new());
    }

    // Verifica API key
    if api_key.is_empty() || api_key == "fake-api-key" {
        warn!("⚠️  DeepL API key não configurada!");
        return Ok(texts
            .iter()
            .map(|t| format!("[SEM API KEY] {}", t))
            .collect());
    }

    let client = reqwest::Client::new();

    let request_body = DeepLRequest {
        text: texts.to_vec(),
        target_lang: target_lang.to_string(),
        source_lang: source_lang.to_string(),
    };

    info!("   🌐 Enviando {} textos para DeepL API...", texts.len());

    let response = client
        .post("https://api-free.deepl.com/v2/translate")
        .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("Falha ao enviar requisição para DeepL")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        error!("❌ DeepL API erro: {} - {}", status, error_text);
        anyhow::bail!("DeepL API erro {}: {}", status, error_text);
    }

    let deepl_response: DeepLResponse = response
        .json()
        .await
        .context("Falha ao parsear resposta DeepL")?;

    let translated: Vec<String> = deepl_response
        .translations
        .iter()
        .map(|t| t.text.clone())
        .collect();

    info!("✅ [DeepL] Tradução concluída!");
    info!("   🇧🇷 {} textos traduzidos", translated.len());

    Ok(translated)
}

// ============================================================================
// GOOGLE TRANSLATE (GRÁTIS, SEM API KEY)
// ============================================================================

async fn translate_batch_google(
    texts: &[String],
    source_lang: &str,
    target_lang: &str,
) -> Result<Vec<String>> {
    info!("🌐 [Google] Iniciando tradução em batch...");
    info!("   📝 {} textos para traduzir", texts.len());

    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let client = reqwest::Client::new();
    let mut translated_texts: Vec<String> = Vec::new();

    // Converte códigos de idioma para formato do Google
    let source = convert_lang_code_to_google(source_lang);
    let target = convert_lang_code_to_google(target_lang);

    // Google Translate não aceita batch oficial, então traduzimos um por um
    // Mas podemos juntar textos com separador para otimizar
    let combined_text = texts.join("\n||||\n");

    info!("   🌐 Enviando para Google Translate...");

    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        source,
        target,
        urlencoding::encode(&combined_text)
    );

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .context("Falha ao enviar requisição para Google Translate")?;

    if !response.status().is_success() {
        let status = response.status();
        error!("❌ Google Translate erro: {}", status);
        anyhow::bail!("Google Translate erro: {}", status);
    }

    let response_text = response.text().await?;

    // Parseia a resposta do Google (formato JSON aninhado complexo)
    let translated_combined = parse_google_response(&response_text)?;

    // Separa os textos de volta
    let parts: Vec<&str> = translated_combined.split("||||").collect();

    for (i, part) in parts.iter().enumerate() {
        let cleaned = part.trim();
        if i < texts.len() {
            translated_texts.push(cleaned.to_string());
        }
    }

    // Se não conseguiu separar corretamente, retorna o texto combinado
    if translated_texts.len() != texts.len() {
        warn!("⚠️  Número de traduções diferente do esperado, ajustando...");
        translated_texts.clear();

        // Traduz um por um como fallback
        for text in texts {
            let single_translated =
                translate_single_google(&client, text, &source, &target).await?;
            translated_texts.push(single_translated);
        }
    }

    info!("✅ [Google] Tradução concluída!");
    info!("   🇧🇷 {} textos traduzidos", translated_texts.len());

    Ok(translated_texts)
}

/// Traduz um único texto via Google Translate
async fn translate_single_google(
    client: &reqwest::Client,
    text: &str,
    source: &str,
    target: &str,
) -> Result<String> {
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        source,
        target,
        urlencoding::encode(text)
    );

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .context("Falha na requisição Google Translate")?;

    let response_text = response.text().await?;
    parse_google_response(&response_text)
}

/// Parseia a resposta JSON do Google Translate
/// O formato é um array aninhado: [[["texto traduzido","texto original",...],...],...]
fn parse_google_response(response: &str) -> Result<String> {
    // Tenta parsear como JSON
    let json: serde_json::Value =
        serde_json::from_str(response).context("Falha ao parsear resposta do Google")?;

    let mut translated = String::new();

    // O formato é: [[["tradução", "original", ...], ...], ...]
    if let Some(outer_array) = json.as_array() {
        if let Some(first) = outer_array.first() {
            if let Some(sentences) = first.as_array() {
                for sentence in sentences {
                    if let Some(arr) = sentence.as_array() {
                        if let Some(text) = arr.first() {
                            if let Some(s) = text.as_str() {
                                translated.push_str(s);
                            }
                        }
                    }
                }
            }
        }
    }

    if translated.is_empty() {
        anyhow::bail!("Não foi possível extrair tradução da resposta");
    }

    Ok(translated)
}

/// Converte códigos de idioma do DeepL para Google
fn convert_lang_code_to_google(lang: &str) -> String {
    match lang.to_uppercase().as_str() {
        "PT-BR" => "pt".to_string(),
        "PT-PT" => "pt".to_string(),
        "EN-US" => "en".to_string(),
        "EN-GB" => "en".to_string(),
        "ZH" => "zh-CN".to_string(),
        "JA" => "ja".to_string(),
        code => code.to_lowercase(),
    }
}
