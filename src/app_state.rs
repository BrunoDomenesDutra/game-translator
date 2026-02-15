// game-translator/src/app_state.rs

// ============================================================================
// MÓDULO APP STATE - Estado compartilhado da aplicação
// ============================================================================
// Contém as estruturas de estado que são compartilhadas entre threads:
// - AppState: estado principal
// - CaptureRegion: região capturada
// - CaptureMode: modo de captura (região/tela cheia)
// - AppCommand: comandos entre threads
// ============================================================================

use crossbeam_channel::Sender;
use std::sync::{Arc, Mutex};

use crate::cache;
use crate::config::Config;
use crate::ocr::TranslatedText;
use crate::subtitle;

// ============================================================================
// COMANDOS ENTRE THREADS
// ============================================================================

/// Comandos que podem ser enviados da thread de hotkeys para a main thread
#[derive(Debug, Clone)]
pub enum AppCommand {
    /// Abre o seletor de região
    OpenRegionSelector,
    /// Abre o seletor de região de legendas
    OpenSubtitleRegionSelector,
    /// Abre a janela de configurações
    OpenSettings,
    /// Fecha a janela de configurações
    CloseSettings,
}

// ============================================================================
// ESTRUTURAS DE CAPTURA
// ============================================================================

/// Região onde o texto foi capturado
#[derive(Clone, Debug)]
pub struct CaptureRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Modo de captura (afeta como o overlay renderiza)
#[derive(Clone, Debug, PartialEq)]
pub enum CaptureMode {
    /// Captura de região específica - exibe texto combinado na região
    Region,
    /// Captura de tela inteira - exibe cada texto na posição original
    FullScreen,
}

// ============================================================================
// ESTADO COMPARTILHADO
// ============================================================================

/// Estado compartilhado entre a UI (overlay) e as threads de background
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub translated_items: Arc<Mutex<Vec<TranslatedText>>>,
    pub capture_region: Arc<Mutex<Option<CaptureRegion>>>,
    pub capture_mode: Arc<Mutex<CaptureMode>>,
    pub translation_timestamp: Arc<Mutex<Option<std::time::Instant>>>,
    pub command_sender: Sender<AppCommand>,
    /// Cache de traduções
    pub translation_cache: cache::TranslationCache,
    /// Indica se o modo legenda está ativo
    pub subtitle_mode_active: Arc<Mutex<bool>>,
    /// Estado do sistema de legendas
    pub subtitle_state: subtitle::SubtitleState,
    /// Controla se o overlay deve ficar escondido (durante captura)
    pub overlay_hidden: Arc<Mutex<bool>>,
    /// Controla se está no modo de configurações
    pub settings_mode: Arc<Mutex<bool>>,
    /// Fator de escala DPI (ex: 1.25 para 125%)
    pub dpi_scale: f32,
    /// Contador de requests OpenAI na sessão atual
    pub openai_request_count: Arc<Mutex<u32>>,
    /// Flag que indica que as hotkeys precisam ser recarregadas
    pub hotkeys_need_reload: Arc<Mutex<bool>>,
}

impl AppState {
    pub fn new(config: Config, command_sender: Sender<AppCommand>, dpi_scale: f32) -> Self {
        // Cria cache com persistência em disco
        let translation_cache = cache::TranslationCache::new(true);

        // Cria estado de legendas com configurações do config
        let subtitle_state = subtitle::SubtitleState::new(
            config.app_config.subtitle.min_display_secs,
            config.app_config.subtitle.max_display_secs,
        );

        AppState {
            config: Arc::new(Mutex::new(config)),
            translated_items: Arc::new(Mutex::new(Vec::new())),
            capture_region: Arc::new(Mutex::new(None)),
            capture_mode: Arc::new(Mutex::new(CaptureMode::Region)),
            translation_timestamp: Arc::new(Mutex::new(None)),
            command_sender,
            translation_cache,
            subtitle_mode_active: Arc::new(Mutex::new(false)),
            subtitle_state,
            overlay_hidden: Arc::new(Mutex::new(false)),
            settings_mode: Arc::new(Mutex::new(false)),
            dpi_scale,
            openai_request_count: Arc::new(Mutex::new(0)),
            hotkeys_need_reload: Arc::new(Mutex::new(false)),
        }
    }

    /// Define a lista de textos traduzidos com posições, região e modo de captura
    pub fn set_translations(
        &self,
        items: Vec<TranslatedText>,
        region: CaptureRegion,
        mode: CaptureMode,
    ) {
        *self.translated_items.lock().unwrap() = items;
        *self.capture_region.lock().unwrap() = Some(region);
        *self.capture_mode.lock().unwrap() = mode;
        *self.translation_timestamp.lock().unwrap() = Some(std::time::Instant::now());
    }

    /// Obtém a lista de traduções, região, modo e timestamp
    pub fn get_translations(
        &self,
    ) -> Option<(
        Vec<TranslatedText>,
        CaptureRegion,
        CaptureMode,
        std::time::Instant,
    )> {
        let items = self.translated_items.lock().unwrap().clone();
        let region = self.capture_region.lock().unwrap().clone()?;
        let mode = self.capture_mode.lock().unwrap().clone();
        let timestamp = self.translation_timestamp.lock().unwrap().clone()?;

        if items.is_empty() {
            return None;
        }

        Some((items, region, mode, timestamp))
    }

    /// Limpa as traduções
    pub fn clear_translations(&self) {
        *self.translated_items.lock().unwrap() = Vec::new();
        *self.capture_region.lock().unwrap() = None;
        *self.translation_timestamp.lock().unwrap() = None;
    }
}
