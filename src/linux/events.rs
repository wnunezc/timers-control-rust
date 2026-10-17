//! Listeners Wayland: actualizan estado o encolan entrada; no despachan eventos
//! recursivamente ni ejecutan comandos/audio desde una llamada del compositor.

use super::{KeyEvent, State, native::*};
use std::{
    ffi::{CStr, c_char, c_void},
    fs::File,
    os::fd::FromRawFd,
    os::unix::fs::FileExt,
    ptr::{null, null_mut},
    time::{Duration, Instant},
};

/// # Safety
/// El proxy es vivo y listener tiene el layout del protocolo y vida estática.
pub unsafe fn listen<T>(
    proxy: Proxy,
    listener: &'static T,
    data: *mut c_void,
) -> Result<(), String> {
    if proxy.is_null() {
        return Err("No se pudo crear un objeto Wayland".into());
    }
    if unsafe { wl_proxy_add_listener(proxy, listener as *const T as *const c_void, data) } != 0 {
        return Err("No se pudo registrar un listener Wayland".into());
    }
    Ok(())
}

#[repr(C)]
pub struct RegistryListener {
    global: unsafe extern "C" fn(*mut c_void, Proxy, u32, *const c_char, u32),
    remove: unsafe extern "C" fn(*mut c_void, Proxy, u32),
}
pub static REGISTRY: RegistryListener = RegistryListener { global, remove };

unsafe extern "C" fn global(
    data: *mut c_void,
    registry: Proxy,
    name: u32,
    interface: *const c_char,
    version: u32,
) {
    // SAFETY: libwayland valida el mensaje; data apunta al State estable de la conexión.
    unsafe {
        let state = &mut *(data as *mut State);
        let kind = CStr::from_ptr(interface).to_bytes();
        let (target, metadata, required, slot) = match kind {
            b"wl_compositor" if state.compositor.is_null() => (
                &mut state.compositor,
                &raw const wl_compositor_interface,
                3,
                0,
            ),
            b"wl_shm" if state.shm.is_null() => (&mut state.shm, &raw const wl_shm_interface, 1, 1),
            b"xdg_wm_base" if state.wm.is_null() => {
                (&mut state.wm, &raw const xdg_wm_base_interface, 1, 2)
            }
            b"wl_seat" if state.seat.is_null() => {
                (&mut state.seat, &raw const wl_seat_interface, 5, 3)
            }
            _ => return,
        };
        if version < required {
            state.error = Some(format!(
                "Versión insuficiente de {}",
                CStr::from_ptr(interface).to_string_lossy()
            ));
            return;
        }
        *target = wl_proxy_marshal_flags(
            registry,
            WL_REGISTRY_BIND,
            metadata,
            required,
            0,
            name,
            (*metadata).name,
            required,
            null::<c_void>(),
        );
        let proxy = *target;
        if proxy.is_null() {
            state.error = Some("No se pudo enlazar un global Wayland".into());
            return;
        }
        state.globals[slot] = name;
        let result = match slot {
            1 => listen(proxy, &SHM, data),
            2 => listen(proxy, &WM, data),
            3 => listen(proxy, &SEAT, data),
            _ => Ok(()),
        };
        if let Err(error) = result {
            state.error = Some(error);
        }
    }
}

unsafe extern "C" fn remove(data: *mut c_void, _: Proxy, name: u32) {
    unsafe {
        let state = &mut *(data as *mut State);
        if state.globals.contains(&name) {
            state.error = Some("El compositor retiró un global en uso".into());
        }
    }
}

#[repr(C)]
struct ShmListener {
    format: unsafe extern "C" fn(*mut c_void, Proxy, u32),
}
static SHM: ShmListener = ShmListener { format: shm_format };
unsafe extern "C" fn shm_format(data: *mut c_void, _: Proxy, format: u32) {
    // SAFETY: listener de wl_shm; solo registra formatos anunciados por el servidor.
    unsafe {
        let state = &mut *(data as *mut State);
        match format {
            WL_SHM_FORMAT_ARGB8888 => state.argb = true,
            // wl_shm RGB565: opaco, R5:G6:B5 nativo, coincide con Cairo RGB16_565.
            WL_SHM_FORMAT_RGB565 => state.rgb565 = true,
            _ => {}
        }
    }
}

#[repr(C)]
struct WmListener {
    ping: unsafe extern "C" fn(*mut c_void, Proxy, u32),
}
static WM: WmListener = WmListener { ping };
unsafe extern "C" fn ping(_: *mut c_void, wm: Proxy, serial: u32) {
    unsafe {
        wl_proxy_marshal_flags(wm, XDG_WM_BASE_PONG, null(), 1, 0, serial);
    }
}

#[repr(C)]
pub struct SurfaceListener {
    configure: unsafe extern "C" fn(*mut c_void, Proxy, u32),
}
pub static SURFACE: SurfaceListener = SurfaceListener { configure };
unsafe extern "C" fn configure(data: *mut c_void, surface: Proxy, serial: u32) {
    unsafe {
        wl_proxy_marshal_flags(surface, XDG_SURFACE_ACK_CONFIGURE, null(), 1, 0, serial);
        let state = &mut *(data as *mut State);
        state.configured = true;
        state.dirty = true;
    }
}

#[repr(C)]
pub struct ToplevelListener {
    configure: unsafe extern "C" fn(*mut c_void, Proxy, i32, i32, *mut Array),
    close: unsafe extern "C" fn(*mut c_void, Proxy),
}
pub static TOPLEVEL: ToplevelListener = ToplevelListener {
    configure: toplevel_configure,
    close,
};
unsafe extern "C" fn toplevel_configure(
    data: *mut c_void,
    _: Proxy,
    width: i32,
    height: i32,
    _: *mut Array,
) {
    unsafe {
        let state = &mut *(data as *mut State);
        if width > 0 {
            state.width = width;
        }
        if height > 0 {
            state.height = height;
        }
        state.dirty = true;
    }
}
unsafe extern "C" fn close(data: *mut c_void, _: Proxy) {
    unsafe {
        (*(data as *mut State)).running = false;
    }
}

#[repr(C)]
pub struct FrameListener {
    done: unsafe extern "C" fn(*mut c_void, Proxy, u32),
}
pub static FRAME: FrameListener = FrameListener { done };
unsafe extern "C" fn done(data: *mut c_void, callback: Proxy, _: u32) {
    unsafe {
        destroy(callback);
        (*(data as *mut State)).frame = null_mut();
    }
}

#[repr(C)]
struct SeatListener {
    capabilities: unsafe extern "C" fn(*mut c_void, Proxy, u32),
    name: unsafe extern "C" fn(*mut c_void, Proxy, *const c_char),
}
static SEAT: SeatListener = SeatListener {
    capabilities,
    name: seat_name,
};
unsafe extern "C" fn seat_name(_: *mut c_void, _: Proxy, _: *const c_char) {}
unsafe extern "C" fn capabilities(data: *mut c_void, seat: Proxy, caps: u32) {
    unsafe {
        let state = &mut *(data as *mut State);
        if caps & 2 != 0 && state.keyboard.is_null() {
            state.keyboard = wl_proxy_marshal_flags(
                seat,
                WL_SEAT_GET_KEYBOARD,
                &raw const wl_keyboard_interface,
                5,
                0,
                null::<c_void>(),
            );
            if let Err(error) = listen(state.keyboard, &KEYBOARD, data) {
                state.error = Some(error);
            }
        } else if caps & 2 == 0 && !state.keyboard.is_null() {
            wl_proxy_marshal_flags(
                state.keyboard,
                WL_KEYBOARD_RELEASE,
                null(),
                5,
                WL_MARSHAL_FLAG_DESTROY,
            );
            state.keyboard = null_mut();
            state.repeat = None;
        }
        if caps & 1 != 0 && state.pointer.is_null() {
            state.pointer = wl_proxy_marshal_flags(
                seat,
                WL_SEAT_GET_POINTER,
                &raw const wl_pointer_interface,
                5,
                0,
                null::<c_void>(),
            );
            if let Err(error) = listen(state.pointer, &POINTER, data) {
                state.error = Some(error);
            }
        } else if caps & 1 == 0 && !state.pointer.is_null() {
            wl_proxy_marshal_flags(
                state.pointer,
                WL_POINTER_RELEASE,
                null(),
                5,
                WL_MARSHAL_FLAG_DESTROY,
            );
            state.pointer = null_mut();
        }
    }
}

#[repr(C)]
struct KeyboardListener {
    keymap: unsafe extern "C" fn(*mut c_void, Proxy, u32, i32, u32),
    enter: unsafe extern "C" fn(*mut c_void, Proxy, u32, Proxy, *mut Array),
    leave: unsafe extern "C" fn(*mut c_void, Proxy, u32, Proxy),
    key: unsafe extern "C" fn(*mut c_void, Proxy, u32, u32, u32, u32),
    modifiers: unsafe extern "C" fn(*mut c_void, Proxy, u32, u32, u32, u32, u32),
    repeat: unsafe extern "C" fn(*mut c_void, Proxy, i32, i32),
}
static KEYBOARD: KeyboardListener = KeyboardListener {
    keymap,
    enter: keyboard_enter,
    leave: keyboard_leave,
    key,
    modifiers,
    repeat: repeat_info,
};

unsafe extern "C" fn keymap(data: *mut c_void, _: Proxy, format: u32, fd: i32, size: u32) {
    // SAFETY: Wayland transfiere el fd al cliente. File lo cierra en todos los caminos.
    unsafe {
        let file = File::from_raw_fd(fd);
        let state = &mut *(data as *mut State);
        if format != 1 || size == 0 || size > 4 * 1024 * 1024 {
            state.error = Some("Keymap Wayland inválido o demasiado grande".into());
            return;
        }
        // El descriptor puede compartir posición con el compositor u otros
        // clientes. Leer desde cero con pread no depende de ella ni la modifica.
        let mut bytes = vec![0; size as usize];
        if let Err(error) = file.read_exact_at(&mut bytes, 0) {
            state.error = Some(format!("No se pudo leer el keymap completo: {error}"));
            return;
        }
        // Wayland xkb_v1 entrega una cadena terminada en NUL, incluido en size.
        let Ok(text) = CStr::from_bytes_with_nul(&bytes) else {
            state.error = Some("Keymap Wayland sin terminador NUL válido".into());
            return;
        };
        let map = xkb_keymap_new_from_string(state.xkb_context, text.as_ptr(), 1, 0);
        if map.is_null() {
            state.error = Some("xkbcommon rechazó el keymap".into());
            return;
        }
        let keys = xkb_state_new(map);
        if keys.is_null() {
            xkb_keymap_unref(map);
            state.error = Some("No se pudo crear el estado de teclado".into());
            return;
        }
        if !state.xkb_state.is_null() {
            xkb_state_unref(state.xkb_state);
        }
        if !state.keymap.is_null() {
            xkb_keymap_unref(state.keymap);
        }
        state.keymap = map;
        state.xkb_state = keys;
    }
}
unsafe extern "C" fn keyboard_enter(_: *mut c_void, _: Proxy, _: u32, _: Proxy, _: *mut Array) {}
unsafe extern "C" fn keyboard_leave(data: *mut c_void, _: Proxy, _: u32, _: Proxy) {
    unsafe {
        (*(data as *mut State)).repeat = None;
    }
}
unsafe extern "C" fn modifiers(
    data: *mut c_void,
    _: Proxy,
    _: u32,
    depressed: u32,
    latched: u32,
    locked: u32,
    group: u32,
) {
    unsafe {
        let state = &mut *(data as *mut State);
        if !state.xkb_state.is_null() {
            xkb_state_update_mask(state.xkb_state, depressed, latched, locked, 0, 0, group);
        }
    }
}
unsafe extern "C" fn repeat_info(data: *mut c_void, _: Proxy, rate: i32, delay: i32) {
    unsafe {
        let state = &mut *(data as *mut State);
        state.repeat_rate = rate.clamp(0, 100) as u32;
        state.repeat_delay = Duration::from_millis(delay.clamp(0, 5000) as u64);
        if rate <= 0 {
            state.repeat = None;
        }
    }
}
unsafe extern "C" fn key(data: *mut c_void, _: Proxy, _: u32, _: u32, key: u32, pressed: u32) {
    unsafe {
        let state = &mut *(data as *mut State);
        if pressed == 1 {
            queue_key(state, key);
            if state.repeat_rate > 0
                && !state.keymap.is_null()
                && key
                    .checked_add(8)
                    .is_some_and(|code| xkb_keymap_key_repeats(state.keymap, code) != 0)
            {
                state.repeat = Some((key, Instant::now() + state.repeat_delay));
            }
        } else if state.repeat.is_some_and(|(held, _)| held == key) {
            state.repeat = None;
        }
    }
}

/// Consulta el keymap vigente; repetición y eventos físicos comparten la conversión.
pub fn queue_key(state: &mut State, key: u32) {
    let Some(code) = key.checked_add(8) else {
        return;
    };
    if state.xkb_state.is_null() {
        return;
    }
    if state.keys.len() >= 4096 {
        state.error = Some("Demasiados eventos de teclado pendientes".into());
        return;
    }
    let mut bytes = [0u8; 64];
    // SAFETY: estado XKB propio, buffer con capacidad exacta y keycode comprobado.
    unsafe {
        let count = xkb_state_key_get_utf8(
            state.xkb_state,
            code,
            bytes.as_mut_ptr().cast(),
            bytes.len(),
        );
        let length = if count > 0
            && (count as usize) < bytes.len()
            && std::str::from_utf8(&bytes[..count as usize]).is_ok()
        {
            count as u8
        } else {
            0
        };
        let control =
            xkb_state_mod_name_is_active(state.xkb_state, c"Control".as_ptr(), 1 << 3) > 0;
        state.keys.push_back(KeyEvent {
            sym: xkb_state_key_get_one_sym(state.xkb_state, code),
            text: bytes,
            length,
            control,
        });
    }
}

#[repr(C)]
struct PointerListener {
    enter: unsafe extern "C" fn(*mut c_void, Proxy, u32, Proxy, i32, i32),
    leave: unsafe extern "C" fn(*mut c_void, Proxy, u32, Proxy),
    motion: unsafe extern "C" fn(*mut c_void, Proxy, u32, i32, i32),
    button: unsafe extern "C" fn(*mut c_void, Proxy, u32, u32, u32, u32),
    axis: unsafe extern "C" fn(*mut c_void, Proxy, u32, u32, i32),
    frame: unsafe extern "C" fn(*mut c_void, Proxy),
    source: unsafe extern "C" fn(*mut c_void, Proxy, u32),
    stop: unsafe extern "C" fn(*mut c_void, Proxy, u32, u32),
    discrete: unsafe extern "C" fn(*mut c_void, Proxy, u32, i32),
}
static POINTER: PointerListener = PointerListener {
    enter: pointer_enter,
    leave: pointer_leave,
    motion: pointer_motion,
    button: pointer_button,
    axis,
    frame: pointer_frame,
    source: pointer_source,
    stop: pointer_stop,
    discrete: pointer_discrete,
};
unsafe extern "C" fn pointer_enter(_: *mut c_void, _: Proxy, _: u32, _: Proxy, _: i32, _: i32) {}
unsafe extern "C" fn pointer_leave(_: *mut c_void, _: Proxy, _: u32, _: Proxy) {}
unsafe extern "C" fn pointer_motion(_: *mut c_void, _: Proxy, _: u32, _: i32, _: i32) {}
unsafe extern "C" fn pointer_button(_: *mut c_void, _: Proxy, _: u32, _: u32, _: u32, _: u32) {}
unsafe extern "C" fn pointer_frame(_: *mut c_void, _: Proxy) {}
unsafe extern "C" fn pointer_source(_: *mut c_void, _: Proxy, _: u32) {}
unsafe extern "C" fn pointer_stop(_: *mut c_void, _: Proxy, _: u32, _: u32) {}
unsafe extern "C" fn pointer_discrete(_: *mut c_void, _: Proxy, _: u32, _: i32) {}
unsafe extern "C" fn axis(data: *mut c_void, _: Proxy, _: u32, axis: u32, value: i32) {
    if axis != 0 || value == 0 {
        return;
    }
    unsafe {
        let state = &mut *(data as *mut State);
        state.scroll = if value > 0 {
            state.scroll.saturating_add(3)
        } else {
            state.scroll.saturating_sub(3)
        };
        state.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Seek, SeekFrom, Write},
        os::fd::IntoRawFd,
    };

    const KEYMAP: &[u8] = b"xkb_keymap { xkb_keycodes { minimum = 8; maximum = 255; <ESC> = 9; }; xkb_types { type \"ONE_LEVEL\" { modifiers = None; map[None] = Level1; }; }; xkb_compatibility {}; xkb_symbols { key <ESC> { [ Escape ] }; }; };\0";

    #[test]
    fn keymap_ignores_shared_file_offset() {
        for offset in [KEYMAP.len() as u64, 17, 0] {
            // SAFETY: descriptor anónimo propio; el callback recibe su propiedad.
            let fd = unsafe { memfd_create(c"timer-rust-keymap-test".as_ptr(), 1) };
            assert!(fd >= 0);
            let mut file = unsafe { File::from_raw_fd(fd) };
            file.write_all(KEYMAP).unwrap();
            file.seek(SeekFrom::Start(offset)).unwrap();
            let mut observer = file.try_clone().unwrap();
            let mut state = State::default();
            // SAFETY: contexto propio, State vivo y descriptor transferido una vez.
            unsafe {
                state.xkb_context = xkb_context_new(0);
                assert!(!state.xkb_context.is_null());
                keymap(
                    (&raw mut state).cast(),
                    null_mut(),
                    1,
                    file.into_raw_fd(),
                    KEYMAP.len() as u32,
                );
            }
            assert!(state.error.is_none(), "offset {offset}: {:?}", state.error);
            assert!(!state.xkb_state.is_null());
            assert_eq!(
                unsafe { xkb_state_key_get_one_sym(state.xkb_state, 9) },
                0xff1b
            );
            assert_eq!(observer.stream_position().unwrap(), offset);
        }
    }
}
