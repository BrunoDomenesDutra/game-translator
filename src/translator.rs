// game-translator/src/translator.rs

// ============================================================================
// MÓDULO TRANSLATOR - Tradução usando DeepL API
// ============================================================================

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ============================================================================
// ESTRUTURAS DE DADOS
// ============================================================================

/// Estrutura que enviamos para a API do DeepL
/// Serializa para JSON automaticamente (graças ao #[derive(Serialize)])
#[derive(Debug, Serialize)]
struct DeepLRequest {
    /// Lista de textos a traduzir (DeepL aceita múltiplos textos de uma vez)
    text: Vec<String>,

    /// Idioma de destino (PT-BR = Português Brasileiro)
    target_lang: String,

    /// Idioma de origem (EN = Inglês)
    source_lang: String,
}

/// Estrutura que recebemos da API do DeepL
/// Deserializa do JSON automaticamente (graças ao #[derive(Deserialize)])
#[derive(Debug, Deserialize)]
struct DeepLResponse {
    /// Lista de traduções (uma para cada texto enviado)
    translations: Vec<Translation>,
}

/// Cada tradução individual
#[derive(Debug, Deserialize)]
struct Translation {
    /// Idioma detectado automaticamente pela API
    detected_source_language: String,

    /// Texto traduzido
    text: String,
}

// ============================================================================
// FUNÇÃO DE TRADUÇÃO
// ============================================================================

/// Traduz texto de inglês para português brasileiro usando DeepL
///
/// # Argumentos
/// * `text` - Texto em inglês a ser traduzido
/// * `api_key` - Chave da API do DeepL
///
/// # Retorna
/// * `Result<String>` - Texto traduzido ou erro
///
/// # Exemplo
/// ```
/// let traducao = translate("Hello world", "minha-api-key").await?;
/// println!("{}", traducao); // Imprime: "Olá mundo"
/// ```
pub async fn translate(text: &str, api_key: &str) -> Result<String> {
    info!("🌐 Iniciando tradução...");
    info!("   📝 Texto original: {} caracteres", text.len());

    // ========================================================================
    // VERIFICAÇÃO: Se não há API key configurada, retorna tradução fake
    // ========================================================================
    if api_key == "fake-api-key" || api_key.is_empty() {
        info!("⚠️  API key do DeepL não configurada");
        info!("   💡 Configure DEEPL_API_KEY no arquivo .env");
        return Ok(format!("[TRADUÇÃO FAKE] {}", text));
    }

    // ========================================================================
    // PASSO 1: Criar cliente HTTP
    // ========================================================================
    // O reqwest::Client é como o axios do Node.js
    let client = reqwest::Client::new();

    // ========================================================================
    // PASSO 2: Montar o corpo da requisição (payload JSON)
    // ========================================================================
    let request_body = DeepLRequest {
        text: vec![text.to_string()],     // Converte para Vec (lista) de Strings
        target_lang: "PT-BR".to_string(), // Português do Brasil
        source_lang: "EN".to_string(),    // Inglês
    };

    info!("   🌐 Enviando requisição para DeepL API...");

    // ========================================================================
    // PASSO 3: Fazer requisição POST para a API
    // ========================================================================
    let response = client
        .post("https://api-free.deepl.com/v2/translate") // URL da API (versão FREE)
        .header("Authorization", format!("DeepL-Auth-Key {}", api_key)) // Header de autenticação
        .header("Content-Type", "application/json") // Tipo do conteúdo
        .json(&request_body) // Serializa o request_body para JSON automaticamente
        .send() // Envia a requisição
        .await // Aguarda a resposta (assíncrono)
        .context("Falha ao enviar requisição para DeepL")?; // Se der erro, retorna mensagem

    // ========================================================================
    // PASSO 4: Verificar se a API retornou sucesso (status 200-299)
    // ========================================================================
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        error!("❌ DeepL API retornou erro!");
        error!("   Status: {}", status);
        error!("   Mensagem: {}", error_text);

        anyhow::bail!("DeepL API retornou erro {}: {}", status, error_text);
    }

    // ========================================================================
    // PASSO 5: Parsear (deserializar) a resposta JSON
    // ========================================================================
    let deepl_response: DeepLResponse = response
        .json() // Converte o JSON para a struct DeepLResponse automaticamente
        .await
        .context("Falha ao parsear resposta da DeepL")?;

    // ========================================================================
    // PASSO 6: Extrair o texto traduzido
    // ========================================================================
    let translated_text = deepl_response
        .translations // Pega a lista de traduções
        .first() // Pega a primeira (só enviamos um texto)
        .context("Nenhuma tradução retornada pela API")? // Retorna erro se não houver
        .text // Pega o campo "text"
        .clone(); // Clona o texto (cria uma cópia)

    info!("✅ Tradução concluída!");
    info!(
        "   🇧🇷 Texto traduzido: {} caracteres",
        translated_text.len()
    );

    Ok(translated_text)
}

// ============================================================================
// TRADUÇÃO EM BATCH (MÚLTIPLOS TEXTOS DE UMA VEZ)
// ============================================================================

/// Traduz múltiplos textos de uma vez (muito mais rápido!)
///
/// # Argumentos
/// * `texts` - Lista de textos em inglês a serem traduzidos
/// * `api_key` - Chave da API do DeepL
///
/// # Retorna
/// * `Result<Vec<String>>` - Lista de textos traduzidos (na mesma ordem)
///
/// # Exemplo
/// ```
/// let textos = vec!["Hello", "World", "Game"];
/// let traducoes = translate_batch(&textos, "api-key").await?;
/// // traducoes = ["Olá", "Mundo", "Jogo"]
/// ```
pub async fn translate_batch(texts: &[String], api_key: &str) -> Result<Vec<String>> {
    info!("🌐 Iniciando tradução em batch...");
    info!("   📝 {} textos para traduzir", texts.len());

    // Se não há textos, retorna lista vazia
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    // ========================================================================
    // VERIFICAÇÃO: Se não há API key configurada, retorna tradução fake
    // ========================================================================
    if api_key == "fake-api-key" || api_key.is_empty() {
        info!("⚠️  API key do DeepL não configurada");
        info!("   💡 Configure DEEPL_API_KEY no arquivo .env");

        // Retorna traduções fake (só adiciona prefixo)
        let fake_translations: Vec<String> =
            texts.iter().map(|t| format!("[FAKE] {}", t)).collect();

        return Ok(fake_translations);
    }

    // ========================================================================
    // PASSO 1: Criar cliente HTTP
    // ========================================================================
    let client = reqwest::Client::new();

    // ========================================================================
    // PASSO 2: Montar o corpo da requisição
    // ========================================================================
    // DeepL aceita múltiplos textos no campo "text" (array)
    let request_body = DeepLRequest {
        text: texts.to_vec(), // Todos os textos de uma vez!
        target_lang: "PT-BR".to_string(),
        source_lang: "EN".to_string(),
    };

    info!("   🌐 Enviando {} textos para DeepL API...", texts.len());

    // ========================================================================
    // PASSO 3: Fazer requisição POST
    // ========================================================================
    let response = client
        .post("https://api-free.deepl.com/v2/translate")
        .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("Falha ao enviar requisição para DeepL")?;

    // ========================================================================
    // PASSO 4: Verificar se a API retornou sucesso
    // ========================================================================
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        error!("❌ DeepL API retornou erro: {} - {}", status, error_text);
        anyhow::bail!("DeepL API erro {}: {}", status, error_text);
    }

    // ========================================================================
    // PASSO 5: Parsear a resposta
    // ========================================================================
    let deepl_response: DeepLResponse = response
        .json()
        .await
        .context("Falha ao parsear resposta da DeepL")?;

    // ========================================================================
    // PASSO 6: Extrair todos os textos traduzidos
    // ========================================================================
    // A API retorna as traduções na mesma ordem que enviamos
    let translated_texts: Vec<String> = deepl_response
        .translations
        .iter()
        .map(|t| t.text.clone())
        .collect();

    info!("✅ Tradução em batch concluída!");
    info!("   🇧🇷 {} textos traduzidos", translated_texts.len());

    Ok(translated_texts)
}
