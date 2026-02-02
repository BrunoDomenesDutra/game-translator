// game-translator/src/region_selector.rs

// ============================================================================
// MÓDULO REGION SELECTOR - Seleção visual usando overlay transparente
// ============================================================================
//
// Este módulo cria uma janela transparente por cima de TUDO na tela,
// permitindo ao usuário clicar e arrastar para selecionar uma região.
//
// Usa UpdateLayeredWindow com per-pixel alpha para ter controle total
// de transparência: a área ao redor da seleção fica escurecida,
// enquanto a área de seleção fica 100% transparente (mostra a tela real).
//
// Tecnologias usadas:
// - winapi: Criação de janela Win32, mensagens, GDI + bitmap BGRA
// - UpdateLayeredWindow: transparência per-pixel (cada pixel tem seu alpha)
// - Nenhuma dependência externa além do winapi (já no Cargo.toml)
//
// ============================================================================

use anyhow::Result;

// Imports do Windows API
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HWND, POINT, SIZE};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::{
    // Funções GDI para bitmap em memória
    CreateCompatibleDC, // Cria um "Device Context" compatível na memória
    // Funções para criar bitmap com acesso direto aos pixels
    CreateDIBSection, // Cria bitmap onde controlamos cada pixel (BGRA)
    DeleteDC,         // Libera o DC de memória
    DeleteObject,     // Libera objetos GDI
    SelectObject,     // Seleciona bitmap no DC
    // Estruturas de bitmap
    BITMAPINFO,       // Info do bitmap (tamanho, formato)
    BITMAPINFOHEADER, // Cabeçalho do bitmap
    BI_RGB,           // Formato sem compressão
    DIB_RGB_COLORS,   // Modo de cores RGB
};
use winapi::um::winuser::{
    // Funções de janela
    CreateWindowExW,
    DefWindowProcW,
    DestroyWindow,
    DispatchMessageW,
    GetMessageW,
    GetSystemMetrics,
    GetWindowLongPtrW,
    LoadCursorW,
    PostQuitMessage,
    RegisterClassExW,
    SetCursor,
    SetWindowLongPtrW,
    ShowWindow,
    TranslateMessage,
    // UpdateLayeredWindow - o coração da transparência per-pixel
    UpdateLayeredWindow,
    UpdateWindow,
    // Constantes
    GWLP_USERDATA,
    IDC_CROSS,
    MSG,
    SM_CXSCREEN,
    SM_CYSCREEN,
    SW_SHOW,
    ULW_ALPHA, // Flag para usar alpha per-pixel
    VK_ESCAPE,
    WM_CREATE,
    WM_DESTROY,
    WM_ERASEBKGND,
    WM_KEYDOWN,
    WM_LBUTTONDOWN,
    WM_LBUTTONUP,
    WM_MOUSEMOVE,
    WM_PAINT,
    WM_SETCURSOR,
    WNDCLASSEXW,
    WS_EX_LAYERED,
    WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
    WS_POPUP,
    WS_VISIBLE,
};

// Imports da biblioteca padrão do Rust
use std::mem;
use std::ptr;
use std::sync::Mutex;

// ============================================================================
// CONSTANTES DE APARÊNCIA
// ============================================================================

/// Opacidade do escurecimento ao redor da seleção (0-255)
/// 120 ≈ 47% de opacidade — escurece sem ficar pesado
const OVERLAY_ALPHA: u8 = 120;

/// Cor da borda da seleção em BGR (azul brilhante)
const BORDER_COLOR_B: u8 = 0xFF;
const BORDER_COLOR_G: u8 = 0x66;
const BORDER_COLOR_R: u8 = 0x00;

/// Espessura da borda em pixels
const BORDER_WIDTH: i32 = 3;

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
    /// Largura da tela em pixels
    screen_width: i32,
    /// Altura da tela em pixels
    screen_height: i32,
}

// ============================================================================
// VARIÁVEIS GLOBAIS
// ============================================================================

/// Resultado da seleção (sobrevive após a janela ser destruída)
static SELECTOR_RESULT: Mutex<Option<Option<SelectedRegion>>> = Mutex::new(None);

/// Título exibido no topo da tela durante a seleção
static SELECTOR_TITLE: Mutex<Option<String>> = Mutex::new(None);

// ============================================================================
// FUNÇÃO PÚBLICA - PONTO DE ENTRADA
// ============================================================================

/// Abre a interface de seleção de região e retorna a região selecionada
///
/// Cria um overlay transparente sobre toda a tela. O usuário clica e
/// arrasta para selecionar uma região. ESC cancela.
///
/// # Parâmetros
/// * `title` - Texto opcional exibido no topo da tela (ex: "SELEÇÃO ÁREA DE LEGENDA")
///
/// # Retorna
/// * `Ok(Some(SelectedRegion))` - Região selecionada com sucesso
/// * `Ok(None)` - Usuário cancelou (ESC)
/// * `Err(...)` - Erro ao criar janela
pub fn select_region(title: Option<&str>) -> Result<Option<SelectedRegion>> {
    info!("🎯 Iniciando seletor de região (overlay transparente)...");

    // Limpa resultado anterior
    *SELECTOR_RESULT.lock().unwrap() = None;
    *SELECTOR_TITLE.lock().unwrap() = title.map(|s| s.to_string());

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
unsafe fn create_selector_window() -> Result<()> {
    // ========================================================================
    // PASSO 1: Registrar a classe da janela
    // ========================================================================
    let class_name = wide_string("GameTranslatorSelector");
    let hinstance = GetModuleHandleW(ptr::null());

    let wc = WNDCLASSEXW {
        cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: ptr::null_mut(),
        hCursor: LoadCursorW(ptr::null_mut(), IDC_CROSS),
        hbrBackground: ptr::null_mut(),
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
    // WS_EX_TOPMOST    = Sempre por cima de todas as janelas
    // WS_EX_LAYERED    = Suporta transparência (necessário para UpdateLayeredWindow)
    // WS_EX_TOOLWINDOW = Não mostra na barra de tarefas
    // WS_POPUP         = Sem borda, sem título
    // WS_VISIBLE       = Já começa visível
    //
    // IMPORTANTE: NÃO usamos SetLayeredWindowAttributes aqui!
    // Em vez disso, usamos UpdateLayeredWindow que dá controle per-pixel.
    //
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
        class_name.as_ptr(),
        wide_string("Seletor de Região").as_ptr(),
        WS_POPUP | WS_VISIBLE,
        0,
        0,
        screen_width,
        screen_height,
        ptr::null_mut(),
        ptr::null_mut(),
        hinstance,
        ptr::null_mut(),
    );

    if hwnd.is_null() {
        anyhow::bail!("Falha ao criar janela do seletor de região");
    }

    // ========================================================================
    // PASSO 4: Criar estado e associar à janela
    // ========================================================================
    let state = Box::new(SelectorState {
        start_point: None,
        current_point: POINT { x: 0, y: 0 },
        is_dragging: false,
        result: None,
        cancelled: false,
        screen_width,
        screen_height,
    });

    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

    // ========================================================================
    // PASSO 5: Desenhar overlay inicial (tela escurecida, sem seleção)
    // ========================================================================
    render_overlay(hwnd);

    // ========================================================================
    // PASSO 6: Mostrar janela e iniciar loop de mensagens
    // ========================================================================
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);

    info!("✅ Janela do seletor aberta. Clique e arraste para selecionar. ESC para cancelar.");

    let mut msg: MSG = mem::zeroed();
    while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    Ok(())
}

// ============================================================================
// RENDERIZAÇÃO DO OVERLAY (UpdateLayeredWindow)
// ============================================================================
//
// Esta é a função principal de desenho. Em vez de usar WM_PAINT + GDI,
// criamos um bitmap 32-bit BGRA na memória, pintamos cada pixel com
// a transparência desejada, e chamamos UpdateLayeredWindow.
//
// Formato de cada pixel no bitmap: [B, G, R, A] (4 bytes)
// - B, G, R = cor do pixel
// - A = alpha (0 = totalmente transparente, 255 = totalmente opaco)
//
// IMPORTANTE: Com per-pixel alpha, as cores precisam ser "premultiplied".
// Isso significa que cada componente de cor (R, G, B) é multiplicado pelo
// alpha antes de ser armazenado. Ex: azul com alpha 200:
//   B = 255 * 200 / 255 = 200
//   G = 0
//   R = 0
//   A = 200
//
// ============================================================================

/// Renderiza o overlay completo e atualiza a janela via UpdateLayeredWindow
///
/// Cria um bitmap BGRA, pinta as áreas com suas respectivas transparências,
/// e aplica na janela. Chamada em cada frame (mouse move, início, etc).
unsafe fn render_overlay(hwnd: HWND) {
    let state = get_state(hwnd);
    if state.is_none() {
        return;
    }
    let state = state.unwrap();

    let w = state.screen_width;
    let h = state.screen_height;

    // ========================================================================
    // PASSO 1: Criar DC e bitmap em memória
    // ========================================================================
    //
    // CreateCompatibleDC: cria um "Device Context" virtual na memória
    // CreateDIBSection: cria um bitmap onde temos acesso direto aos pixels
    //
    // O bitmap é 32-bit (4 bytes por pixel): B, G, R, Alpha
    //

    let hdc_screen = winapi::um::winuser::GetDC(ptr::null_mut()); // DC da tela
    let hdc_mem = CreateCompatibleDC(hdc_screen); // DC em memória

    // Configura o formato do bitmap
    let mut bmi: BITMAPINFO = mem::zeroed();
    bmi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = w;
    bmi.bmiHeader.biHeight = -h; // Negativo = top-down (linha 0 no topo)
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32; // 32 bits = BGRA
    bmi.bmiHeader.biCompression = BI_RGB as u32;

    // Cria o bitmap e recebe ponteiro para os pixels
    let mut bits_ptr: *mut winapi::ctypes::c_void = ptr::null_mut();
    let hbitmap = CreateDIBSection(
        hdc_mem,
        &bmi,
        DIB_RGB_COLORS,
        &mut bits_ptr,
        ptr::null_mut(),
        0,
    );

    if hbitmap.is_null() || bits_ptr.is_null() {
        DeleteDC(hdc_mem);
        winapi::um::winuser::ReleaseDC(ptr::null_mut(), hdc_screen);
        return;
    }

    // Seleciona o bitmap no DC de memória
    let old_bitmap = SelectObject(hdc_mem, hbitmap as *mut _);

    // ========================================================================
    // PASSO 2: Pintar os pixels
    // ========================================================================
    //
    // Acessamos o buffer de pixels diretamente como um slice de bytes.
    // Cada pixel = 4 bytes: [B, G, R, A]
    // Total de pixels = largura * altura
    //
    let pixel_count = (w * h) as usize;
    let pixels = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, pixel_count * 4);

    // Calcula coordenadas da seleção (se estiver arrastando)
    let selection = if state.is_dragging {
        state.start_point.map(|start| {
            let x1 = start.x.min(state.current_point.x);
            let y1 = start.y.min(state.current_point.y);
            let x2 = start.x.max(state.current_point.x);
            let y2 = start.y.max(state.current_point.y);
            (x1, y1, x2, y2)
        })
    } else {
        None
    };

    // Pinta cada pixel
    // Premultiplied alpha: cor = cor_original * alpha / 255
    let dark_r: u8 = 0; // Preto
    let dark_g: u8 = 0;
    let dark_b: u8 = 0;
    let dark_a: u8 = OVERLAY_ALPHA;
    // Premultiplied: preto * alpha = 0, então R=G=B=0 independente do alpha
    // (preto é o caso mais simples pois 0 * qualquer coisa = 0)

    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) as usize) * 4;

            // Verifica se o pixel está dentro da área de seleção
            let in_selection = selection.map_or(false, |(x1, y1, x2, y2)| {
                x >= x1 && x < x2 && y >= y1 && y < y2
            });

            // Verifica se o pixel está na borda da seleção
            let in_border = selection.map_or(false, |(x1, y1, x2, y2)| {
                let bw = BORDER_WIDTH;
                // Está dentro da borda se está na "moldura" de BORDER_WIDTH pixels
                x >= x1 - bw
                    && x < x2 + bw
                    && y >= y1 - bw
                    && y < y2 + bw
                    && !(x >= x1 && x < x2 && y >= y1 && y < y2)
            });

            if in_border {
                // Borda: cor azul, totalmente opaca
                // Premultiplied: como alpha=255, cor fica igual
                pixels[idx] = BORDER_COLOR_B; // B
                pixels[idx + 1] = BORDER_COLOR_G; // G
                pixels[idx + 2] = BORDER_COLOR_R; // R
                pixels[idx + 3] = 255; // A = opaco
            } else if in_selection {
                // Dentro da seleção: totalmente transparente (mostra tela real)
                pixels[idx] = 0; // B
                pixels[idx + 1] = 0; // G
                pixels[idx + 2] = 0; // R
                pixels[idx + 3] = 0; // A = transparente
            } else {
                // Área escurecida: preto semi-transparente
                pixels[idx] = dark_b; // B (premultiplied = 0)
                pixels[idx + 1] = dark_g; // G (premultiplied = 0)
                pixels[idx + 2] = dark_r; // R (premultiplied = 0)
                pixels[idx + 3] = dark_a; // A = 120
            }
        }
    }

    // ========================================================================
    // PASSO 3: Desenhar texto no bitmap (título e dimensões)
    // ========================================================================

    // Desenha título centralizado (se houver)
    if let Some(ref title) = *SELECTOR_TITLE.lock().unwrap() {
        draw_text_on_bitmap(pixels, w, h, title, w / 2, 50, true);
    }

    // Desenha dimensões da seleção acima do retângulo
    if let Some((x1, y1, x2, y2)) = selection {
        let width = x2 - x1;
        let height = y2 - y1;
        let dim_text = format!("{}x{}", width, height);
        draw_text_on_bitmap(pixels, w, h, &dim_text, x1, y1 - 18, false);
    }

    // ========================================================================
    // PASSO 4: Aplicar o bitmap na janela via UpdateLayeredWindow
    // ========================================================================
    //
    // UpdateLayeredWindow recebe:
    // - hdcSrc: nosso DC de memória com o bitmap pintado
    // - pptSrc: posição no bitmap (0,0 = começo)
    // - psize: tamanho da janela
    // - pblend: configuração de blending (AC_SRC_ALPHA para per-pixel)
    //
    let pt_zero = POINT { x: 0, y: 0 };
    let size = SIZE { cx: w, cy: h };

    // BLENDFUNCTION configura como o bitmap é misturado com a tela
    let blend = winapi::um::wingdi::BLENDFUNCTION {
        BlendOp: 0,               // AC_SRC_OVER (padrão)
        BlendFlags: 0,            // Sempre 0
        SourceConstantAlpha: 255, // 255 = usar alpha de cada pixel (não global)
        AlphaFormat: 1,           // AC_SRC_ALPHA = bitmap tem alpha per-pixel
    };

    UpdateLayeredWindow(
        hwnd,
        hdc_screen,                             // DC destino (tela)
        &pt_zero as *const POINT as *mut POINT, // Posição da janela na tela
        &size as *const SIZE as *mut SIZE,      // Tamanho
        hdc_mem,                                // DC fonte (nosso bitmap)
        &pt_zero as *const POINT as *mut POINT, // Posição no bitmap
        0,                                      // Cor de transparência (não usada com ULW_ALPHA)
        &blend as *const _ as *mut _,           // Configuração de blending
        ULW_ALPHA,                              // Usar alpha per-pixel
    );

    // ========================================================================
    // PASSO 5: Limpar recursos
    // ========================================================================
    SelectObject(hdc_mem, old_bitmap);
    DeleteObject(hbitmap as *mut _);
    DeleteDC(hdc_mem);
    winapi::um::winuser::ReleaseDC(ptr::null_mut(), hdc_screen);
}

// ============================================================================
// DESENHO DE TEXTO NO BITMAP
// ============================================================================
//
// Como estamos desenhando direto no bitmap (não via GDI), o texto precisa
// ser rasterizado manualmente. Usamos uma fonte bitmap 5x7 simples.
// Não é bonita, mas funciona sem dependências externas.
//
// Cada caractere é definido como 5 colunas × 7 linhas de pixels.
//

/// Desenha texto diretamente no buffer de pixels do bitmap
///
/// # Parâmetros
/// * `pixels` - Buffer BGRA do bitmap
/// * `bw` / `bh` - Largura e altura do bitmap
/// * `text` - Texto a desenhar
/// * `tx` / `ty` - Posição X, Y (se centered=true, X é o centro)
/// * `centered` - Se true, centraliza o texto em X
fn draw_text_on_bitmap(
    pixels: &mut [u8],
    bw: i32,
    bh: i32,
    text: &str,
    tx: i32,
    ty: i32,
    centered: bool,
) {
    let char_w = 8; // Largura de cada caractere (5 pixels + 3 espaço)
    let text_width = text.len() as i32 * char_w;

    let start_x = if centered { tx - text_width / 2 } else { tx };

    // Desenha cada caractere
    for (i, ch) in text.chars().enumerate() {
        let cx = start_x + (i as i32) * char_w;
        let glyph = get_glyph(ch);

        // Cada glyph é 7 linhas de 5 bits
        for row in 0..7 {
            for col in 0..5 {
                // Verifica se o bit está ligado nesta posição
                if (glyph[row] >> (4 - col)) & 1 == 1 {
                    let px = cx + col as i32;
                    let py = ty + row as i32;

                    // Desenha pixel branco opaco (se dentro dos limites)
                    if px >= 0 && px < bw && py >= 0 && py < bh {
                        let idx = ((py * bw + px) as usize) * 4;
                        pixels[idx] = 255; // B
                        pixels[idx + 1] = 255; // G
                        pixels[idx + 2] = 255; // R
                        pixels[idx + 3] = 255; // A
                    }
                }
            }
        }
    }
}

/// Retorna o glyph bitmap 5x7 de um caractere
///
/// Cada elemento do array é uma linha de 5 bits (bit mais significativo = esquerda)
/// Exemplo: 'A' = [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]
fn get_glyph(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        '0' => [
            0b01110, 0b10011, 0b10101, 0b10101, 0b10101, 0b11001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
        ],
        '3' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10101, 0b10011, 0b10011, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],

        ' ' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        // Caracteres acentuados comuns em português
        // Ã = A com til
        _ if ch == 'Ã' || ch == 'ã' => [
            0b01010, 0b00000, 0b01110, 0b10001, 0b11111, 0b10001, 0b10001,
        ],
        // Á = A com acento
        _ if ch == 'Á' || ch == 'á' => [
            0b00010, 0b00100, 0b01110, 0b10001, 0b11111, 0b10001, 0b10001,
        ],
        // É = E com acento
        _ if ch == 'É' || ch == 'é' => [
            0b00010, 0b00100, 0b11111, 0b10000, 0b11110, 0b10000, 0b11111,
        ],
        // Ç = C cedilha
        _ if ch == 'Ç' || ch == 'ç' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10001, 0b01110, 0b00100,
        ],
        // Ó = O com acento
        _ if ch == 'Ó' || ch == 'ó' => [
            0b00010, 0b00100, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        // Ú = U com acento
        _ if ch == 'Ú' || ch == 'ú' => [
            0b00010, 0b00100, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        // Qualquer outro caractere: quadrado preenchido
        _ => [
            0b11111, 0b11111, 0b11111, 0b11111, 0b11111, 0b11111, 0b11111,
        ],
    }
}

// ============================================================================
// CALLBACK DE MENSAGENS (WndProc)
// ============================================================================

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
        WM_CREATE => 0,

        // ====================================================================
        // WM_ERASEBKGND - Não precisa apagar fundo (UpdateLayeredWindow cuida)
        // ====================================================================
        WM_ERASEBKGND => 1,

        // ====================================================================
        // WM_PAINT - Não precisa pintar (UpdateLayeredWindow cuida)
        // ====================================================================
        WM_PAINT => {
            // Precisamos chamar BeginPaint/EndPaint para o Windows
            // parar de enviar WM_PAINT repetidamente
            let mut ps: winapi::um::winuser::PAINTSTRUCT = mem::zeroed();
            winapi::um::winuser::BeginPaint(hwnd, &mut ps);
            winapi::um::winuser::EndPaint(hwnd, &ps);
            0
        }

        // ====================================================================
        // WM_SETCURSOR - Cursor de cruz
        // ====================================================================
        WM_SETCURSOR => {
            SetCursor(LoadCursorW(ptr::null_mut(), IDC_CROSS));
            1
        }

        // ====================================================================
        // WM_LBUTTONDOWN - Início da seleção
        // ====================================================================
        WM_LBUTTONDOWN => {
            let state = get_state(hwnd);
            if let Some(state) = state {
                let x = (lparam & 0xFFFF) as i16 as i32;
                let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                state.start_point = Some(POINT { x, y });
                state.current_point = POINT { x, y };
                state.is_dragging = true;

                // Redesenha com a seleção inicial
                render_overlay(hwnd);
            }
            0
        }

        // ====================================================================
        // WM_MOUSEMOVE - Atualiza seleção durante arraste
        // ====================================================================
        WM_MOUSEMOVE => {
            let state = get_state(hwnd);
            if let Some(state) = state {
                if state.is_dragging {
                    let x = (lparam & 0xFFFF) as i16 as i32;
                    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                    state.current_point = POINT { x, y };

                    // Redesenha o overlay com a nova posição
                    render_overlay(hwnd);
                }
            }
            0
        }

        // ====================================================================
        // WM_LBUTTONUP - Fim da seleção
        // ====================================================================
        WM_LBUTTONUP => {
            let state = get_state(hwnd);
            if let Some(state) = state {
                if state.is_dragging {
                    let x = (lparam & 0xFFFF) as i16 as i32;
                    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                    state.current_point = POINT { x, y };
                    state.is_dragging = false;

                    if let Some(start) = state.start_point {
                        let x1 = start.x.min(x);
                        let y1 = start.y.min(y);
                        let x2 = start.x.max(x);
                        let y2 = start.y.max(y);

                        let width = x2 - x1;
                        let height = y2 - y1;

                        if width > 5 && height > 5 {
                            let region = SelectedRegion {
                                x: x1 as u32,
                                y: y1 as u32,
                                width: width as u32,
                                height: height as u32,
                            };

                            state.result = Some(region.clone());
                            *SELECTOR_RESULT.lock().unwrap() = Some(Some(region));
                        }
                    }

                    DestroyWindow(hwnd);
                }
            }
            0
        }

        // ====================================================================
        // WM_KEYDOWN - ESC cancela
        // ====================================================================
        WM_KEYDOWN => {
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
        // WM_DESTROY - Limpeza
        // ====================================================================
        WM_DESTROY => {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SelectorState;
            if !state_ptr.is_null() {
                let _ = Box::from_raw(state_ptr);
            }
            PostQuitMessage(0);
            0
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ============================================================================
// FUNÇÕES AUXILIARES
// ============================================================================

/// Recupera o SelectorState associado à janela
unsafe fn get_state<'a>(hwnd: HWND) -> Option<&'a mut SelectorState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SelectorState;
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

/// Converte string Rust para wide string do Windows (UTF-16 + null terminator)
fn wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
