//! ABI y constantes de Win32 usadas exclusivamente por el adaptador Windows.
//! repr(C) y extern system conservan layout y convencion de llamada del SDK.
//! Los handles prestados no se liberan aqui; Ui posee ventana y fuente.

use std::ffi::c_void;

/// HANDLE/HWND/HFONT opacos; su propiedad depende de la API que los devuelve.
pub(super) type Handle = *mut c_void;
/// ABI WNDPROC: WPARAM tiene ancho de puntero y LPARAM/LRESULT son enteros con signo.
pub(super) type WindowProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;

/// WNDCLASSW: el padding de punteros lo determina repr(C), sin empaquetado manual.
#[repr(C)]
pub(super) struct WindowClass {
    pub(super) style: u32,
    pub(super) procedure: Option<WindowProc>,
    pub(super) class_extra: i32,
    pub(super) window_extra: i32,
    pub(super) instance: Handle,
    pub(super) icon: Handle,
    pub(super) cursor: Handle,
    pub(super) background: Handle,
    pub(super) menu_name: *const u16,
    pub(super) class_name: *const u16,
}

/// RECT: coordenadas LONG de 32 bits incluso en Windows de 64 bits.
#[repr(C)]
#[derive(Default)]
pub(super) struct Rect {
    pub(super) left: i32,
    pub(super) top: i32,
    pub(super) right: i32,
    pub(super) bottom: i32,
}

/// MSG: incluye POINT como dos LONG y lPrivate; compatible con el layout del SDK.
#[repr(C)]
#[derive(Default)]
pub(super) struct Message {
    pub(super) window: Handle,
    pub(super) kind: u32,
    pub(super) wparam: usize,
    pub(super) lparam: isize,
    pub(super) time: u32,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) private: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    pub(super) fn RegisterClassW(class: *const WindowClass) -> u16;
    pub(super) fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        parameter: *mut c_void,
    ) -> Handle;
    pub(super) fn DefWindowProcW(window: Handle, kind: u32, wparam: usize, lparam: isize) -> isize;
    pub(super) fn DestroyWindow(window: Handle) -> i32;
    pub(super) fn IsWindow(window: Handle) -> i32;
    pub(super) fn PostQuitMessage(code: i32);
    pub(super) fn GetMessageW(message: *mut Message, window: Handle, first: u32, last: u32) -> i32;
    pub(super) fn TranslateMessage(message: *const Message) -> i32;
    pub(super) fn DispatchMessageW(message: *const Message) -> isize;
    pub(super) fn ShowWindow(window: Handle, command: i32) -> i32;
    pub(super) fn LoadCursorW(instance: Handle, name: *const u16) -> Handle;
    pub(super) fn GetClientRect(window: Handle, rect: *mut Rect) -> i32;
    pub(super) fn GetDlgItem(window: Handle, id: i32) -> Handle;
    pub(super) fn MoveWindow(
        window: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        repaint: i32,
    ) -> i32;
    pub(super) fn SetFocus(window: Handle) -> Handle;
    pub(super) fn SendMessageW(window: Handle, kind: u32, wparam: usize, lparam: isize) -> isize;
    pub(super) fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
    pub(super) fn GetWindowTextW(window: Handle, text: *mut u16, capacity: i32) -> i32;
    pub(super) fn MessageBoxW(
        window: Handle,
        text: *const u16,
        title: *const u16,
        kind: u32,
    ) -> i32;
    pub(super) fn SetTimer(
        window: Handle,
        id: usize,
        milliseconds: u32,
        callback: *const c_void,
    ) -> usize;
    pub(super) fn KillTimer(window: Handle, id: usize) -> i32;
    pub(super) fn IsIconic(window: Handle) -> i32;
    pub(super) fn InvalidateRect(window: Handle, rect: *const Rect, erase: i32) -> i32;
}

// Nombres y valores del SDK: mensajes, estilos y recursos compartidos del SO.
pub(super) const WM_DESTROY: u32 = 0x0002;
pub(super) const WM_SIZE: u32 = 0x0005;
pub(super) const WM_SETFOCUS: u32 = 0x0007;
pub(super) const WM_CLOSE: u32 = 0x0010;
pub(super) const WM_SETREDRAW: u32 = 0x000B;
pub(super) const WM_SETFONT: u32 = 0x0030;
pub(super) const WM_KEYDOWN: u32 = 0x0100;
pub(super) const WM_TIMER: u32 = 0x0113;
pub(super) const EM_SETLIMITTEXT: u32 = 0x00C5;
pub(super) const LB_ADDSTRING: u32 = 0x0180;
pub(super) const LB_RESETCONTENT: u32 = 0x0184;
pub(super) const LB_SETCURSEL: u32 = 0x0186;
pub(super) const LB_GETCURSEL: u32 = 0x0188;
pub(super) const LB_GETTOPINDEX: u32 = 0x018E;
pub(super) const LB_SETTOPINDEX: u32 = 0x0197;
pub(super) const LB_ERR: isize = -1;
pub(super) const LB_ERRSPACE: isize = -2;
pub(super) const VK_RETURN: usize = 13;
pub(super) const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
pub(super) const WS_CLIPCHILDREN: u32 = 0x02000000;
pub(super) const WS_CHILD: u32 = 0x40000000;
pub(super) const WS_VISIBLE: u32 = 0x10000000;
pub(super) const WS_VSCROLL: u32 = 0x00200000;
pub(super) const WS_TABSTOP: u32 = 0x00010000;
pub(super) const WS_EX_CLIENTEDGE: u32 = 0x00000200;
pub(super) const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
pub(super) const SS_LEFTNOWORDWRAP: u32 = 0x0080;
pub(super) const ES_AUTOHSCROLL: u32 = 0x0080;
pub(super) const IDC_ARROW: usize = 32512;
pub(super) const COLOR_WINDOW_BRUSH: usize = 6; // COLOR_WINDOW + 1, pseudo-handle.
pub(super) const SW_SHOW: i32 = 5;
pub(super) const MB_ICONERROR: u32 = 0x10;
pub(super) const SND_SYNC: u32 = 0;
pub(super) const SND_NODEFAULT: u32 = 0x0002;
pub(super) const SND_ALIAS: u32 = 0x00010000;

#[link(name = "winmm")]
unsafe extern "system" {
    pub(super) fn PlaySoundW(sound: *const u16, module: Handle, flags: u32) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub(super) fn GetModuleHandleW(name: *const u16) -> Handle;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    pub(super) fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strikeout: u32,
        charset: u32,
        output_precision: u32,
        clip_precision: u32,
        quality: u32,
        pitch: u32,
        face: *const u16,
    ) -> Handle;
    pub(super) fn DeleteObject(object: Handle) -> i32;
}
