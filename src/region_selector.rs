// game-translator/src/region_selector.rs

// ============================================================================
// MÓDULO REGION SELECTOR - Seleção visual usando overlay transparente
// ============================================================================
//
// Este módulo cria uma janela transparente por cima de TUDO na tela,
// permitindo ao usuário clicar e arrastar para selecionar uma região.
//
// Diferente da versão anterior (que tirava screenshot e mostrava uma imagem
// estática), esta versão usa Windows API pura para criar um overlay
// transparente. A tela real continua visível e rodando por baixo.
//
// Tecnologias usadas:
// - winapi: Criação de janela Win32, mensagens, GDI para desenho
// - Nenhuma dependência externa além do winapi (já no Cargo.toml)
//
// ============================================================================

use anyhow::Result;

// Imports do Windows API
// Cada um desses é uma função ou constante da API do Windows
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HWND, POINT, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::{
    // Funções de desenho GDI (Graphics Device Interface)
    CreatePen,        // Cria uma "caneta" para desenhar linhas/bordas
    CreateSolidBrush, // Cria um "pincel" para preencher áreas
    DeleteObject,     // Libera objetos GDI da memória
    // Funções de preenchimento
    Rectangle,    // Desenha retângulo com borda + preenchimento
    SelectObject, // Seleciona caneta/pincel para uso
    SetBkMode,    // Define modo de fundo (transparente/opaco)
    SetTextColor, // Define cor do texto
    TextOutW,     // Desenha texto na tela
    // Estilos de caneta
    PS_SOLID, // Linha sólida (não tracejada)
    // Modos de fundo
    TRANSPARENT, // Fundo transparente para texto
};
use winapi::um::winuser::{
    BeginPaint, // Início de pintura (WM_PAINT)
    // Funções de janela
    CreateWindowExW,   // Cria janela com estilos estendidos
    DefWindowProcW,    // Processamento padrão de mensagens
    DestroyWindow,     // Fecha/destrói uma janela
    DispatchMessageW,  // Despacha mensagem para WndProc
    EndPaint,          // Fim de pintura (WM_PAINT)
    FillRect,          // Preenche um retângulo com um pincel
    GetClientRect,     // Pega dimensões internas da janela
    GetMessageW,       // Pega próxima mensagem da fila
    GetSystemMetrics,  // Pega info do sistema (tamanho da tela)
    GetWindowLongPtrW, // Recupera dados da janela
    InvalidateRect,    // Marca área para redesenho
    LoadCursorW,       // Carrega cursor do sistema
    PostQuitMessage,   // Envia mensagem de encerramento
    RegisterClassExW,  // Registra classe de janela
    SetCursor,         // Define cursor do mouse
    // Layered window
    SetLayeredWindowAttributes,
    SetWindowLongPtrW, // Armazena dados na janela
    ShowWindow,        // Mostra/esconde janela
    TranslateMessage,  // Traduz mensagens de teclado
    UpdateWindow,      // Força atualização da janela
    // Armazenamento na janela
    GWLP_USERDATA, // Slot para dados do usuário na janela
    // Cursor padrão
    IDC_CROSS, // Cursor em formato de cruz (+)
    LWA_ALPHA, // Transparência por opacidade (0-255)
    // Constantes de mensagens do Windows
    MSG,         // Estrutura de mensagem
    PAINTSTRUCT, // Estrutura de pintura
    // Métricas do sistema
    SM_CXSCREEN, // Largura da tela
    SM_CYSCREEN, // Altura da tela
    // Constantes de exibição
    SW_SHOW, // Código para mostrar janela
    // Tecla virtual
    VK_ESCAPE, // Código da tecla ESC
    // Mensagens que o Windows envia para nossa janela
    WM_CREATE,        // Janela foi criada
    WM_DESTROY,       // Janela está sendo destruída
    WM_ERASEBKGND,    // Apagar fundo (interceptamos para transparência)
    WM_KEYDOWN,       // Tecla pressionada
    WM_LBUTTONDOWN,   // Botão esquerdo do mouse pressionado
    WM_LBUTTONUP,     // Botão esquerdo do mouse solto
    WM_MOUSEMOVE,     // Mouse se moveu
    WM_PAINT,         // Janela precisa ser redesenhada
    WM_SETCURSOR,     // Definir cursor
    WNDCLASSEXW,      // Estrutura de classe de janela
    WS_EX_LAYERED,    // Suporta transparência
    WS_EX_TOOLWINDOW, // Não aparece na barra de tarefas
    // Estilos estendidos de janela
    WS_EX_TOPMOST, // Sempre por cima de tudo
    // Estilos de janela
    WS_POPUP,   // Janela sem borda/título
    WS_VISIBLE, // Janela visível
};

// Imports da biblioteca padrão do Rust
use std::mem; // Para inicialização de structs com zeroed()
use std::ptr; // Para ponteiros nulos (null_mut)
use std::sync::Mutex; // Para compartilhar resultado entre threads

// ============================================================================
// ESTRUTURAS DE DADOS
// ============================================================================

/// Coordenadas da região selecionada (mesma interface da versão anterior)
/// O main.rs usa essa struct, então mantemos ela igual
#[derive(Debug, Clone)]
pub struct SelectedRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Estado interno do seletor de região
/// Essa struct é armazenada dentro da janela Win32 via GWLP_USERDATA
struct SelectorState {
    /// Ponto onde o usuário começou a arrastar (None = não começou ainda)
    start_point: Option<POINT>,
    /// Posição atual do mouse
    current_point: POINT,
    /// Se o usuário está arrastando (botão pressionado)
    is_dragging: bool,
    /// Resultado da seleção (preenchido quando o usuário solta o botão)
    result: Option<SelectedRegion>,
    /// Se o usuário cancelou (ESC)
    cancelled: bool,
}

// ============================================================================
// VARIÁVEL GLOBAL PARA RESULTADO
// ============================================================================
//
// A Windows API usa callbacks (WndProc) que não recebem dados diretamente.
// Usamos GWLP_USERDATA para associar nosso SelectorState à janela,
// mas o resultado final precisa sobreviver após a janela ser destruída.
// Por isso usamos uma variável global protegida por Mutex.
//
// Isso é seguro porque:
// 1. Só uma instância do seletor roda por vez
// 2. O Mutex garante acesso exclusivo
//
static SELECTOR_RESULT: Mutex<Option<Option<SelectedRegion>>> = Mutex::new(None);

// ============================================================================
// FUNÇÃO PÚBLICA - PONTO DE ENTRADA
// ============================================================================

/// Abre a interface de seleção de região e retorna a região selecionada
///
/// Cria um overlay transparente sobre toda a tela. O usuário clica e
/// arrasta para selecionar uma região. ESC cancela.
///
/// # Retorna
/// * `Ok(Some(SelectedRegion))` - Região selecionada com sucesso
/// * `Ok(None)` - Usuário cancelou (ESC)
/// * `Err(...)` - Erro ao criar janela
pub fn select_region() -> Result<Option<SelectedRegion>> {
    info!("🎯 Iniciando seletor de região (overlay transparente)...");

    // Limpa resultado anterior
    *SELECTOR_RESULT.lock().unwrap() = None;

    // Cria e executa a janela do seletor
    // Essa função bloqueia até o usuário selecionar ou cancelar
    unsafe {
        create_selector_window()?;
    }

    // Pega o resultado
    let result = SELECTOR_RESULT.lock().unwrap().take().unwrap_or(None);

    match &result {
        Some(region) => {
            info!(
                "✅ Região selecionada: {}x{} na posição ({}, {})",
                region.width, region.height, region.x, region.y
            );
        }
        None => {
            info!("❌ Seleção cancelada");
        }
    }

    Ok(result)
}

// ============================================================================
// CRIAÇÃO DA JANELA WIN32
// ============================================================================

/// Cria a janela overlay transparente e inicia o loop de mensagens
///
/// # Segurança
/// Usa Windows API (unsafe). Todas as chamadas são padrão Win32.
unsafe fn create_selector_window() -> Result<()> {
    // ========================================================================
    // PASSO 1: Registrar a classe da janela
    // ========================================================================
    //
    // No Windows, antes de criar uma janela, você precisa registrar uma
    // "classe" que define o comportamento dela (ícone, cursor, callback, etc).
    //
    let class_name = wide_string("GameTranslatorSelector");
    let hinstance = GetModuleHandleW(ptr::null());

    let wc = WNDCLASSEXW {
        cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(wnd_proc), // Callback que processa mensagens
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: ptr::null_mut(),
        hCursor: LoadCursorW(ptr::null_mut(), IDC_CROSS), // Cursor de cruz
        hbrBackground: ptr::null_mut(),                   // Sem fundo (nós controlamos)
        lpszMenuName: ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: ptr::null_mut(),
    };

    RegisterClassExW(&wc);

    // ========================================================================
    // PASSO 2: Pegar tamanho da tela
    // ========================================================================
    let screen_width = GetSystemMetrics(SM_CXSCREEN);
    let screen_height = GetSystemMetrics(SM_CYSCREEN);

    info!("   📐 Tela: {}x{}", screen_width, screen_height);

    // ========================================================================
    // PASSO 3: Criar a janela
    // ========================================================================
    //
    // WS_EX_TOPMOST  = Sempre por cima de todas as janelas
    // WS_EX_LAYERED  = Permite transparência por cor (color key)
    // WS_EX_TOOLWINDOW = Não mostra na barra de tarefas
    // WS_POPUP       = Sem borda, sem título, sem botões
    // WS_VISIBLE     = Já começa visível
    //
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
        class_name.as_ptr(),
        wide_string("Seletor de Região").as_ptr(),
        WS_POPUP | WS_VISIBLE,
        0,               // Posição X = 0 (canto esquerdo)
        0,               // Posição Y = 0 (topo)
        screen_width,    // Largura = tela inteira
        screen_height,   // Altura = tela inteira
        ptr::null_mut(), // Sem janela pai
        ptr::null_mut(), // Sem menu
        hinstance,
        ptr::null_mut(), // Sem dados extras na criação
    );

    if hwnd.is_null() {
        anyhow::bail!("Falha ao criar janela do seletor de região");
    }

    // ========================================================================
    // PASSO 4: Configurar transparência por Color Key
    // ========================================================================
    //
    // LWA_COLORKEY diz ao Windows: "qualquer pixel com esta cor exata
    // deve ser tratado como transparente". Assim, pintamos o fundo com
    // TRANSPARENCY_COLOR e ele fica invisível. Só o retângulo de seleção
    // (que usa outras cores) fica visível.
    //
    SetLayeredWindowAttributes(hwnd, 0, 1, LWA_ALPHA);

    // ========================================================================
    // PASSO 5: Criar estado e associar à janela
    // ========================================================================
    //
    // Alocamos o SelectorState no heap (Box) e armazenamos o ponteiro
    // na janela via GWLP_USERDATA. Assim, o WndProc pode acessar o estado.
    //
    let state = Box::new(SelectorState {
        start_point: None,
        current_point: POINT { x: 0, y: 0 },
        is_dragging: false,
        result: None,
        cancelled: false,
    });

    // Box::into_raw converte o Box em ponteiro bruto (não será liberado automaticamente)
    // Nós liberamos manualmente no WM_DESTROY
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

    // ========================================================================
    // PASSO 6: Mostrar janela e iniciar loop de mensagens
    // ========================================================================
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);

    info!("✅ Janela do seletor aberta. Clique e arraste para selecionar. ESC para cancelar.");

    // Loop de mensagens do Windows
    // Roda até receber WM_QUIT (quando a janela é fechada)
    let mut msg: MSG = mem::zeroed();
    while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    Ok(())
}

// ============================================================================
// CALLBACK DE MENSAGENS (WndProc)
// ============================================================================
//
// Esta função é chamada pelo Windows toda vez que algo acontece na janela:
// mouse moveu, tecla pressionada, janela precisa ser redesenhada, etc.
//
// É o "coração" da janela Win32.
//
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // ====================================================================
        // WM_CREATE - Janela acabou de ser criada
        // ====================================================================
        WM_CREATE => {
            0 // Retorna 0 = sucesso
        }

        // ====================================================================
        // WM_ERASEBKGND - Windows quer apagar o fundo
        // ====================================================================
        // Interceptamos para pintar com nossa cor de transparência
        WM_ERASEBKGND => {
            1 // Retorna 1 = "já apaguei o fundo, não precisa fazer nada"
        }

        // ====================================================================
        // WM_SETCURSOR - Windows pergunta qual cursor usar
        // ====================================================================
        WM_SETCURSOR => {
            // Sempre usa cursor de cruz durante a seleção
            SetCursor(LoadCursorW(ptr::null_mut(), IDC_CROSS));
            1 // Retorna 1 = "já defini o cursor"
        }

        // ====================================================================
        // WM_LBUTTONDOWN - Botão esquerdo do mouse pressionado
        // ====================================================================
        WM_LBUTTONDOWN => {
            // Aumenta opacidade para o retângulo de seleção ficar visível
            SetLayeredWindowAttributes(hwnd, 0, 180, LWA_ALPHA);

            let state = get_state(hwnd);
            if let Some(state) = state {
                // Extrai coordenadas do mouse do LPARAM
                // Os 16 bits inferiores = X, os 16 bits superiores = Y
                let x = (lparam & 0xFFFF) as i16 as i32;
                let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                state.start_point = Some(POINT { x, y });
                state.current_point = POINT { x, y };
                state.is_dragging = true;
            }
            0
        }

        // ====================================================================
        // WM_MOUSEMOVE - Mouse se moveu
        // ====================================================================
        WM_MOUSEMOVE => {
            let state = get_state(hwnd);
            if let Some(state) = state {
                if state.is_dragging {
                    let x = (lparam & 0xFFFF) as i16 as i32;
                    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                    state.current_point = POINT { x, y };

                    // Pede para o Windows redesenhar a janela
                    // NULL = redesenha tudo, TRUE = apaga fundo primeiro
                    InvalidateRect(hwnd, ptr::null(), 1);
                }
            }
            0
        }

        // ====================================================================
        // WM_LBUTTONUP - Botão esquerdo do mouse solto
        // ====================================================================
        WM_LBUTTONUP => {
            let state = get_state(hwnd);
            if let Some(state) = state {
                if state.is_dragging {
                    let x = (lparam & 0xFFFF) as i16 as i32;
                    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                    state.current_point = POINT { x, y };
                    state.is_dragging = false;

                    // Calcula região final
                    if let Some(start) = state.start_point {
                        let x1 = start.x.min(x);
                        let y1 = start.y.min(y);
                        let x2 = start.x.max(x);
                        let y2 = start.y.max(y);

                        let width = x2 - x1;
                        let height = y2 - y1;

                        // Só aceita se tiver tamanho mínimo (evita clique acidental)
                        if width > 5 && height > 5 {
                            let region = SelectedRegion {
                                x: x1 as u32,
                                y: y1 as u32,
                                width: width as u32,
                                height: height as u32,
                            };

                            // Salva no estado e na variável global
                            state.result = Some(region.clone());
                            *SELECTOR_RESULT.lock().unwrap() = Some(Some(region));
                        }
                    }

                    // Fecha a janela
                    DestroyWindow(hwnd);
                }
            }
            0
        }

        // ====================================================================
        // WM_KEYDOWN - Tecla pressionada
        // ====================================================================
        WM_KEYDOWN => {
            // ESC cancela a seleção
            if wparam == VK_ESCAPE as usize {
                let state = get_state(hwnd);
                if let Some(state) = state {
                    state.cancelled = true;
                }
                *SELECTOR_RESULT.lock().unwrap() = Some(None);
                DestroyWindow(hwnd);
            }
            0
        }

        // ====================================================================
        // WM_PAINT - Janela precisa ser redesenhada
        // ====================================================================
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            // Limpa a janela inteira com preto (fica semi-transparente por causa do LWA_ALPHA)
            // Isso remove artefatos do retângulo anterior
            let mut client_rect: RECT = mem::zeroed();
            GetClientRect(hwnd, &mut client_rect);
            let bg_brush = CreateSolidBrush(0x00000000); // Preto
            FillRect(hdc, &client_rect, bg_brush);
            DeleteObject(bg_brush as *mut _);

            // Se estiver arrastando, desenha o retângulo de seleção
            let state = get_state(hwnd);
            if let Some(state) = state {
                if state.is_dragging {
                    if let Some(start) = state.start_point {
                        let x1 = start.x.min(state.current_point.x);
                        let y1 = start.y.min(state.current_point.y);
                        let x2 = start.x.max(state.current_point.x);
                        let y2 = start.y.max(state.current_point.y);

                        // --- Preenchimento semi-transparente ---
                        // GDI não tem transparência real, então usamos
                        // um azul escuro que se destaca visualmente
                        let fill_brush = CreateSolidBrush(0x00553300); // BGR: azul escuro
                        let fill_rect = RECT {
                            left: x1,
                            top: y1,
                            right: x2,
                            bottom: y2,
                        };
                        FillRect(hdc, &fill_rect, fill_brush);
                        DeleteObject(fill_brush as *mut _);

                        // --- Borda do retângulo ---
                        // Caneta azul brilhante, 2 pixels de espessura
                        let pen = CreatePen(PS_SOLID as i32, 2, 0x00FF6600); // BGR: azul
                        let old_pen = SelectObject(hdc, pen as *mut _);

                        // Pincel nulo para não preencher por cima do que já fizemos
                        let null_brush = wingdi_get_stock_object(5); // HOLLOW_BRUSH = 5
                        let old_brush = SelectObject(hdc, null_brush);

                        // Desenha retângulo (só borda, preenchimento já foi feito)
                        Rectangle(hdc, x1, y1, x2, y2);

                        // Restaura objetos GDI originais
                        SelectObject(hdc, old_pen);
                        SelectObject(hdc, old_brush);
                        DeleteObject(pen as *mut _);

                        // --- Texto com dimensões ---
                        let width = x2 - x1;
                        let height = y2 - y1;
                        let info_text = format!("{}x{}", width, height);
                        let wide_text = wide_string(&info_text);

                        // Posiciona o texto acima do retângulo
                        let text_x = x1;
                        let text_y = y1 - 20;

                        SetBkMode(hdc, TRANSPARENT as i32);
                        SetTextColor(hdc, 0x00FFFFFF); // Branco

                        TextOutW(
                            hdc,
                            text_x,
                            text_y,
                            wide_text.as_ptr(),
                            info_text.len() as i32,
                        );
                    }
                }
            }

            EndPaint(hwnd, &ps);
            0
        }

        // ====================================================================
        // WM_DESTROY - Janela está sendo destruída
        // ====================================================================
        WM_DESTROY => {
            // Recupera e libera o SelectorState que alocamos com Box
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SelectorState;
            if !state_ptr.is_null() {
                // Box::from_raw reconstrói o Box, que será liberado automaticamente
                let _ = Box::from_raw(state_ptr);
            }

            // Envia WM_QUIT para encerrar o loop de mensagens
            PostQuitMessage(0);
            0
        }

        // ====================================================================
        // OUTRAS MENSAGENS - Processamento padrão do Windows
        // ====================================================================
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ============================================================================
// FUNÇÕES AUXILIARES
// ============================================================================

/// Recupera o SelectorState associado à janela
///
/// Usa GWLP_USERDATA para pegar o ponteiro que armazenamos em create_selector_window.
/// Retorna None se o ponteiro for nulo (janela ainda não foi inicializada).
unsafe fn get_state<'a>(hwnd: HWND) -> Option<&'a mut SelectorState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SelectorState;
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

/// Pega um "stock object" do GDI (objetos pré-definidos do Windows)
///
/// Usamos para pegar HOLLOW_BRUSH (pincel vazio), que desenha retângulo
/// sem preenchimento (só a borda).
unsafe fn wingdi_get_stock_object(index: i32) -> *mut winapi::ctypes::c_void {
    // GetStockObject está em wingdi
    winapi::um::wingdi::GetStockObject(index)
}

/// Converte uma string Rust (&str) para formato wide string do Windows (UTF-16)
///
/// O Windows API usa strings UTF-16 (cada caractere = 2 bytes).
/// Rust usa UTF-8. Esta função faz a conversão e adiciona o \0 final
/// que o Windows espera.
fn wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16() // Converte cada caractere para UTF-16
        .chain(Some(0)) // Adiciona \0 no final (null terminator)
        .collect() // Coleta em um Vec<u16>
}
