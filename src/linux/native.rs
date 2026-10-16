//! ABI mínima de bibliotecas nativas. Los opcodes corresponden al XML oficial.
//! Se usa wl_proxy_marshal_flags como los wrappers inline de los headers Wayland.

use std::ffi::{c_char, c_void};

pub type Proxy = *mut c_void;

pub const WL_SHM_FORMAT_ARGB8888: u32 = 0;
pub const WL_SHM_FORMAT_RGB565: u32 = 0x36314752;

#[repr(C)]
pub struct Interface {
    pub name: *const c_char,
    pub version: i32,
    pub method_count: i32,
    pub methods: *const c_void,
    pub event_count: i32,
    pub events: *const c_void,
}

#[repr(C)]
pub struct Array {
    pub size: usize,
    pub alloc: usize,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

#[link(name = "wayland-client")]
unsafe extern "C" {
    pub static wl_registry_interface: Interface;
    pub static wl_compositor_interface: Interface;
    pub static wl_surface_interface: Interface;
    pub static wl_shm_interface: Interface;
    pub static wl_shm_pool_interface: Interface;
    pub static wl_buffer_interface: Interface;
    pub static wl_seat_interface: Interface;
    pub static wl_keyboard_interface: Interface;
    pub static wl_pointer_interface: Interface;
    pub static wl_callback_interface: Interface;
    pub fn wl_display_connect(name: *const c_char) -> Proxy;
    pub fn wl_display_disconnect(display: Proxy);
    pub fn wl_display_roundtrip(display: Proxy) -> i32;
    pub fn wl_display_dispatch_pending(display: Proxy) -> i32;
    pub fn wl_display_dispatch(display: Proxy) -> i32;
    pub fn wl_display_flush(display: Proxy) -> i32;
    pub fn wl_display_get_fd(display: Proxy) -> i32;
    pub fn wl_proxy_destroy(proxy: Proxy);
    pub fn wl_proxy_add_listener(
        proxy: Proxy,
        implementation: *const c_void,
        data: *mut c_void,
    ) -> i32;
    pub fn wl_proxy_marshal_flags(
        proxy: Proxy,
        opcode: u32,
        interface: *const Interface,
        version: u32,
        flags: u32,
        ...
    ) -> Proxy;
}

// Símbolos del XML xdg-shell generados por wayland-scanner en build.rs.
unsafe extern "C" {
    pub static xdg_wm_base_interface: Interface;
    pub static xdg_surface_interface: Interface;
    pub static xdg_toplevel_interface: Interface;
}

#[link(name = "xkbcommon")]
unsafe extern "C" {
    pub fn xkb_context_new(flags: u32) -> Proxy;
    pub fn xkb_context_unref(context: Proxy);
    pub fn xkb_keymap_new_from_string(
        context: Proxy,
        text: *const c_char,
        format: u32,
        flags: u32,
    ) -> Proxy;
    pub fn xkb_keymap_unref(keymap: Proxy);
    pub fn xkb_state_new(keymap: Proxy) -> Proxy;
    pub fn xkb_state_unref(state: Proxy);
    pub fn xkb_state_update_mask(
        state: Proxy,
        depressed: u32,
        latched: u32,
        locked: u32,
        depressed_layout: u32,
        latched_layout: u32,
        locked_layout: u32,
    ) -> u32;
    pub fn xkb_state_key_get_one_sym(state: Proxy, key: u32) -> u32;
    pub fn xkb_state_key_get_utf8(state: Proxy, key: u32, buffer: *mut c_char, size: usize) -> i32;
    pub fn xkb_state_mod_name_is_active(state: Proxy, name: *const c_char, components: u32) -> i32;
    pub fn xkb_keymap_key_repeats(keymap: Proxy, key: u32) -> i32;
}

unsafe extern "C" {
    pub fn memfd_create(name: *const c_char, flags: u32) -> i32;
    pub fn mmap(
        address: *mut c_void,
        length: usize,
        protection: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
    pub fn munmap(address: *mut c_void, length: usize) -> i32;
    pub fn poll(fds: *mut PollFd, count: usize, timeout: i32) -> i32;
}

pub const POLLIN: i16 = 1;
pub const POLLOUT: i16 = 4;
pub const POLLERR: i16 = 8;
pub const POLLHUP: i16 = 16;
pub const WL_MARSHAL_FLAG_DESTROY: u32 = 1;
// Requests del protocolo estable. Las versiones negociadas se fijan al enlazar.
pub const WL_DISPLAY_GET_REGISTRY: u32 = 1;
pub const WL_REGISTRY_BIND: u32 = 0;
pub const WL_COMPOSITOR_CREATE_SURFACE: u32 = 0;
pub const WL_SHM_CREATE_POOL: u32 = 0;
pub const WL_SHM_POOL_CREATE_BUFFER: u32 = 0;
pub const WL_SHM_POOL_DESTROY: u32 = 1;
pub const WL_BUFFER_DESTROY: u32 = 0;
pub const WL_SEAT_GET_POINTER: u32 = 0;
pub const WL_SEAT_GET_KEYBOARD: u32 = 1;
pub const WL_KEYBOARD_RELEASE: u32 = 0;
pub const WL_POINTER_RELEASE: u32 = 1;
pub const WL_SURFACE_ATTACH: u32 = 1;
pub const WL_SURFACE_DAMAGE: u32 = 2;
pub const WL_SURFACE_FRAME: u32 = 3;
pub const WL_SURFACE_COMMIT: u32 = 6;
pub const XDG_WM_BASE_GET_XDG_SURFACE: u32 = 2;
pub const XDG_WM_BASE_PONG: u32 = 3;
pub const XDG_SURFACE_GET_TOPLEVEL: u32 = 1;
pub const XDG_SURFACE_ACK_CONFIGURE: u32 = 4;
pub const XDG_TOPLEVEL_SET_TITLE: u32 = 2;
pub const XDG_TOPLEVEL_SET_APP_ID: u32 = 3;
pub const XDG_TOPLEVEL_SET_MIN_SIZE: u32 = 8;
pub const KEY_BACKSPACE: u32 = 0xff08;
pub const KEY_RETURN: u32 = 0xff0d;
pub const KEY_KP_ENTER: u32 = 0xff8d;
pub const KEY_DELETE: u32 = 0xffff;
pub const KEY_HOME: u32 = 0xff50;
pub const KEY_LEFT: u32 = 0xff51;
pub const KEY_RIGHT: u32 = 0xff53;
pub const KEY_END: u32 = 0xff57;
pub const KEY_PAGE_UP: u32 = 0xff55;
pub const KEY_PAGE_DOWN: u32 = 0xff56;

/// Destruye solo un proxy local; el cierre de conexión libera el recurso remoto.
/// # Safety
/// Proxy vivo y propiedad exclusiva del hilo; no puede usarse después.
pub unsafe fn destroy(proxy: Proxy) {
    if !proxy.is_null() {
        unsafe {
            wl_proxy_destroy(proxy);
        }
    }
}
