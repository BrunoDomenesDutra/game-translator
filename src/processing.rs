// game-translator/src/processing.rs

// ============================================================================
// MÓDULO PROCESSING - Processamento de tradução
// ============================================================================
// Contém as funções que fazem o pipeline completo:
// captura → OCR → tradução → overlay (modo região/tela cheia)
// e o pipeline simplificado para legendas.
// ============================================================================

use anyhow::Result;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::app_state::{AppState, CaptureMode, CaptureRegion};
use crate::hotkey;
use crate::ocr;
use crate::screenshot;
use crate::translator;
use crate::tts;

// ============================================================================
// TRADUÇÃO MODO REGIÃO / TELA CHEIA
// ============================================================================

/// Processa o pipeline completo de tradução:
/// 1. Esconde overlay
/// 2. Captura tela (região ou tela cheia)
/// 3. Pré-processamento da imagem
/// 4. OCR para detectar textos
/// 5. Tradução via API (com cache)
/// 6. Exibe no overlay
/// 7. TTS opcional
pub fn process_translation_blocking(state: &AppState, action: hotkey::HotkeyAction) -> Result<()> {
    // === ESCONDE O OVERLAY ANTES DE CAPTURAR ===
    {
        let mut hidden = state.overlay_hidden.lock().unwrap();
        *hidden = true;
    }
    thread::sleep(Duration::from_millis(100));

    // Verifica se usa modo memória (mais rápido) ou arquivo (debug)
    let use_memory = state
        .config
        .lock()
        .unwrap()
        .app_config
        .display
        .use_memory_capture;

    info!("📸 [1/4] Capturando tela...");

    // Pega configurações de pré-processamento
    let preprocess_config = {
        let config = state.config.lock().unwrap();
        config.app_config.display.preprocess.clone()
    };

    // No modo tela cheia, força upscale 1.0 (desativado)
    // porque a imagem já é grande e upscale deixaria muito lento
    let effective_upscale = if action == hotkey::HotkeyAction::TranslateFullScreen {
        1.0
    } else {
        preprocess_config.upscale
    };

    // OCR result vai ser preenchido de acordo com o modo
    let mut ocr_result = if use_memory {
        // ====================================================================
        // MODO MEMÓRIA (RÁPIDO) - Não salva arquivo em disco
        // ====================================================================
        let image = match action {
            hotkey::HotkeyAction::TranslateRegion => {
                let (x, y, w, h) = {
                    let config = state.config.lock().unwrap();
                    (
                        config.region_x,
                        config.region_y,
                        config.region_width,
                        config.region_height,
                    )
                };
                info!("   🎯 Região: {}x{} em ({}, {}) [MEMÓRIA]", w, h, x, y);
                screenshot::capture_region_to_memory(x, y, w, h)?
            }
            hotkey::HotkeyAction::TranslateFullScreen => {
                info!("   🖥️  Tela inteira [MEMÓRIA]");
                screenshot::capture_screen_to_memory()?
            }
            hotkey::HotkeyAction::SelectRegion
            | hotkey::HotkeyAction::SelectSubtitleRegion
            | hotkey::HotkeyAction::ToggleSubtitleMode
            | hotkey::HotkeyAction::HideTranslation
            | hotkey::HotkeyAction::OpenSettings => {
                anyhow::bail!("Esta ação não deveria chamar process_translation")
            }
        };

        // Aplica pré-processamento se habilitado
        let processed_image = if preprocess_config.enabled {
            screenshot::preprocess_image(
                &image,
                preprocess_config.grayscale,
                preprocess_config.invert,
                preprocess_config.contrast,
                preprocess_config.threshold,
                preprocess_config.save_debug_image,
                effective_upscale,
                preprocess_config.blur,
                preprocess_config.dilate,
                preprocess_config.erode,
                preprocess_config.edge_detection,
            )
        } else {
            image
        };

        info!("✅ Screenshot capturada em memória!");
        info!("🔍 [2/4] Executando OCR...");
        ocr::extract_text_from_memory(&processed_image)?
    } else {
        // ====================================================================
        // MODO ARQUIVO (DEBUG) - Salva screenshot.png em disco
        // ====================================================================
        let screenshot_path = PathBuf::from("screenshot.png");

        match action {
            hotkey::HotkeyAction::TranslateRegion => {
                let (x, y, w, h) = {
                    let config = state.config.lock().unwrap();
                    (
                        config.region_x,
                        config.region_y,
                        config.region_width,
                        config.region_height,
                    )
                };
                info!("   🎯 Região: {}x{} em ({}, {}) [ARQUIVO]", w, h, x, y);
                screenshot::capture_region(&screenshot_path, x, y, w, h)?;
            }
            hotkey::HotkeyAction::TranslateFullScreen => {
                info!("   🖥️  Tela inteira [ARQUIVO]");
                screenshot::capture_screen(&screenshot_path)?;
            }
            hotkey::HotkeyAction::SelectRegion => {
                anyhow::bail!("SelectRegion não deveria chamar process_translation")
            }
            hotkey::HotkeyAction::SelectSubtitleRegion
            | hotkey::HotkeyAction::ToggleSubtitleMode
            | hotkey::HotkeyAction::HideTranslation
            | hotkey::HotkeyAction::OpenSettings => {
                unreachable!("Esta ação não deveria chamar process_translation")
            }
        };

        info!("✅ Screenshot capturada!");
        info!("🔍 [2/4] Executando OCR...");
        ocr::extract_text_with_positions(&screenshot_path)?
    };

    if ocr_result.lines.is_empty() {
        info!("⚠️  Nenhum texto detectado!");
        // Mostra overlay de novo antes de sair
        *state.overlay_hidden.lock().unwrap() = false;
        return Ok(());
    }

    info!("   📍 {} linhas detectadas", ocr_result.lines.len());

    // Se upscale foi aplicado, corrige coordenadas dividindo de volta
    let upscale_factor = if preprocess_config.enabled
        && preprocess_config.upscale > 1.0
        && action != hotkey::HotkeyAction::TranslateFullScreen
    {
        preprocess_config.upscale as f64
    } else {
        1.0
    };

    if upscale_factor > 1.0 {
        info!(
            "   📐 Corrigindo coordenadas (÷{:.1}x upscale)",
            upscale_factor
        );
        for line in &mut ocr_result.lines {
            line.x /= upscale_factor;
            line.y /= upscale_factor;
            line.width /= upscale_factor;
            line.height /= upscale_factor;
        }
    }

    // Extrai textos para traduzir e limpa erros de OCR
    let texts_to_translate: Vec<String> = ocr_result
        .lines
        .iter()
        .map(|line| ocr::clean_ocr_text(&line.text))
        .collect();

    // Tradução em batch
    info!("🌐 [3/4] Traduzindo {} textos...", texts_to_translate.len());

    let (api_key, provider, source_lang, target_lang, libre_url, openai_config) = {
        let config = state.config.lock().unwrap();
        (
            config.app_config.translation.deepl_api_key.clone(),
            config.app_config.translation.provider.clone(),
            config.app_config.translation.source_language.clone(),
            config.app_config.translation.target_language.clone(),
            config.app_config.translation.libretranslate_url.clone(),
            config.app_config.openai.clone(),
        )
    };

    // Verifica quais textos já estão no cache
    let (cached, not_cached) = state.translation_cache.get_batch(
        &provider,
        &source_lang,
        &target_lang,
        &texts_to_translate,
    );

    info!(
        "   📦 Cache: {} encontrados, {} novos",
        cached.len(),
        not_cached.len()
    );

    // Prepara vetor de resultados
    let mut translated_texts: Vec<String> = vec![String::new(); texts_to_translate.len()];

    // Preenche com os que estavam no cache
    for (index, translated) in &cached {
        translated_texts[*index] = translated.clone();
    }

    // Traduz apenas os que não estavam no cache
    if !not_cached.is_empty() {
        let texts_to_api: Vec<String> = not_cached.iter().map(|(_, t)| t.clone()).collect();

        // Verifica se OpenAI atingiu limite de requests na sessão
        let effective_provider = if provider == "openai"
            && openai_config.max_requests_per_session > 0
        {
            let count = *state.openai_request_count.lock().unwrap();
            if count >= openai_config.max_requests_per_session {
                info!(
                    "⚠️  OpenAI atingiu limite ({}/{}), usando fallback: {}",
                    count, openai_config.max_requests_per_session, openai_config.fallback_provider
                );
                openai_config.fallback_provider.clone()
            } else {
                provider.clone()
            }
        } else {
            provider.clone()
        };

        let runtime = tokio::runtime::Runtime::new()?;
        let new_translations = runtime.block_on(async {
            translator::translate_batch_with_provider(
                &texts_to_api,
                &effective_provider,
                &api_key,
                &source_lang,
                &target_lang,
                Some(&libre_url),
                Some(&openai_config),
            )
            .await
        })?;

        // Incrementa contador se usou OpenAI
        if effective_provider == "openai" {
            *state.openai_request_count.lock().unwrap() += 1;
            info!(
                "📊 OpenAI requests: {}/{}",
                *state.openai_request_count.lock().unwrap(),
                if openai_config.max_requests_per_session == 0 {
                    "∞".to_string()
                } else {
                    openai_config.max_requests_per_session.to_string()
                }
            );
        }

        // Preenche os resultados e adiciona ao cache
        let mut cache_pairs: Vec<(String, String)> = Vec::new();

        for (i, (original_index, original_text)) in not_cached.iter().enumerate() {
            if let Some(translated) = new_translations.get(i) {
                translated_texts[*original_index] = translated.clone();
                cache_pairs.push((original_text.clone(), translated.clone()));
            }
        }

        // Salva no cache
        state
            .translation_cache
            .set_batch(&provider, &source_lang, &target_lang, &cache_pairs);

        // Salva cache em disco periodicamente
        let _ = state.translation_cache.save_to_disk();
    }

    let (cache_total, cache_size) = state.translation_cache.stats();
    info!(
        "✅ Tradução concluída! (Cache: {} entradas, {} bytes)",
        cache_total, cache_size
    );

    // Monta lista com posições
    let (offset_x, offset_y) = match action {
        hotkey::HotkeyAction::TranslateRegion => {
            let config = state.config.lock().unwrap();
            (config.region_x as f64, config.region_y as f64)
        }
        hotkey::HotkeyAction::TranslateFullScreen => (0.0, 0.0),
        _ => (0.0, 0.0),
    };

    let translated_items: Vec<ocr::TranslatedText> = ocr_result
        .lines
        .iter()
        .zip(translated_texts.iter())
        .map(|(detected, translated)| ocr::TranslatedText {
            original: ocr::clean_ocr_text(&detected.text),
            translated: translated.clone(),
            screen_x: detected.x + offset_x,
            screen_y: detected.y + offset_y,
            width: detected.width,
            height: detected.height,
        })
        .collect();

    // Define a região de captura (para posicionar o overlay)
    let capture_region = match action {
        hotkey::HotkeyAction::TranslateRegion => {
            let config = state.config.lock().unwrap();
            CaptureRegion {
                x: config.region_x,
                y: config.region_y,
                width: config.region_width,
                height: config.region_height,
            }
        }
        hotkey::HotkeyAction::TranslateFullScreen => {
            let config = state.config.lock().unwrap();
            CaptureRegion {
                x: config.app_config.overlay.x,
                y: config.app_config.overlay.y,
                width: config.app_config.overlay.width,
                height: config.app_config.overlay.height,
            }
        }
        _ => unreachable!(),
    };

    // Define o modo baseado na ação
    let capture_mode = match action {
        hotkey::HotkeyAction::TranslateFullScreen => CaptureMode::FullScreen,
        hotkey::HotkeyAction::TranslateRegion => CaptureMode::Region,
        _ => CaptureMode::Region,
    };

    // Envia para o overlay
    info!("🖼️  [4/4] Exibindo traduções...");
    state.set_translations(translated_items, capture_region, capture_mode);

    // ========================================================================
    // TTS - Fala a tradução (se configurado)
    // ========================================================================
    let (elevenlabs_key, elevenlabs_voice, tts_enabled) = {
        let config = state.config.lock().unwrap();
        let key = config.app_config.translation.elevenlabs_api_key.clone();
        let voice = config.app_config.translation.elevenlabs_voice_id.clone();
        (
            key.clone(),
            voice.clone(),
            config.app_config.display.tts_enabled && !key.is_empty() && !voice.is_empty(),
        )
    };

    if tts_enabled {
        info!("🔊 [5/5] Sintetizando voz...");

        let text_to_speak: String = translated_texts
            .iter()
            .filter(|t| !t.is_empty())
            .cloned()
            .collect::<Vec<String>>()
            .join(" ");

        if !text_to_speak.is_empty() {
            let key = elevenlabs_key.clone();
            let voice = elevenlabs_voice.clone();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Err(e) = tts::speak(&text_to_speak, &key, &voice).await {
                        error!("❌ Erro no TTS: {}", e);
                    }
                });
            });
        }
    } else {
        info!("🔇 [5/5] TTS desabilitado");
    }

    info!("✅ Completo!");
    info!("");

    // === MOSTRA O OVERLAY DE NOVO ===
    {
        let mut hidden = state.overlay_hidden.lock().unwrap();
        *hidden = false;
    }

    Ok(())
}

// ============================================================================
// TRADUÇÃO DE LEGENDAS
// ============================================================================

/// Processa a tradução de uma legenda individual.
/// Pipeline simplificado: cache check → contexto → tradução → histórico
pub fn process_subtitle_translation(state: &AppState, text: &str) -> anyhow::Result<()> {
    info!("📺 Traduzindo legenda: \"{}\"", text);

    // Pega configurações de tradução (do app_config pra ter hot reload)
    let (api_key, provider, source_lang, target_lang, libre_url, openai_config) = {
        let config = state.config.lock().unwrap();
        (
            config.app_config.translation.deepl_api_key.clone(),
            config.app_config.translation.provider.clone(),
            config.app_config.translation.source_language.clone(),
            config.app_config.translation.target_language.clone(),
            config.app_config.translation.libretranslate_url.clone(),
            config.app_config.openai.clone(),
        )
    };

    // Verifica cache primeiro
    if let Some(cached) = state
        .translation_cache
        .get(&provider, &source_lang, &target_lang, text)
    {
        info!("   📦 Cache hit!");
        state.subtitle_state.add_translated_subtitle(cached);
        return Ok(());
    }

    // Pega contexto das últimas legendas traduzidas (para OpenAI)
    let context: Vec<String> = if provider == "openai" && openai_config.context_lines > 0 {
        let history = state.subtitle_state.get_subtitle_history();
        let n = openai_config.context_lines as usize;
        history
            .iter()
            .rev()
            .take(n)
            .map(|entry| entry.translated.clone())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    } else {
        Vec::new()
    };

    // Verifica se OpenAI atingiu limite de requests na sessão
    let effective_provider = if provider == "openai" && openai_config.max_requests_per_session > 0 {
        let count = *state.openai_request_count.lock().unwrap();
        if count >= openai_config.max_requests_per_session {
            info!(
                "⚠️  OpenAI limite atingido ({}/{}), fallback: {}",
                count, openai_config.max_requests_per_session, openai_config.fallback_provider
            );
            openai_config.fallback_provider.clone()
        } else {
            provider.clone()
        }
    } else {
        provider.clone()
    };

    // Traduz via API (com contexto de conversa se for OpenAI)
    let runtime = tokio::runtime::Runtime::new()?;
    let translated = runtime.block_on(async {
        translator::translate_with_context(
            &[text.to_string()],
            &effective_provider,
            &api_key,
            &source_lang,
            &target_lang,
            Some(&libre_url),
            Some(&openai_config),
            &context,
        )
        .await
    })?;

    // Incrementa contador se usou OpenAI
    if effective_provider == "openai" {
        *state.openai_request_count.lock().unwrap() += 1;
    }

    if let Some(translated_text) = translated.first() {
        info!("   ✅ Traduzido: \"{}\"", translated_text);

        // Salva no cache
        state
            .translation_cache
            .set(&provider, &source_lang, &target_lang, text, translated_text);

        // Adiciona ao histórico de legendas
        state
            .subtitle_state
            .add_translated_subtitle(translated_text.clone());
    }

    Ok(())
}

// ============================================================================
// THREAD DE LEGENDAS (captura contínua)
// ============================================================================

/// Inicia a thread que captura legendas continuamente quando o modo legenda está ativo.
/// Faz OCR na região de legenda, detecta mudanças de texto e dispara tradução.
pub fn start_subtitle_thread(state: AppState) {
    std::thread::spawn(move || {
        info!("📺 Thread de legendas iniciada (aguardando ativação)");

        loop {
            let timeout_secs: u64 = {
                let config = state.config.lock().unwrap();
                config.app_config.subtitle.max_display_secs
            };

            let is_active = *state.subtitle_mode_active.lock().unwrap();

            if is_active {
                // Verifica timeout (sem texto por X segundos)
                if state.subtitle_state.has_subtitles()
                    && state.subtitle_state.is_timed_out(timeout_secs)
                {
                    state.subtitle_state.reset();
                }

                // Pega configurações da região de legenda
                let (region_x, region_y, region_w, region_h, interval_ms) = {
                    let config = state.config.lock().unwrap();
                    (
                        config.app_config.subtitle.region.x,
                        config.app_config.subtitle.region.y,
                        config.app_config.subtitle.region.width,
                        config.app_config.subtitle.region.height,
                        config.app_config.subtitle.capture_interval_ms,
                    )
                };

                // Pega configurações de pré-processamento
                let preprocess_config = {
                    let config = state.config.lock().unwrap();
                    config.app_config.subtitle.preprocess.clone()
                };

                // Captura a região da legenda
                match screenshot::capture_region_to_memory(region_x, region_y, region_w, region_h) {
                    Ok(image) => {
                        let processed_image = if preprocess_config.enabled {
                            screenshot::preprocess_image(
                                &image,
                                preprocess_config.grayscale,
                                preprocess_config.invert,
                                preprocess_config.contrast,
                                preprocess_config.threshold,
                                preprocess_config.save_debug_image,
                                preprocess_config.upscale,
                                preprocess_config.blur,
                                preprocess_config.dilate,
                                preprocess_config.erode,
                                preprocess_config.edge_detection,
                            )
                        } else {
                            image
                        };

                        match ocr::extract_text_from_memory(&processed_image) {
                            Ok(ocr_result) => {
                                let full_text = ocr::clean_ocr_text(&ocr_result.full_text);

                                if full_text.len() >= 3 {
                                    state.subtitle_state.update_detection_time();
                                }

                                if let Some(text_to_translate) =
                                    state.subtitle_state.process_detected_text(&full_text)
                                {
                                    let state_clone = state.clone();
                                    std::thread::spawn(move || {
                                        if let Err(e) = process_subtitle_translation(
                                            &state_clone,
                                            &text_to_translate,
                                        ) {
                                            error!("❌ Erro ao traduzir legenda: {}", e);
                                        }
                                    });
                                }
                            }
                            Err(e) => {
                                trace!("OCR falhou: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ Erro ao capturar região de legenda: {}", e);
                    }
                }

                std::thread::sleep(Duration::from_millis(interval_ms));
            } else {
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    });
}
