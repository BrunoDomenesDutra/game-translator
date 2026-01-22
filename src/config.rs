// game-translator/src/config.rs

// ============================================================================
// MÓDULO CONFIG - Configurações da aplicação
// ============================================================================

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

/// Estrutura de configuração da região de captura
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConfig {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Default for RegionConfig {
    fn default() -> Self {
        RegionConfig {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }
}

/// Estrutura de configuração do overlay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub background_type: String,
    pub background_color: [u8; 4],
    pub background_image_path: String,
    /// Se true, mostra fundo preto semi-transparente. Se false, só texto com contorno.
    pub show_background: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        OverlayConfig {
            x: 400,
            y: 100,
            width: 1200,
            height: 200,
            background_type: "solid".to_string(),
            background_color: [0, 0, 0, 235],
            background_image_path: "backgrounds/custom.png".to_string(),
            show_background: false, // Padrão: só texto com contorno
        }
    }
}

/// Estrutura de configuração das hotkeys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub translate_fullscreen: String,
    pub translate_region: String,
    pub select_region: String,
    pub select_subtitle_region: String,
    pub toggle_subtitle_mode: String,
    pub hide_translation: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        HotkeyConfig {
            translate_fullscreen: "NumpadSubtract".to_string(),
            translate_region: "NumpadAdd".to_string(),
            select_region: "NumpadMultiply".to_string(),
            select_subtitle_region: "NumpadDivide".to_string(),
            toggle_subtitle_mode: "Numpad0".to_string(),
            hide_translation: "NumpadDecimal".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowConfig {
    pub enabled: bool,
    pub offset_x: i32,
    pub offset_y: i32,
    pub color: [u8; 4], // RGBA
    pub blur: u32,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        ShadowConfig {
            enabled: false,
            offset_x: 2,
            offset_y: 2,
            color: [0, 0, 0, 180],
            blur: 0,
        }
    }
}

/// Configuração de contorno do texto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineConfig {
    pub enabled: bool,
    pub width: u32,
    pub color: [u8; 4], // RGBA
}

impl Default for OutlineConfig {
    fn default() -> Self {
        OutlineConfig {
            enabled: false,
            width: 2,
            color: [0, 0, 0, 255],
        }
    }
}

/// Configuração completa de fonte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub font_type: String, // "system", "file", "embedded"
    pub system_font_name: String,
    pub file_path: String,
    pub size: f32,
    pub color: [u8; 4], // RGBA
    pub shadow: ShadowConfig,
    pub outline: OutlineConfig,
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig {
            font_type: "system".to_string(),
            system_font_name: "Arial".to_string(),
            file_path: "fonts/default.ttf".to_string(),
            size: 32.0,
            color: [255, 255, 255, 255],
            shadow: ShadowConfig::default(),
            outline: OutlineConfig::default(),
        }
    }
}

/// Estrutura de configuração de exibição
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub overlay_duration_secs: u64,
    /// Se true, processa OCR direto da memória (mais rápido)
    /// Se false, salva screenshot em disco primeiro (mais lento, mas útil para debug)
    pub use_memory_capture: bool,
    /// Se true, ativa Text-to-Speech (ElevenLabs)
    /// Se false, apenas exibe tradução sem áudio
    pub tts_enabled: bool,
}

/// Estrutura de configuração de tradução
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    /// Provedor de tradução: "deepl", "google" ou "libretranslate"
    pub provider: String,
    /// Idioma de origem (ex: "EN", "JA", "auto")
    pub source_language: String,
    /// Idioma de destino (ex: "PT-BR", "PT", "ES")
    pub target_language: String,
    /// URL do LibreTranslate (se usar LibreTranslate local) ← NOVO!
    #[serde(default = "default_libretranslate_url")]
    pub libretranslate_url: String,
}

/// URL padrão do LibreTranslate
fn default_libretranslate_url() -> String {
    "http://localhost:5000".to_string()
}

impl Default for TranslationConfig {
    fn default() -> Self {
        TranslationConfig {
            provider: "libretranslate".to_string(), // Padrão: LibreTranslate local
            source_language: "EN".to_string(),
            target_language: "PT-BR".to_string(),
            libretranslate_url: "http://localhost:5000".to_string(), // ← NOVO!
        }
    }
}

/// Estrutura de configuração de legendas em tempo real
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleConfig {
    /// Região onde as legendas originais aparecem no jogo
    pub region: RegionConfig,
    /// Intervalo entre capturas em milissegundos
    pub capture_interval_ms: u64,
    /// Tempo mínimo de exibição da tradução (segundos)
    pub min_display_secs: u64,
    /// Tempo máximo de exibição da tradução (segundos)
    pub max_display_secs: u64,
    /// Configuração de fonte específica para legendas
    pub font: FontConfig,
}

impl Default for SubtitleConfig {
    fn default() -> Self {
        SubtitleConfig {
            region: RegionConfig {
                x: 400,
                y: 900,
                width: 1200,
                height: 100,
            },
            capture_interval_ms: 1000,
            min_display_secs: 2,
            max_display_secs: 10,
            font: FontConfig {
                font_type: "system".to_string(),
                system_font_name: "Arial".to_string(),
                file_path: "fonts/Font.ttf".to_string(),
                size: 24.0, // Menor que o padrão para legendas
                color: [255, 255, 255, 255],
                shadow: ShadowConfig::default(),
                outline: OutlineConfig::default(),
            },
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        DisplayConfig {
            overlay_duration_secs: 5,
            use_memory_capture: true,
            tts_enabled: true, // TTS ligado por padrão
        }
    }
}

/// Estrutura principal de configuração
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub region: RegionConfig,
    pub overlay: OverlayConfig,
    pub font: FontConfig,
    pub hotkeys: HotkeyConfig,
    pub display: DisplayConfig,
    pub translation: TranslationConfig,
    pub subtitle: SubtitleConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            region: RegionConfig::default(),
            overlay: OverlayConfig::default(),
            font: FontConfig::default(),
            hotkeys: HotkeyConfig::default(),
            display: DisplayConfig::default(),
            translation: TranslationConfig::default(),
            subtitle: SubtitleConfig::default(), // <- ADICIONE ESTA LINHA
        }
    }
}

impl AppConfig {
    /// Caminho do arquivo de configuração
    const CONFIG_FILE: &'static str = "config.json";

    /// Carrega configurações do arquivo (ou cria um padrão se não existir)
    pub fn load() -> Result<Self> {
        info!("📋 Carregando configurações...");

        if Path::new(Self::CONFIG_FILE).exists() {
            // Carrega do arquivo existente
            let contents =
                fs::read_to_string(Self::CONFIG_FILE).context("Falha ao ler config.json")?;

            let config: AppConfig =
                serde_json::from_str(&contents).context("Falha ao parsear config.json")?;

            info!("✅ Configurações carregadas de config.json");
            info!(
                "   📍 Região: {}x{} na posição ({}, {})",
                config.region.width, config.region.height, config.region.x, config.region.y
            );
            info!(
                "   🖼️  Overlay: {}x{} na posição ({}, {})",
                config.overlay.width, config.overlay.height, config.overlay.x, config.overlay.y
            );

            Ok(config)
        } else {
            // Cria arquivo padrão
            warn!("⚠️  config.json não encontrado, criando arquivo padrão...");
            let config = AppConfig::default();
            config.save()?;
            info!("✅ config.json criado com valores padrão");
            Ok(config)
        }
    }

    /// Salva configurações no arquivo
    pub fn save(&self) -> Result<()> {
        info!("💾 Salvando configurações...");

        let json =
            serde_json::to_string_pretty(self).context("Falha ao serializar configurações")?;

        fs::write(Self::CONFIG_FILE, json).context("Falha ao escrever config.json")?;

        info!("✅ Configurações salvas em config.json");

        Ok(())
    }

    /// Atualiza a região de captura e salva
    pub fn update_region(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<()> {
        info!("🔄 Atualizando região de captura...");

        self.region.x = x;
        self.region.y = y;
        self.region.width = width;
        self.region.height = height;

        self.save()?;

        info!(
            "✅ Região atualizada: {}x{} na posição ({}, {})",
            width, height, x, y
        );

        Ok(())
    }

    /// Atualiza a posição e tamanho do overlay e salva
    pub fn update_overlay(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<()> {
        info!("🔄 Atualizando configuração do overlay...");

        self.overlay.x = x;
        self.overlay.y = y;
        self.overlay.width = width;
        self.overlay.height = height;

        self.save()?;

        info!(
            "✅ Overlay atualizado: {}x{} na posição ({}, {})",
            width, height, x, y
        );

        Ok(())
    }
}

/// Estrutura que guarda todas as configurações da aplicação (compatibilidade)
#[derive(Debug, Clone)]
pub struct Config {
    /// API key do DeepL para tradução
    pub deepl_api_key: String,

    /// API key do ElevenLabs para TTS
    pub elevenlabs_api_key: String,

    /// ID da voz no ElevenLabs
    pub elevenlabs_voice_id: String,

    /// Configurações da aplicação
    pub app_config: AppConfig,

    // Atalhos para acessar facilmente (retrocompatibilidade)
    pub use_region_capture: bool,
    pub region_x: u32,
    pub region_y: u32,
    pub region_width: u32,
    pub region_height: u32,
}

impl Config {
    /// Carrega as configurações completas
    pub fn load() -> Result<Self> {
        info!("📋 Carregando configurações completas...");

        // Carrega variáveis de ambiente (.env)
        dotenv::dotenv().ok();

        // API keys do .env
        let deepl_api_key = env::var("DEEPL_API_KEY").unwrap_or_else(|_| {
            warn!("⚠️  DEEPL_API_KEY não configurada no arquivo .env");
            "fake-api-key".to_string()
        });

        let elevenlabs_api_key = env::var("ELEVENLABS_API_KEY").unwrap_or_else(|_| String::new());

        let elevenlabs_voice_id = env::var("ELEVENLABS_VOICE_ID").unwrap_or_else(|_| String::new());

        // Carrega config.json
        let app_config = AppConfig::load()?;

        info!("✅ Configurações carregadas!");

        // Status das API keys
        if deepl_api_key == "fake-api-key" {
            info!("   🌐 DeepL: ❌ Não configurado (modo fake)");
        } else {
            let masked_key = format!("{}...", &deepl_api_key[..8.min(deepl_api_key.len())]);
            info!("   🌐 DeepL: ✅ Configurado ({})", masked_key);
        }

        if elevenlabs_api_key.is_empty() {
            info!("   🔊 ElevenLabs: ⏸️  Não configurado");
        } else {
            info!("   🔊 ElevenLabs: ✅ Configurado");
        }

        info!(
            "   📸 Captura: 🎯 Região customizada ({}x{} na posição {},{})",
            app_config.region.width,
            app_config.region.height,
            app_config.region.x,
            app_config.region.y
        );

        Ok(Config {
            deepl_api_key,
            elevenlabs_api_key,
            elevenlabs_voice_id,

            // Atalhos para retrocompatibilidade
            use_region_capture: true,
            region_x: app_config.region.x,
            region_y: app_config.region.y,
            region_width: app_config.region.width,
            region_height: app_config.region.height,

            app_config,
        })
    }

    /// Atualiza a região e salva
    pub fn update_region(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<()> {
        self.app_config.update_region(x, y, width, height)?;

        // Atualiza atalhos
        self.region_x = x;
        self.region_y = y;
        self.region_width = width;
        self.region_height = height;

        Ok(())
    }
}
