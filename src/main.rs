// ============================================================================
// GAME TRANSLATOR - Aplicação para traduzir textos de jogos em tempo real
// ============================================================================

#[macro_use]
extern crate log;

// ============================================================================
// DECLARAÇÃO DE MÓDULOS
// ============================================================================
mod config;
mod hotkey;
mod ocr;
mod overlay;
mod screenshot;
mod translator;
mod tts;

// ============================================================================
// IMPORTS
// ============================================================================
use anyhow::Result;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use config::Config;
use ocr::extract_text;
use overlay::show_overlay;
use translator::translate;

// ============================================================================
// FUNÇÃO PRINCIPAL
// ============================================================================
#[tokio::main]
async fn main() -> Result<()> {
    // Inicializa o sistema de logs
    env_logger::init();

    info!("🎮 ============================================");
    info!("🎮 GAME TRANSLATOR - Tradutor para Jogos");
    info!("🎮 ============================================");
    info!("");
    info!("📋 Configurações:");
    info!("   🎯 Jogo: Judgment (Yakuza)");
    info!("   🌐 Tradução: DeepL (EN → PT-BR)");
    info!("   🔊 Voz: ElevenLabs");
    info!("   📸 Modo: Tela inteira");
    info!("   ⌨️  Hotkeys:");
    info!("      - Numpad - (menos) = Tela inteira");
    info!("      - Numpad + (mais)  = Região customizada");
    info!("");

    info!("⚙️  Configurando sistema...");

    // Carrega configurações (API keys do arquivo .env)
    let config = Config::load()?;

    // Cria o gerenciador de hotkeys
    let hotkey_manager = hotkey::HotkeyManager::new();

    // ========================================================================
    // INICIA O OVERLAY PERMANENTE
    // ========================================================================
    info!("🖼️  Iniciando overlay permanente...");

    // Calcula posição do overlay baseado nas coordenadas da região
    // O overlay vai aparecer logo acima da região de captura
    let overlay_x = config.region_x as f32;
    let overlay_y = (config.region_y - 250) as f32; // 250 pixels acima da legenda
    let overlay_width = config.region_width as f32;
    let overlay_height = 200.0; // Altura fixa do overlay

    let overlay_channel =
        overlay::start_overlay(overlay_x, overlay_y, overlay_width, overlay_height)?;

    info!("✅ Overlay pronto!");

    info!("✅ Sistema pronto!");
    info!("");
    info!("🎯 Pressione Numpad - para capturar TELA INTEIRA");
    info!("🎯 Pressione Numpad + para capturar REGIÃO customizada");
    info!("🎯 Pressione Ctrl+C para sair");
    info!("🎯 Pressione Ctrl+C para sair");
    info!("");

    // ========================================================================
    // LOOP PRINCIPAL - Verifica a tecla continuamente
    // ========================================================================
    loop {
        // Verifica se alguma hotkey foi pressionada
        if let Some(capture_mode) = hotkey_manager.check_hotkey() {
            info!("");
            info!("▶️  ============================================");

            // Mostra qual modo foi ativado
            match capture_mode {
                hotkey::CaptureMode::FullScreen => {
                    info!("▶️  MODO: 🖥️  TELA INTEIRA");
                }
                hotkey::CaptureMode::Region => {
                    info!("▶️  MODO: 🎯 REGIÃO CUSTOMIZADA");
                }
            }

            info!("▶️  ============================================");

            // Processa a tradução com o modo escolhido
            if let Err(e) = process_translation(&config, capture_mode, &overlay_channel).await {
                error!("❌ Erro durante o processo: {}", e);
            }

            info!("▶️  ============================================");
            info!("▶️  Pronto! Aguardando próxima ativação...");
            info!("▶️  ============================================");
            info!("");

            // Aguarda a tecla ser solta antes de continuar
            hotkey_manager.wait_for_key_release();
        }

        // Pausa pequena para não consumir 100% da CPU
        thread::sleep(Duration::from_millis(50));
    }
}

// ============================================================================
// FUNÇÃO DE PROCESSAMENTO
// ============================================================================
async fn process_translation(
    config: &Config,
    capture_mode: hotkey::CaptureMode,
    overlay_channel: &overlay::OverlayChannel,
) -> Result<()> {
    info!("📸 [1/5] Capturando tela...");

    let screenshot_path = PathBuf::from("screenshot.png");

    // Decide qual modo de captura usar baseado na hotkey pressionada
    let _image = match capture_mode {
        hotkey::CaptureMode::Region => {
            // Modo: Captura apenas a região customizada
            info!(
                "   🎯 Capturando região: {}x{} na posição ({}, {})",
                config.region_width, config.region_height, config.region_x, config.region_y
            );
            screenshot::capture_region(
                &screenshot_path,
                config.region_x,
                config.region_y,
                config.region_width,
                config.region_height,
            )?
        }
        hotkey::CaptureMode::FullScreen => {
            // Modo: Captura a tela inteira
            info!("   🖥️  Capturando tela inteira");
            screenshot::capture_screen(&screenshot_path)?
        }
    };

    info!("✅ Screenshot capturada!");

    info!("🔍 [2/5] Executando OCR...");

    let extracted_text = extract_text(&screenshot_path)?;

    if extracted_text.is_empty() {
        info!("⚠️  Nenhum texto detectado na imagem!");
        info!("💡 Dica: Certifique-se de que há texto visível no jogo");
        return Ok(());
    }

    info!("✅ Texto extraído:");
    info!("   📝 {}", extracted_text);

    info!("🌐 [3/5] Traduzindo texto...");

    // Por enquanto, tradução fake
    let translated_text = translate(&extracted_text, &config.deepl_api_key).await?;

    info!("✅ Texto traduzido:");
    info!("   🇧🇷 {}", translated_text);

    info!("🖼️  [4/5] Enviando tradução para overlay...");
    overlay_channel.show_text(translated_text.clone())?;
    info!("✅ Tradução enviada ao overlay!");
    info!("✅ Overlay exibido!");

    info!("🔊 [5/5] Sintetizando voz...");
    info!("⚠️  TTS desabilitado temporariamente");

    info!("✅ Processo completo!");

    Ok(())
}
