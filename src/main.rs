#![windows_subsystem = "windows"]

// game-translator/src/main.rs

// ============================================================================
// GAME TRANSLATOR - Aplicação para traduzir textos de jogos em tempo real
// ============================================================================

#[macro_use]
extern crate log;

// ============================================================================
// DECLARAÇÃO DE MÓDULOS
// ============================================================================
mod app_state;
mod cache;
mod config;
mod hotkey;
mod ocr;
mod processing;
mod region_selector;
mod screenshot;
mod settings_ui;
mod subtitle;
mod translator;
mod tts;

// ============================================================================
// IMPORTS
// ============================================================================
use anyhow::Result;
use app_state::{AppCommand, AppState, CaptureMode};
use config::Config;
use crossbeam_channel::{unbounded, Receiver};
use notify::{RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

// ============================================================================
// APLICAÇÃO DE OVERLAY (roda na main thread)
// ============================================================================

struct OverlayApp {
    state: AppState,
    display_duration: Duration,
    command_receiver: Receiver<AppCommand>,
    /// Cópia local das configurações para edição
    settings_config: Option<config::AppConfig>,
    /// Aba atual das configurações
    settings_tab: u8,
    /// Mensagem de status
    settings_status: Option<(String, std::time::Instant)>,
    /// Se já posicionou a janela de configurações (evita forçar posição todo frame)
    settings_positioned: bool,
    /// Textura da imagem debug de pré-processamento (preview em tempo real)
    debug_texture: Option<eframe::egui::TextureHandle>,
    /// Quando a textura debug foi atualizada pela última vez
    debug_texture_last_update: std::time::Instant,
    /// Textura da imagem original do laboratório
    lab_original_texture: Option<eframe::egui::TextureHandle>,
    /// Textura da imagem processada do laboratório
    lab_processed_texture: Option<eframe::egui::TextureHandle>,
    /// Configuração de pré-processamento do laboratório (independente)
    lab_preprocess: Option<crate::config::PreprocessConfig>,
    /// Nome do arquivo selecionado no laboratório
    lab_selected_file: Option<String>,
    /// Imagem original carregada (pra não reler do disco toda hora)
    lab_original_image: Option<image::DynamicImage>,
    /// Flag que indica que os parâmetros mudaram e precisa reprocessar
    lab_needs_reprocess: bool,
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // Totalmente transparente
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // ====================================================================
        // TORNA A JANELA CLICK-THROUGH (apenas uma vez)
        // ====================================================================
        #[cfg(windows)]
        {
            // Verifica se está no modo configurações
            let is_settings = *self.state.settings_mode.lock().unwrap();

            if is_settings {
                // Modo configurações: remove click-through pra poder interagir
                remove_window_click_through();
            } else {
                // Modo normal: reaplica click-through periodicamente (a cada ~500ms)
                use std::sync::atomic::{AtomicU64, Ordering};
                static LAST_CLICK_THROUGH: AtomicU64 = AtomicU64::new(0);

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                let last = LAST_CLICK_THROUGH.load(Ordering::Relaxed);
                if now - last > 500 {
                    make_window_click_through();
                    LAST_CLICK_THROUGH.store(now, Ordering::Relaxed);
                }
            }
        }
        // ====================================================================
        // VERIFICA SE O OVERLAY DEVE FICAR ESCONDIDO (durante captura)
        // ====================================================================
        let is_hidden = *self.state.overlay_hidden.lock().unwrap();
        if is_hidden {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                eframe::egui::vec2(1.0, 1.0),
            ));
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            return;
        }
        // ====================================================================
        // PROCESSA COMANDOS RECEBIDOS
        // ====================================================================
        while let Ok(command) = self.command_receiver.try_recv() {
            match command {
                AppCommand::OpenRegionSelector => {
                    info!("🎯 Abrindo seletor de região...");

                    // Esconde o overlay temporariamente
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                        eframe::egui::vec2(1.0, 1.0),
                    ));

                    // Abre o seletor de região
                    match region_selector::select_region(None) {
                        Ok(Some(selected)) => {
                            info!(
                                "✅ Região selecionada: {}x{} na posição ({}, {})",
                                selected.width, selected.height, selected.x, selected.y
                            );

                            let mut config = self.state.config.lock().unwrap();
                            if let Err(e) = config.app_config.update_region(
                                selected.x,
                                selected.y,
                                selected.width,
                                selected.height,
                            ) {
                                error!("❌ Erro ao salvar região: {}", e);
                            } else {
                                info!("💾 Região salva no config.json!");
                                config.region_x = selected.x;
                                config.region_y = selected.y;
                                config.region_width = selected.width;
                                config.region_height = selected.height;
                            }
                        }
                        Ok(None) => info!("❌ Seleção cancelada"),
                        Err(e) => error!("❌ Erro no seletor: {}", e),
                    }
                }

                AppCommand::OpenSubtitleRegionSelector => {
                    info!("📺 Abrindo seletor de região de legendas...");

                    // Esconde o overlay temporariamente
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                        eframe::egui::vec2(1.0, 1.0),
                    ));

                    // Abre o seletor de região
                    match region_selector::select_region(Some("SELEÇÃO ÁREA DE LEGENDA")) {
                        Ok(Some(selected)) => {
                            info!(
                                "✅ Região de legendas selecionada: {}x{} na posição ({}, {})",
                                selected.width, selected.height, selected.x, selected.y
                            );

                            let mut config = self.state.config.lock().unwrap();
                            // Atualiza a região de legendas
                            config.app_config.subtitle.region.x = selected.x;
                            config.app_config.subtitle.region.y = selected.y;
                            config.app_config.subtitle.region.width = selected.width;
                            config.app_config.subtitle.region.height = selected.height;

                            // Salva no arquivo
                            if let Err(e) = config.app_config.save() {
                                error!("❌ Erro ao salvar região de legendas: {}", e);
                            } else {
                                info!("💾 Região de legendas salva no config.json!");
                            }
                        }
                        Ok(None) => info!("❌ Seleção cancelada"),
                        Err(e) => error!("❌ Erro no seletor: {}", e),
                    }
                }

                AppCommand::OpenSettings => {
                    info!("⚙️  Entrando no modo configurações...");

                    // Copia as configurações atuais para edição
                    let config = self.state.config.lock().unwrap();
                    self.settings_config = Some(config.app_config.clone());
                    drop(config);

                    // Ativa o modo configurações
                    *self.state.settings_mode.lock().unwrap() = true;
                    self.settings_tab = 0;
                    self.settings_status = None;
                }

                AppCommand::CloseSettings => {
                    info!("⚙️  Saindo do modo configurações...");

                    *self.state.settings_mode.lock().unwrap() = false;
                    self.settings_config = None;
                    self.settings_positioned = false; // Reseta pra próxima abertura

                    // Restaura janela para modo overlay
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Decorations(false));
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Resizable(false));
                }
            }
        }

        // ====================================================================
        // MODO CONFIGURAÇÕES - Janela de edição
        // ====================================================================
        let is_settings_mode = *self.state.settings_mode.lock().unwrap();

        if is_settings_mode {
            // Na primeira vez que abre as configs, centraliza e define tamanho inicial.
            // Depois disso, não força mais posição/tamanho — deixa o usuário mover e redimensionar.
            // Usamos uma flag estática pra saber se já posicionou.
            {
                if !self.settings_positioned {
                    self.settings_positioned = true;

                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Decorations(true));
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Resizable(true));
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                        eframe::egui::vec2(600.0, 700.0),
                    ));

                    let screen_w = unsafe {
                        winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN)
                    } as f32
                        / self.state.dpi_scale;
                    let screen_h = unsafe {
                        winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN)
                    } as f32
                        / self.state.dpi_scale;

                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                        eframe::egui::pos2((screen_w - 600.0) / 2.0, (screen_h - 700.0) / 2.0),
                    ));
                }
            }

            // Remove transparência temporariamente
            let visuals = eframe::egui::Visuals::dark();
            ctx.set_visuals(visuals);

            // ================================================================
            // HEADER FIXO - Título + Abas (sempre visível no topo)
            // ================================================================
            eframe::egui::TopBottomPanel::top("settings_header").show(ctx, |ui| {
                // ui.add_space(5.0);

                // // Título
                // ui.horizontal(|ui| {
                //     ui.heading("Game Translator - Configuracoes");
                // });

                ui.add_space(5.0);

                // Abas de navegação
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.settings_tab == 0, "Overlay")
                        .clicked()
                    {
                        self.settings_tab = 0;
                    }
                    if ui
                        .selectable_label(self.settings_tab == 1, "Fonte")
                        .clicked()
                    {
                        self.settings_tab = 1;
                    }
                    if ui
                        .selectable_label(self.settings_tab == 2, "Display")
                        .clicked()
                    {
                        self.settings_tab = 2;
                    }
                    if ui
                        .selectable_label(self.settings_tab == 3, "Legendas")
                        .clicked()
                    {
                        self.settings_tab = 3;
                    }
                    if ui
                        .selectable_label(self.settings_tab == 4, "Atalhos")
                        .clicked()
                    {
                        self.settings_tab = 4;
                    }
                    if ui
                        .selectable_label(self.settings_tab == 5, "Servicos")
                        .clicked()
                    {
                        self.settings_tab = 5;
                    }
                    if ui
                        .selectable_label(self.settings_tab == 6, "Historico")
                        .clicked()
                    {
                        self.settings_tab = 6;
                    }
                    if ui
                        .selectable_label(self.settings_tab == 7, "OpenAI")
                        .clicked()
                    {
                        self.settings_tab = 7;
                    }
                    if ui
                        .selectable_label(self.settings_tab == 8, "Laboratorio")
                        .clicked()
                    {
                        self.settings_tab = 8;
                    }
                });

                ui.add_space(3.0);
            });

            // ================================================================
            // FOOTER FIXO - Botões de ação (sempre visível embaixo)
            // ================================================================
            eframe::egui::TopBottomPanel::bottom("settings_footer").show(ctx, |ui| {
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    // Botão Salvar
                    if ui.button("Salvar").clicked() {
                        if let Some(ref cfg) = self.settings_config {
                            match cfg.save() {
                                Ok(_) => {
                                    let mut config = self.state.config.lock().unwrap();
                                    config.app_config = cfg.clone();
                                    self.settings_status =
                                        Some(("Salvo!".to_string(), std::time::Instant::now()));
                                    info!("Configuracoes salvas!");
                                }
                                Err(e) => {
                                    self.settings_status =
                                        Some((format!("Erro: {}", e), std::time::Instant::now()));
                                    error!("Erro ao salvar: {}", e);
                                }
                            }
                        }
                    }

                    // Botão Recarregar
                    if ui.button("Recarregar").clicked() {
                        match config::AppConfig::load() {
                            Ok(cfg) => {
                                self.settings_config = Some(cfg);
                                self.settings_status =
                                    Some(("Recarregado!".to_string(), std::time::Instant::now()));
                            }
                            Err(e) => {
                                self.settings_status =
                                    Some((format!("Erro: {}", e), std::time::Instant::now()));
                            }
                        }
                    }

                    // Mostra status temporário (3 segundos)
                    if let Some((ref msg, time)) = self.settings_status {
                        if time.elapsed() < std::time::Duration::from_secs(3) {
                            ui.label(msg);
                        }
                    }

                    // Botões do lado direito
                    ui.with_layout(
                        eframe::egui::Layout::right_to_left(eframe::egui::Align::Center),
                        |ui| {
                            if ui.button("Sair do Programa").clicked() {
                                std::process::exit(0);
                            }
                            ui.add_space(5.0);
                            if ui.button("Fechar").clicked() {
                                *self.state.settings_mode.lock().unwrap() = false;
                                self.settings_config = None;
                                self.settings_positioned = false;
                                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Decorations(
                                    false,
                                ));
                                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Resizable(
                                    false,
                                ));
                            }
                        },
                    );
                });

                ui.add_space(5.0);
            });

            // ================================================================
            // CONTEÚDO COM SCROLL (entre header e footer)
            // ================================================================
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                eframe::egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(5.0);
                    if let Some(ref mut cfg) = self.settings_config {
                        settings_ui::render_tab(
                            ui,
                            self.settings_tab,
                            cfg,
                            &self.state.subtitle_state,
                            &self.state.openai_request_count,
                            &mut self.debug_texture,
                            &mut self.debug_texture_last_update,
                            &mut self.lab_original_texture,
                            &mut self.lab_processed_texture,
                            &mut self.lab_preprocess,
                            &mut self.lab_selected_file,
                            &mut self.lab_original_image,
                            &mut self.lab_needs_reprocess,
                        );
                    }

                    // Espaço extra no final pra não colar no footer
                    ui.add_space(5.0);
                });
            });

            ctx.request_repaint();
            return; // Não renderiza o overlay enquanto estiver nas configurações
        }

        // ====================================================================
        // VERIFICA SE HÁ LEGENDAS DO MODO LEGENDA PARA EXIBIR
        // ====================================================================
        let subtitle_mode_active = *self.state.subtitle_mode_active.lock().unwrap();
        let has_subtitles = self.state.subtitle_state.has_subtitles();

        // ====================================================================
        // VERIFICA SE HÁ TRADUÇÕES NORMAIS PARA EXIBIR
        // ====================================================================
        let should_display = if let Some((_, _, _, timestamp)) = self.state.get_translations() {
            timestamp.elapsed() < self.display_duration
        } else {
            false
        };

        // ====================================================================
        // MODO LEGENDA: Exibe histórico de legendas acima da região
        // ====================================================================
        if subtitle_mode_active && has_subtitles {
            // Pega a região de legenda do config
            let (_sub_x, sub_y, _sub_w, _sub_h) = {
                let config = self.state.config.lock().unwrap();
                (
                    config.app_config.subtitle.region.x as f32,
                    config.app_config.subtitle.region.y as f32,
                    config.app_config.subtitle.region.width as f32,
                    config.app_config.subtitle.region.height as f32,
                )
            };

            // Pega configurações de fonte (específica de legendas) e fundo
            let (
                font_size,
                font_color,
                show_background,
                bg_color,
                outline_enabled,
                outline_width,
                outline_color,
            ) = {
                let config = self.state.config.lock().unwrap();
                (
                    config.app_config.subtitle.font.size,
                    config.app_config.subtitle.font.color,
                    config.app_config.overlay.show_background,
                    config.app_config.overlay.background_color,
                    config.app_config.subtitle.font.outline.enabled,
                    config.app_config.subtitle.font.outline.width,
                    config.app_config.subtitle.font.outline.color,
                )
            };

            // Pega o histórico de legendas
            let history = self.state.subtitle_state.get_subtitle_history();

            // Pega número máximo de legendas do config
            let max_lines = {
                let config = self.state.config.lock().unwrap();
                config.app_config.subtitle.max_lines
            };

            // Pega apenas as últimas N legendas
            let visible_history: Vec<_> = if history.len() > max_lines {
                history[(history.len() - max_lines)..].to_vec()
            } else {
                history.clone()
            };

            // Calcula altura dinâmica baseada no conteúdo real
            let font_id_calc = eframe::egui::FontId::proportional(font_size);
            let screen_width_calc =
                unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN) }
                    as f32;
            let max_width_calc = (screen_width_calc - 100.0) / self.state.dpi_scale - 100.0;

            let mut calculated_height = 10.0; // Margem superior
            for entry in &visible_history {
                let text = format!("- {}", entry.translated);
                let galley = ctx.fonts(|f| {
                    f.layout(
                        text,
                        font_id_calc.clone(),
                        eframe::egui::Color32::WHITE,
                        max_width_calc,
                    )
                });
                calculated_height += galley.rect.height() + 5.0;
            }
            calculated_height += 10.0; // Margem inferior

            let overlay_height = calculated_height.max(50.0); // Mínimo de 50px

            // Posiciona o overlay ACIMA da região de legenda
            // Usa largura TOTAL da tela para a caixa de tradução
            let scale = self.state.dpi_scale;
            let screen_width =
                unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN) }
                    as f32;
            // Margem lateral (pixels físicos) - ajuste se quiser mais/menos borda
            let side_margin = 50.0;
            let overlay_width = (screen_width - side_margin * 2.0) / scale;
            let overlay_x = side_margin / scale;
            // overlay_height já está em lógico (calculado pelo galley)
            let overlay_y = sub_y / scale - overlay_height - 10.0;

            // Posiciona e redimensiona a janela
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                eframe::egui::pos2(overlay_x, overlay_y),
            ));
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                eframe::egui::vec2(overlay_width, overlay_height),
            ));

            // Renderiza o histórico de legendas
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
                .show(ctx, |ui| {
                    // Se show_background = true, desenha o fundo
                    if show_background {
                        let rect = ui.max_rect();
                        ui.painter().rect_filled(
                            rect,
                            0.0,
                            eframe::egui::Color32::from_rgba_unmultiplied(
                                bg_color[0],
                                bg_color[1],
                                bg_color[2],
                                bg_color[3],
                            ),
                        );
                    }

                    // Configura renderização
                    let font_id = eframe::egui::FontId::proportional(font_size);
                    let max_width = overlay_width - 100.0; // Mais margem lateral pro texto

                    let text_color = eframe::egui::Color32::from_rgba_unmultiplied(
                        font_color[0],
                        font_color[1],
                        font_color[2],
                        font_color[3],
                    );

                    // Renderiza cada legenda do histórico
                    let mut y_offset = 5.0;

                    for entry in &visible_history {
                        let text = format!("- {}", entry.translated);

                        // Centraliza horizontalmente: calcula largura do texto
                        // e posiciona no centro do overlay
                        let galley_measure = ui.painter().layout(
                            text.clone(),
                            font_id.clone(),
                            text_color,
                            max_width,
                        );
                        let text_w = galley_measure.rect.width();
                        let center_x = (overlay_width - text_w) / 2.0;
                        let text_pos = eframe::egui::pos2(center_x.max(10.0), y_offset);

                        // Calcula o galley para obter a altura real
                        let galley = ui.painter().layout(
                            text.clone(),
                            font_id.clone(),
                            text_color,
                            max_width,
                        );
                        let text_height = galley.rect.height();

                        // Desenha contorno se habilitado OU se não tem fundo
                        if outline_enabled || !show_background {
                            let size = outline_width as f32;
                            let color = eframe::egui::Color32::from_rgba_unmultiplied(
                                outline_color[0],
                                outline_color[1],
                                outline_color[2],
                                outline_color[3],
                            );

                            // Gera pontos em círculo para contorno suave
                            // Quanto maior o size, mais pontos precisamos
                            let num_points = (size * 8.0).max(16.0) as i32;

                            for layer in 1..=(size.ceil() as i32) {
                                let radius = layer as f32;

                                for i in 0..num_points {
                                    let angle =
                                        (i as f32 / num_points as f32) * std::f32::consts::PI * 2.0;
                                    let dx = angle.cos() * radius;
                                    let dy = angle.sin() * radius;

                                    let offset_pos = text_pos + eframe::egui::vec2(dx, dy);
                                    let outline_galley = ui.painter().layout(
                                        text.clone(),
                                        font_id.clone(),
                                        color,
                                        max_width,
                                    );
                                    ui.painter().galley(offset_pos, outline_galley, color);
                                }
                            }
                        }

                        // Desenha o texto principal
                        ui.painter().galley(text_pos, galley, text_color);

                        // Avança Y pela altura real do texto + espaçamento
                        y_offset += text_height + 5.0;
                    }
                });
        } else if should_display {
            // ================================================================
            // HÁ TRADUÇÃO: Mostra overlay com os textos
            // ================================================================
            if let Some((items, region, mode, _timestamp)) = self.state.get_translations() {
                // Pega tamanho da fonte do config
                let font_size = self.state.config.lock().unwrap().app_config.font.size;

                // Pega configuração de fundo e outline
                let (show_background, bg_color, outline_enabled, outline_width, outline_color) = {
                    let config = self.state.config.lock().unwrap();
                    (
                        config.app_config.overlay.show_background,
                        config.app_config.overlay.background_color,
                        config.app_config.font.outline.enabled,
                        config.app_config.font.outline.width,
                        config.app_config.font.outline.color,
                    )
                };

                // Usa o modo de captura para decidir como renderizar
                let is_fullscreen_mode = mode == CaptureMode::FullScreen;

                if is_fullscreen_mode {
                    // ========================================================
                    // MODO TELA CHEIA: Cada tradução na posição original
                    // ========================================================

                    // Calcula bounding box de todos os textos
                    let mut min_x = f64::MAX;
                    let mut min_y = f64::MAX;
                    let mut max_x = 0.0f64;
                    let mut max_y = 0.0f64;

                    for item in &items {
                        if item.translated.is_empty() || item.original == item.translated {
                            continue;
                        }
                        min_x = min_x.min(item.screen_x);
                        min_y = min_y.min(item.screen_y);
                        max_x = max_x.max(item.screen_x + item.width);
                        max_y = max_y.max(item.screen_y + item.height);
                    }

                    // Se não há textos válidos, esconde
                    if min_x == f64::MAX {
                        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                            eframe::egui::vec2(1.0, 1.0),
                        ));
                    } else {
                        // Adiciona margem
                        let margin = 20.0;
                        let scale = self.state.dpi_scale;
                        let overlay_x = (min_x - margin).max(0.0) as f32 / scale;
                        let overlay_y = (min_y - margin).max(0.0) as f32 / scale;
                        let overlay_width = (max_x - min_x + margin * 2.0) as f32 / scale;
                        let overlay_height = (max_y - min_y + margin * 2.0 + 50.0) as f32 / scale;

                        // Posiciona e redimensiona a janela
                        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                            eframe::egui::pos2(overlay_x, overlay_y),
                        ));
                        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                            eframe::egui::vec2(overlay_width, overlay_height),
                        ));

                        // Renderiza o conteúdo
                        eframe::egui::CentralPanel::default()
                            .frame(
                                eframe::egui::Frame::none()
                                    .fill(eframe::egui::Color32::TRANSPARENT),
                            )
                            .show(ctx, |ui| {
                                let font_id = eframe::egui::FontId::proportional(font_size);

                                for item in &items {
                                    if item.translated.is_empty()
                                        || item.original == item.translated
                                    {
                                        continue;
                                    }

                                    // Posição relativa ao overlay (ajustada por DPI)
                                    // Como a janela foi posicionada em coordenadas lógicas
                                    // (divididas por scale), o conteúdo interno também precisa
                                    let text_x = (item.screen_x - min_x + margin) as f32 / scale;
                                    let text_y = (item.screen_y - min_y + margin) as f32 / scale;
                                    let text_pos = eframe::egui::pos2(text_x, text_y);

                                    // Largura máxima baseada na largura original do texto
                                    let max_width = (item.width as f32 / scale * 1.5).max(200.0);

                                    // Se show_background, desenha fundo atrás do texto
                                    if show_background {
                                        let galley = ui.painter().layout(
                                            item.translated.clone(),
                                            font_id.clone(),
                                            eframe::egui::Color32::WHITE,
                                            max_width,
                                        );
                                        let text_rect = eframe::egui::Rect::from_min_size(
                                            text_pos,
                                            galley.rect.size() + eframe::egui::vec2(10.0, 6.0),
                                        );
                                        ui.painter().rect_filled(
                                            text_rect,
                                            4.0,
                                            eframe::egui::Color32::from_rgba_unmultiplied(
                                                bg_color[0],
                                                bg_color[1],
                                                bg_color[2],
                                                bg_color[3],
                                            ),
                                        );
                                    }

                                    // Desenha contorno se habilitado
                                    if outline_enabled || !show_background {
                                        let size = outline_width as f32;
                                        let color = eframe::egui::Color32::from_rgba_unmultiplied(
                                            outline_color[0],
                                            outline_color[1],
                                            outline_color[2],
                                            outline_color[3],
                                        );

                                        let num_points = (size * 8.0).max(16.0) as i32;

                                        for layer in 1..=(size.ceil() as i32) {
                                            let radius = layer as f32;

                                            for i in 0..num_points {
                                                let angle = (i as f32 / num_points as f32)
                                                    * std::f32::consts::PI
                                                    * 2.0;
                                                let dx = angle.cos() * radius;
                                                let dy = angle.sin() * radius;

                                                let offset_pos =
                                                    text_pos + eframe::egui::vec2(dx, dy);
                                                let outline_galley = ui.painter().layout(
                                                    item.translated.clone(),
                                                    font_id.clone(),
                                                    color,
                                                    max_width,
                                                );
                                                ui.painter().galley(
                                                    offset_pos,
                                                    outline_galley,
                                                    color,
                                                );
                                            }
                                        }
                                    }

                                    // Desenha o texto principal
                                    let galley = ui.painter().layout(
                                        item.translated.clone(),
                                        font_id.clone(),
                                        eframe::egui::Color32::WHITE,
                                        max_width,
                                    );
                                    ui.painter().galley(
                                        text_pos,
                                        galley,
                                        eframe::egui::Color32::WHITE,
                                    );
                                }
                            });
                    }
                } else {
                    // ========================================================
                    // MODO REGIÃO: Texto combinado em bloco único
                    // ========================================================
                    let scale = self.state.dpi_scale;
                    let overlay_x = region.x as f32 / scale;
                    let overlay_y = region.y as f32 / scale;
                    let overlay_width = region.width as f32 / scale;
                    let overlay_height = region.height as f32 / scale;

                    // Posiciona e redimensiona a janela
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                        eframe::egui::pos2(overlay_x, overlay_y),
                    ));
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                        eframe::egui::vec2(overlay_width, overlay_height),
                    ));

                    // Renderiza o conteúdo
                    eframe::egui::CentralPanel::default()
                        .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
                        .show(ctx, |ui| {
                            // Junta todas as traduções em um texto só
                            let combined_text: String = items
                                .iter()
                                .filter(|item| item.original != item.translated)
                                .map(|item| item.translated.as_str())
                                .collect::<Vec<&str>>()
                                .join(" ");

                            if !combined_text.is_empty() {
                                // Se show_background = true, desenha o fundo preto
                                if show_background {
                                    let rect = ui.max_rect();
                                    ui.painter().rect_filled(
                                        rect,
                                        0.0,
                                        eframe::egui::Color32::from_rgba_unmultiplied(
                                            bg_color[0],
                                            bg_color[1],
                                            bg_color[2],
                                            bg_color[3],
                                        ),
                                    );
                                }

                                // Posição inicial do texto (com margem)
                                let text_pos = eframe::egui::pos2(20.0, 15.0);

                                // Configura a fonte
                                let font_id = eframe::egui::FontId::proportional(font_size);

                                // Largura máxima para wrap
                                let max_width = overlay_width - 40.0;

                                // Desenha contorno se habilitado OU se não tem fundo
                                if outline_enabled || !show_background {
                                    let size = outline_width as f32;
                                    let color = eframe::egui::Color32::from_rgba_unmultiplied(
                                        outline_color[0],
                                        outline_color[1],
                                        outline_color[2],
                                        outline_color[3],
                                    );

                                    let num_points = (size * 8.0).max(16.0) as i32;

                                    for layer in 1..=(size.ceil() as i32) {
                                        let radius = layer as f32;

                                        for i in 0..num_points {
                                            let angle = (i as f32 / num_points as f32)
                                                * std::f32::consts::PI
                                                * 2.0;
                                            let dx = angle.cos() * radius;
                                            let dy = angle.sin() * radius;

                                            let offset_pos = text_pos + eframe::egui::vec2(dx, dy);
                                            let outline_galley = ui.painter().layout(
                                                combined_text.clone(),
                                                font_id.clone(),
                                                color,
                                                max_width,
                                            );
                                            ui.painter().galley(offset_pos, outline_galley, color);
                                        }
                                    }
                                }

                                // Desenha o texto principal
                                let galley = ui.painter().layout(
                                    combined_text.clone(),
                                    font_id.clone(),
                                    eframe::egui::Color32::WHITE,
                                    max_width,
                                );
                                ui.painter()
                                    .galley(text_pos, galley, eframe::egui::Color32::WHITE);
                            }
                        });
                }
            }
        } else {
            // ================================================================
            // SEM TRADUÇÃO: Janela mínima e invisível
            // ================================================================
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
                eframe::egui::vec2(1.0, 1.0),
            ));
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                eframe::egui::pos2(0.0, 0.0),
            ));

            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::none().fill(eframe::egui::Color32::TRANSPARENT))
                .show(ctx, |_ui| {});
        }

        // Repaint contínuo
        ctx.request_repaint();
    }
}

// ============================================================================
// THREAD DE HOTKEYS (roda em background)
// ============================================================================

fn start_hotkey_thread(state: AppState) {
    thread::spawn(move || {
        info!("⌨️  Thread de hotkeys iniciada");

        // Pega as configurações de hotkeys
        let hotkeys = state.config.lock().unwrap().app_config.hotkeys.clone();
        let mut hotkey_manager = hotkey::HotkeyManager::new(&hotkeys);

        loop {
            if let Some(action) = hotkey_manager.check_hotkey() {
                // Se está no modo configurações, ignora TODAS as hotkeys
                // exceto OpenSettings (pra poder fechar a janela)
                let is_settings = *state.settings_mode.lock().unwrap();
                if is_settings && action != hotkey::HotkeyAction::OpenSettings {
                    continue;
                }

                match action {
                    hotkey::HotkeyAction::SelectRegion => {
                        info!("");
                        info!("🎯 ============================================");
                        info!("🎯 SOLICITANDO ABERTURA DO SELETOR DE REGIÃO");
                        info!("🎯 ============================================");

                        if let Err(e) = state.command_sender.send(AppCommand::OpenRegionSelector) {
                            error!("❌ Erro ao enviar comando: {}", e);
                        }
                    }

                    hotkey::HotkeyAction::SelectSubtitleRegion => {
                        info!("");
                        info!("📺 ============================================");
                        info!("📺 SOLICITANDO ABERTURA DO SELETOR DE LEGENDA");
                        info!("📺 ============================================");

                        if let Err(e) = state
                            .command_sender
                            .send(AppCommand::OpenSubtitleRegionSelector)
                        {
                            error!("❌ Erro ao enviar comando: {}", e);
                        }
                    }

                    hotkey::HotkeyAction::HideTranslation => {
                        info!("");
                        info!("🙈 ============================================");
                        info!("🙈 ESCONDENDO TRADUÇÃO");
                        info!("🙈 ============================================");

                        state.clear_translations();
                    }

                    hotkey::HotkeyAction::ToggleSubtitleMode => {
                        let mut active = state.subtitle_mode_active.lock().unwrap();
                        *active = !*active;

                        info!("");
                        if *active {
                            info!("📺 ============================================");
                            info!("📺 MODO LEGENDA: ✅ ATIVADO");
                            info!("📺 ============================================");
                        } else {
                            info!("📺 ============================================");
                            info!("📺 MODO LEGENDA: ❌ DESATIVADO");
                            info!("📺 ============================================");
                        }
                    }

                    hotkey::HotkeyAction::TranslateFullScreen => {
                        info!("");
                        info!("▶️  ============================================");
                        info!("▶️  MODO: 🖥️  TELA INTEIRA");
                        info!("▶️  ============================================");

                        let state_clone = state.clone();
                        thread::spawn(move || {
                            if let Err(e) = processing::process_translation_blocking(
                                &state_clone,
                                hotkey::HotkeyAction::TranslateFullScreen,
                            ) {
                                error!("❌ Erro: {}", e);
                            }
                        });
                    }

                    hotkey::HotkeyAction::TranslateRegion => {
                        info!("");
                        info!("▶️  ============================================");
                        info!("▶️  MODO: 🎯 REGIÃO CUSTOMIZADA");
                        info!("▶️  ============================================");

                        let state_clone = state.clone();
                        thread::spawn(move || {
                            if let Err(e) = processing::process_translation_blocking(
                                &state_clone,
                                hotkey::HotkeyAction::TranslateRegion,
                            ) {
                                error!("❌ Erro: {}", e);
                            }
                        });
                    }

                    hotkey::HotkeyAction::OpenSettings => {
                        // Verifica se já está no modo configurações
                        let is_settings = *state.settings_mode.lock().unwrap();

                        if is_settings {
                            info!("");
                            info!("⚙️  ============================================");
                            info!("⚙️  FECHANDO JANELA DE CONFIGURAÇÕES");
                            info!("⚙️  ============================================");

                            if let Err(e) = state.command_sender.send(AppCommand::CloseSettings) {
                                error!("❌ Erro ao enviar comando: {}", e);
                            }
                        } else {
                            info!("");
                            info!("⚙️  ============================================");
                            info!("⚙️  ABRINDO JANELA DE CONFIGURAÇÕES");
                            info!("⚙️  ============================================");

                            if let Err(e) = state.command_sender.send(AppCommand::OpenSettings) {
                                error!("❌ Erro ao enviar comando: {}", e);
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(50));
        }
    });
}

// ============================================================================
// THREAD DE CONFIG WATCHER (monitora mudanças no config.json)
// ============================================================================

fn start_config_watcher(state: AppState) {
    thread::spawn(move || {
        info!("👁️  Thread de monitoramento do config.json iniciada");

        let (tx, rx) = channel();

        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                error!("❌ Erro ao criar watcher: {}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(Path::new("config.json"), RecursiveMode::NonRecursive) {
            error!("❌ Erro ao monitorar config.json: {}", e);
            return;
        }

        info!("✅ Monitorando config.json para mudanças...");

        let mut last_reload = std::time::Instant::now();
        let debounce_duration = Duration::from_millis(500);

        loop {
            match rx.recv() {
                Ok(event_result) => {
                    if let Ok(event) = event_result {
                        if matches!(event.kind, notify::EventKind::Modify(_)) {
                            if last_reload.elapsed() < debounce_duration {
                                continue;
                            }

                            last_reload = std::time::Instant::now();

                            info!("");
                            info!("🔄 CONFIG.JSON MODIFICADO - RECARREGANDO");

                            thread::sleep(Duration::from_millis(100));

                            match Config::load() {
                                Ok(new_config) => {
                                    let mut config = state.config.lock().unwrap();
                                    *config = new_config;
                                    info!("✅ Configurações recarregadas!");
                                }
                                Err(e) => {
                                    error!("❌ Erro ao recarregar config: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Erro ao receber evento: {}", e);
                    break;
                }
            }
        }
    });
}

// ============================================================================
// FUNÇÃO PRINCIPAL
// ============================================================================

fn main() -> Result<()> {
    // Declara que o programa é DPI-aware (Per-Monitor V2)
    // Sem isso, o Windows "mente" e diz que o DPI é 96 (100%)
    // mesmo quando o usuário tem 125%, 150%, etc.
    unsafe {
        winapi::um::shellscalingapi::SetProcessDpiAwareness(2); // 2 = Per-Monitor DPI Aware
    }

    env_logger::init();

    info!("🎮 ============================================");
    info!("🎮 GAME TRANSLATOR - Tradutor para Jogos");
    info!("🎮 ============================================");
    info!("");

    // Carrega configurações
    let config = Config::load()?;

    // Cria canal de comunicação
    let (command_sender, command_receiver) = unbounded::<AppCommand>();

    // Cria estado compartilhado
    let dpi = unsafe { winapi::um::winuser::GetDpiForSystem() };
    let dpi_scale = dpi as f32 / 96.0;
    info!(
        "📐 DPI do sistema: {} (escala: {}%)",
        dpi,
        (dpi_scale * 100.0) as u32
    );

    let state = AppState::new(config, command_sender, dpi_scale);

    // Inicia threads
    start_hotkey_thread(state.clone());
    start_config_watcher(state.clone());
    processing::start_subtitle_thread(state.clone());

    info!("✅ Sistema pronto!");
    info!("   Numpad - = Tela inteira");
    info!("   Numpad + = Região customizada");
    info!("   Numpad * = Selecionar região");
    info!("");

    // Configurações do overlay
    let config = state.config.lock().unwrap();
    let overlay_width = config.app_config.overlay.width as f32;
    let overlay_height = config.app_config.overlay.height as f32;
    let display_duration = config.app_config.display.overlay_duration_secs;
    drop(config);

    // Opções da janela
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([overlay_width, overlay_height])
            .with_position([0.0, 0.0])
            .with_always_on_top()
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true),
        ..Default::default()
    };

    // Inicia o overlay
    let _ = eframe::run_native(
        "Game Translator",
        options,
        Box::new(move |cc| {
            // Configura visual transparente
            let mut visuals = eframe::egui::Visuals::dark();
            visuals.panel_fill = eframe::egui::Color32::TRANSPARENT;
            visuals.window_fill = eframe::egui::Color32::TRANSPARENT;
            cc.egui_ctx.set_visuals(visuals);

            // Carrega fonte custom se configurada como "file"
            {
                let config = state.config.lock().unwrap();
                let font_path = &config.app_config.font.file_path;
                let font_type = &config.app_config.font.font_type;

                if font_type == "file" && !font_path.is_empty() {
                    match std::fs::read(font_path) {
                        Ok(font_data) => {
                            let mut fonts = eframe::egui::FontDefinitions::default();

                            // Registra a fonte com nome "custom"
                            fonts.font_data.insert(
                                "custom".to_owned(),
                                eframe::egui::FontData::from_owned(font_data),
                            );

                            // Coloca como primeira opção para Proportional e Monospace
                            fonts
                                .families
                                .get_mut(&eframe::egui::FontFamily::Proportional)
                                .unwrap()
                                .insert(0, "custom".to_owned());

                            fonts
                                .families
                                .get_mut(&eframe::egui::FontFamily::Monospace)
                                .unwrap()
                                .insert(0, "custom".to_owned());

                            cc.egui_ctx.set_fonts(fonts);
                            info!("✅ Fonte custom carregada: {}", font_path);
                        }
                        Err(e) => {
                            error!("❌ Erro ao carregar fonte '{}': {}", font_path, e);
                            info!("   Usando fonte padrão do sistema");
                        }
                    }
                } else {
                    info!(
                        "🔤 Usando fonte do sistema: {}",
                        config.app_config.font.system_font_name
                    );
                }
            }

            Ok(Box::new(OverlayApp {
                state: state.clone(),
                display_duration: Duration::from_secs(display_duration),
                command_receiver,
                settings_config: None,
                settings_tab: 0,
                settings_status: None,
                settings_positioned: false,
                debug_texture: None,
                debug_texture_last_update: std::time::Instant::now(),
                lab_original_texture: None,
                lab_processed_texture: None,
                lab_preprocess: None,
                lab_selected_file: None,
                lab_original_image: None,
                lab_needs_reprocess: false,
                // last_window_size: (0.0, 0.0),
            }) as Box<dyn eframe::App>)
        }),
    );

    Ok(())
}

// ============================================================================
// FUNÇÃO PARA TORNAR JANELA CLICK-THROUGH (WINDOWS)
// ============================================================================

#[cfg(windows)]
fn make_window_click_through() {
    use winapi::um::winuser::{
        FindWindowW, GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };

    unsafe {
        // Encontra a janela pelo título
        let title: Vec<u16> = "Game Translator\0".encode_utf16().collect();
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());

        if !hwnd.is_null() {
            // Pega o estilo atual
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);

            // Adiciona WS_EX_LAYERED e WS_EX_TRANSPARENT para click-through
            let new_style = ex_style | WS_EX_LAYERED as i32 | WS_EX_TRANSPARENT as i32;
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);

            trace!("✅ Janela configurada como click-through!");
        } else {
            warn!("⚠️  Não foi possível encontrar a janela para click-through");
        }
    }
}

fn remove_window_click_through() {
    use winapi::um::winuser::{
        FindWindowW, GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };

    unsafe {
        let title: Vec<u16> = "Game Translator\0".encode_utf16().collect();
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());

        if !hwnd.is_null() {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            // Remove WS_EX_TRANSPARENT mas mantém WS_EX_LAYERED (pra transparência visual)
            let new_style = (ex_style | WS_EX_LAYERED as i32) & !(WS_EX_TRANSPARENT as i32);
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
        }
    }
}
