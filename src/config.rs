// ============================================================================
// MÓDULO CONFIG - Configurações da aplicação
// ============================================================================

use anyhow::Result;
use std::env;

/// Estrutura que guarda todas as configurações da aplicação
/// Similar a um objeto/interface do TypeScript
#[derive(Debug, Clone)]
pub struct Config {
    /// API key do DeepL para tradução
    pub deepl_api_key: String,

    /// API key do ElevenLabs para TTS (vamos usar depois)
    pub elevenlabs_api_key: String,

    /// ID da voz no ElevenLabs (vamos usar depois)
    pub elevenlabs_voice_id: String,

    // ========================================================================
    // CONFIGURAÇÕES DE CAPTURA DE TELA
    // ========================================================================
    /// Se true, captura apenas uma região. Se false, captura tela inteira
    pub use_region_capture: bool,

    /// Posição X do canto superior esquerdo da região (em pixels)
    pub region_x: u32,

    /// Posição Y do canto superior esquerdo da região (em pixels)
    pub region_y: u32,

    /// Largura da região a capturar (em pixels)
    pub region_width: u32,

    /// Altura da região a capturar (em pixels)
    pub region_height: u32,
}

impl Config {
    /// Carrega as configurações das variáveis de ambiente
    ///
    /// Tenta ler do arquivo .env primeiro, depois das variáveis de ambiente do sistema
    ///
    /// # Retorna
    /// * `Result<Config>` - Configuração carregada ou erro
    pub fn load() -> Result<Self> {
        info!("📋 Carregando configurações...");

        // ====================================================================
        // PASSO 1: Tentar carregar o arquivo .env
        // ====================================================================
        // O dotenv::dotenv() lê o arquivo .env e coloca as variáveis
        // no ambiente (como se você tivesse feito $env:VARIAVEL="valor")
        // .ok() significa "se der erro, ignora e continua"
        dotenv::dotenv().ok();

        // ====================================================================
        // PASSO 2: Ler a API key do DeepL
        // ====================================================================
        // env::var() tenta ler uma variável de ambiente
        // Se não existir, usamos .unwrap_or_else() para definir um valor padrão
        let deepl_api_key = env::var("DEEPL_API_KEY").unwrap_or_else(|_| {
            // Se não encontrou a variável, loga um aviso
            warn!("⚠️  DEEPL_API_KEY não configurada no arquivo .env");
            warn!("   💡 Crie um arquivo .env com: DEEPL_API_KEY=sua-chave-aqui");
            "fake-api-key".to_string()
        });

        // ====================================================================
        // PASSO 3: Ler as configurações do ElevenLabs (opcional por enquanto)
        // ====================================================================
        let elevenlabs_api_key = env::var("ELEVENLABS_API_KEY").unwrap_or_else(|_| {
            // Não mostra aviso porque ElevenLabs ainda não está implementado
            String::new()
        });

        let elevenlabs_voice_id = env::var("ELEVENLABS_VOICE_ID").unwrap_or_else(|_| String::new());

        // ====================================================================
        // PASSO 4: Mostrar status das configurações
        // ====================================================================
        info!("✅ Configurações carregadas!");

        // Verifica se a API key do DeepL está configurada
        if deepl_api_key == "fake-api-key" {
            info!("   🌐 DeepL: ❌ Não configurado (modo fake)");
        } else {
            // Mostra apenas os primeiros caracteres da key por segurança
            let masked_key = format!("{}...", &deepl_api_key[..8.min(deepl_api_key.len())]);
            info!("   🌐 DeepL: ✅ Configurado ({})", masked_key);
        }

        // ElevenLabs é opcional por enquanto
        if elevenlabs_api_key.is_empty() {
            info!("   🔊 ElevenLabs: ⏸️  Não configurado (será implementado depois)");
        } else {
            info!("   🔊 ElevenLabs: ✅ Configurado");
        }

        // ====================================================================
        // PASSO 5: Ler configurações de captura de tela
        // ====================================================================
        let use_region_capture = env::var("USE_REGION_CAPTURE")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let region_x = env::var("REGION_X")
            .unwrap_or_else(|_| "0".to_string())
            .parse::<u32>()
            .unwrap_or(0);

        let region_y = env::var("REGION_Y")
            .unwrap_or_else(|_| "0".to_string())
            .parse::<u32>()
            .unwrap_or(0);

        let region_width = env::var("REGION_WIDTH")
            .unwrap_or_else(|_| "1920".to_string())
            .parse::<u32>()
            .unwrap_or(1920);

        let region_height = env::var("REGION_HEIGHT")
            .unwrap_or_else(|_| "1080".to_string())
            .parse::<u32>()
            .unwrap_or(1080);

        // Mostra modo de captura
        if use_region_capture {
            info!(
                "   📸 Captura: 🎯 Região customizada ({}x{} na posição {},{}",
                region_width, region_height, region_x, region_y
            );
        } else {
            info!("   📸 Captura: 🖥️  Tela inteira");
        }

        // ====================================================================
        // PASSO 6: Retornar a configuração
        // ====================================================================
        Ok(Config {
            deepl_api_key,
            elevenlabs_api_key,
            elevenlabs_voice_id,
            use_region_capture,
            region_x,
            region_y,
            region_width,
            region_height,
        })
    }
}
