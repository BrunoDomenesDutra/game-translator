// // game-translator/src/overlay.rs

// // ============================================================================
// // MÓDULO OVERLAY - Overlay permanente e transparente
// // ============================================================================

// use anyhow::Result;
// use crossbeam_channel::{Receiver, Sender};
// use eframe::egui;
// use std::sync::{Arc, Mutex};
// use std::thread;
// use std::time::{Duration, Instant};

// /// Mensagem para o overlay
// #[derive(Clone, Debug)]
// pub struct OverlayMessage {
//     /// Texto a ser exibido
//     pub text: String,
// }

// /// Canal de comunicação com o overlay
// pub struct OverlayChannel {
//     sender: Sender<OverlayMessage>,
// }

// impl OverlayChannel {
//     /// Envia texto para ser exibido no overlay
//     pub fn show_text(&self, text: String) -> Result<()> {
//         self.sender.send(OverlayMessage { text })?;
//         Ok(())
//     }
// }

// /// Estado compartilhado do overlay
// #[derive(Clone)]
// struct OverlayState {
//     /// Texto atual sendo exibido
//     current_text: Arc<Mutex<Option<String>>>,
//     /// Quando o texto começou a ser exibido
//     display_start: Arc<Mutex<Option<Instant>>>,
//     /// Quanto tempo exibir (em segundos)
//     display_duration: Duration,
// }

// impl OverlayState {
//     fn new() -> Self {
//         OverlayState {
//             current_text: Arc::new(Mutex::new(None)),
//             display_start: Arc::new(Mutex::new(None)),
//             display_duration: Duration::from_secs(5),
//         }
//     }

//     /// Define novo texto para exibir
//     fn set_text(&self, text: String) {
//         *self.current_text.lock().unwrap() = Some(text);
//         *self.display_start.lock().unwrap() = Some(Instant::now());
//     }

//     /// Obtém o texto atual (se ainda dentro do tempo de exibição)
//     fn get_current_text(&self) -> Option<String> {
//         let start = self.display_start.lock().unwrap();
//         if let Some(start_time) = *start {
//             if start_time.elapsed() < self.display_duration {
//                 return self.current_text.lock().unwrap().clone();
//             } else {
//                 // Tempo esgotado, limpa o texto
//                 drop(start);
//                 *self.current_text.lock().unwrap() = None;
//                 *self.display_start.lock().unwrap() = None;
//             }
//         }
//         None
//     }

//     /// Tempo restante de exibição
//     fn time_remaining(&self) -> Option<u64> {
//         let start = self.display_start.lock().unwrap();
//         if let Some(start_time) = *start {
//             let elapsed = start_time.elapsed();
//             if elapsed < self.display_duration {
//                 return Some((self.display_duration - elapsed).as_secs());
//             }
//         }
//         None
//     }
// }

// /// Aplicação do overlay
// struct OverlayApp {
//     state: OverlayState,
//     receiver: Receiver<OverlayMessage>,
// }

// impl eframe::App for OverlayApp {
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         // ====================================================================
//         // Recebe novas mensagens do canal
//         // ====================================================================
//         while let Ok(msg) = self.receiver.try_recv() {
//             info!("📨 Overlay recebeu texto: {} caracteres", msg.text.len());
//             self.state.set_text(msg.text);
//         }

//         // ====================================================================
//         // Verifica se há texto para exibir
//         // ====================================================================
//         let current_text = self.state.get_current_text();
//         let time_remaining = self.state.time_remaining();

//         // ====================================================================
//         // PAINEL CENTRAL
//         // ====================================================================
//         egui::CentralPanel::default()
//             .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
//             .show(ctx, |ui| {
//                 if let Some(text) = current_text {
//                     // ========================================================
//                     // EXIBIR TRADUÇÃO
//                     // ========================================================

//                     // Fundo semi-transparente
//                     let rect = ui.max_rect();
//                     ui.painter().rect_filled(
//                         rect,
//                         0.0,
//                         egui::Color32::from_rgba_unmultiplied(0, 0, 0, 235),
//                     );

//                     ui.vertical_centered(|ui| {
//                         ui.add_space(25.0);

//                         // Texto da tradução
//                         ui.label(
//                             egui::RichText::new(&text)
//                                 .color(egui::Color32::WHITE)
//                                 .size(36.0),
//                         );

//                         ui.add_space(15.0);

//                         // Contador regressivo
//                         if let Some(remaining) = time_remaining {
//                             ui.label(
//                                 egui::RichText::new(format!("⏱ {} segundos", remaining + 1))
//                                     .color(egui::Color32::from_rgb(150, 150, 150))
//                                     .size(14.0),
//                             );
//                         }
//                     });
//                 }
//                 // Se não há texto, a janela fica completamente transparente
//             });

//         // Solicita repaint para atualizar o timer
//         ctx.request_repaint_after(Duration::from_millis(100));
//     }
// }

// /// Inicia o overlay permanente em uma thread separada
// ///
// /// # Argumentos
// /// * `x` - Posição X da janela
// /// * `y` - Posição Y da janela
// /// * `width` - Largura da janela
// /// * `height` - Altura da janela
// ///
// /// # Retorna
// /// * Canal para enviar textos ao overlay
// pub fn start_overlay(x: f32, y: f32, width: f32, height: f32) -> Result<OverlayChannel> {
//     info!("🖼️  Iniciando overlay permanente...");
//     info!("   Posição: ({}, {})", x, y);
//     info!("   Tamanho: {}x{}", width, height);

//     // Cria canal de comunicação
//     let (sender, receiver) = crossbeam_channel::unbounded();

//     // Estado compartilhado
//     let state = OverlayState::new();

//     // Inicia overlay na thread principal (obrigatório no Windows)
//     thread::spawn(move || {
//         let options = eframe::NativeOptions {
//             viewport: egui::ViewportBuilder::default()
//                 .with_inner_size([width, height])
//                 .with_position([x, y])
//                 .with_always_on_top()
//                 .with_decorations(false)
//                 .with_resizable(false)
//                 .with_transparent(true),

//             ..Default::default()
//         };

//         let app = OverlayApp { state, receiver };

//         let _ = eframe::run_native(
//             "Game Translator Overlay",
//             options,
//             Box::new(move |_cc| Ok(Box::new(app))),
//         );
//     });

//     info!("✅ Overlay iniciado!");

//     Ok(OverlayChannel { sender })
// }

// /// Função auxiliar para manter compatibilidade (deprecada)
// pub fn show_overlay(_text: &str) -> Result<()> {
//     warn!("⚠️  show_overlay() está deprecada. Use o canal do overlay.");
//     Ok(())
// }
