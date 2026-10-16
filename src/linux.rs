//! GUI Wayland de un solo hilo: protocolo, entrada, planificación y presentación.
//! Solo se despachan callbacks desde el bucle principal; los recursos y sus datos
//! permanecen en Box estables hasta eliminar listeners y desconectar el display.

mod audio;
mod entry;
mod events;
mod font;
mod library;
mod native;
mod render;

use crate::{
    app::{Outcome, RowCache, TimerApp},
    command::Command,
};
use entry::Entry;
use font::Font;
use native::*;
use render::{Buffer, View};
use std::{
    collections::VecDeque,
    ffi::c_void,
    ptr::{null, null_mut},
    time::{Duration, Instant},
};

struct KeyEvent {
    sym: u32,
    text: [u8; 64],
    length: u8,
    control: bool,
}

/// Estado no compartido: nunca se mueve el contenido del Box después de registrar
/// su dirección en los listeners. No se prestan referencias a través de dispatch.
struct State {
    display: Proxy,
    registry: Proxy,
    compositor: Proxy,
    shm: Proxy,
    wm: Proxy,
    seat: Proxy,
    keyboard: Proxy,
    pointer: Proxy,
    surface: Proxy,
    xdg: Proxy,
    toplevel: Proxy,
    frame: Proxy,
    xkb_context: Proxy,
    keymap: Proxy,
    xkb_state: Proxy,
    globals: [u32; 4],
    argb: bool,
    rgb565: bool,
    configured: bool,
    running: bool,
    dirty: bool,
    width: i32,
    height: i32,
    error: Option<String>,
    app: TimerApp,
    entry: Entry,
    status: String,
    rows: RowCache,
    rows_dirty: bool,
    rows_due: Option<Instant>,
    rows_version: u64,
    footer_version: u64,
    font: Option<Font>,
    scroll: usize,
    #[expect(
        clippy::vec_box,
        reason = "Cada listener wl_buffer.release conserva la dirección estable de su Buffer"
    )]
    buffers: Vec<Box<Buffer>>,
    keys: VecDeque<KeyEvent>,
    repeat_rate: u32,
    repeat_delay: Duration,
    repeat: Option<(u32, Instant)>,
    sound_pending: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            display: null_mut(),
            registry: null_mut(),
            compositor: null_mut(),
            shm: null_mut(),
            wm: null_mut(),
            seat: null_mut(),
            keyboard: null_mut(),
            pointer: null_mut(),
            surface: null_mut(),
            xdg: null_mut(),
            toplevel: null_mut(),
            frame: null_mut(),
            xkb_context: null_mut(),
            keymap: null_mut(),
            xkb_state: null_mut(),
            globals: [0; 4],
            argb: false,
            rgb565: false,
            configured: false,
            running: true,
            dirty: true,
            width: 660,
            height: 420,
            error: None,
            app: TimerApp::default(),
            entry: Entry::default(),
            status: "Prueba Wayland · escribe ayuda y pulsa Enter".into(),
            rows: RowCache::default(),
            rows_dirty: true,
            rows_due: None,
            rows_version: 0,
            footer_version: 0,
            font: None,
            scroll: 0,
            buffers: Vec::new(),
            keys: VecDeque::new(),
            repeat_rate: 0,
            repeat_delay: Duration::from_millis(500),
            repeat: None,
            sound_pending: false,
        }
    }
}

impl State {
    fn take_error(&mut self) -> Result<(), String> {
        self.error.take().map_or(Ok(()), Err)
    }

    fn present(&mut self, outcome: Outcome) {
        self.status = outcome.status;
        self.footer_version = self.footer_version.wrapping_add(1);
        self.sound_pending |= outcome.sound;
        self.dirty = true;
    }

    /// Eventos de entrada se procesan fuera del callback, incluido el audio.
    fn update(&mut self) -> Result<(), String> {
        self.take_error()?;
        let now = Instant::now();
        if let Some(outcome) = self.app.advance(now) {
            self.rows_dirty = true;
            self.present(outcome);
        }
        if let Some((key, deadline)) = self.repeat
            && deadline <= now
            && self.repeat_rate > 0
        {
            events::queue_key(self, key);
            self.repeat = Some((
                key,
                now + Duration::from_nanos(1_000_000_000 / self.repeat_rate as u64),
            ));
        }
        while let Some(key) = self.keys.pop_front() {
            if key.control {
                match key.sym {
                    0x61 | 0x41 => {
                        self.entry.selected = true;
                        self.entry.cursor = self.entry.text.len();
                    }
                    0x75 | 0x55 => self.entry.clear(),
                    _ => continue,
                }
            } else {
                match key.sym {
                    KEY_RETURN | KEY_KP_ENTER => {
                        let parsed = Command::parse(&self.entry.text);
                        let changes_rows = matches!(
                            &parsed,
                            Ok(Command::Create { .. }
                                | Command::Configure { .. }
                                | Command::Pause(_)
                                | Command::Resume(_)
                                | Command::Stop(_))
                        );
                        match parsed.and_then(|command| self.app.execute(command, Instant::now())) {
                            Ok(outcome) => {
                                self.entry.clear();
                                self.rows_dirty |= changes_rows;
                                self.present(outcome);
                            }
                            Err(error) => {
                                self.status = error;
                                self.entry.cursor = self.entry.text.len();
                                self.entry.selected = false;
                            }
                        }
                    }
                    KEY_BACKSPACE => self.entry.backspace(),
                    KEY_DELETE => self.entry.delete(),
                    KEY_LEFT => self.entry.left(),
                    KEY_RIGHT => self.entry.right(),
                    KEY_HOME => {
                        self.entry.cursor = 0;
                        self.entry.selected = false;
                    }
                    KEY_END => {
                        self.entry.cursor = self.entry.text.len();
                        self.entry.selected = false;
                    }
                    KEY_PAGE_UP => self.scroll = self.scroll.saturating_sub(5),
                    KEY_PAGE_DOWN => self.scroll = self.scroll.saturating_add(5),
                    _ => self.entry.insert(
                        std::str::from_utf8(&key.text[..key.length as usize])
                            .expect("UTF-8 validado al encolar"),
                    ),
                }
            }
            self.dirty = true;
            self.footer_version = self.footer_version.wrapping_add(1);
        }
        let now = Instant::now();
        if self.rows_dirty || self.rows_due.is_some_and(|due| due <= now) {
            if self.app.update_rows(&mut self.rows, now) {
                self.rows_version = self.rows_version.wrapping_add(1);
                self.dirty = true;
            }
            self.rows_due = self
                .app
                .next_wake(now, false)
                .and_then(|delay| now.checked_add(delay));
            self.rows_dirty = false;
        }
        self.scroll = self
            .scroll
            .min(self.rows.lines.len().saturating_sub(self.visible_rows()));
        Ok(())
    }

    fn visible_rows(&self) -> usize {
        ((self.height - 96).max(24) / 24) as usize
    }

    /// Reutiliza un buffer libre. Nunca modifica memoria aún prestada al servidor.
    fn render(&mut self, data: *mut c_void) -> Result<(), String> {
        // Un resize grande seguido de uno pequeño no conserva imágenes grandes
        // libres. Las ocupadas se retiran en cuanto llega wl_buffer.release.
        self.buffers.retain(|buffer| {
            buffer.busy.get() || (buffer.width == self.width && buffer.height == self.height)
        });
        if !self.configured || !self.dirty || !self.frame.is_null() {
            return Ok(());
        }
        let index = if let Some(index) = self.buffers.iter().position(|buffer| !buffer.busy.get()) {
            index
        } else if self.buffers.len() < 2 {
            self.buffers
                .push(Buffer::new(self.shm, self.width, self.height, self.rgb565)?);
            self.buffers.len() - 1
        } else {
            return Ok(());
        };
        if self.buffers[index].width != self.width || self.buffers[index].height != self.height {
            self.buffers[index] = Buffer::new(self.shm, self.width, self.height, self.rgb565)?;
        }
        let view = View {
            rows: &self.rows.lines,
            status: &self.status,
            entry: &self.entry,
            scroll: self.scroll,
            rows_version: self.rows_version,
            footer_version: self.footer_version,
        };
        let buffer = &mut self.buffers[index];
        let damage = buffer.draw(
            &view,
            self.font
                .as_mut()
                .expect("Fuente inicializada antes de la ventana"),
        )?;
        // SAFETY: opcodes de wl_surface v3; data estable; marshalling no despacha
        // callbacks. Los píxeles quedan ocupados hasta wl_buffer.release.
        unsafe {
            self.frame = wl_proxy_marshal_flags(
                self.surface,
                WL_SURFACE_FRAME,
                &raw const wl_callback_interface,
                1,
                0,
                null::<c_void>(),
            );
            events::listen(self.frame, &events::FRAME, data)?;
            wl_proxy_marshal_flags(
                self.surface,
                WL_SURFACE_ATTACH,
                null(),
                3,
                0,
                buffer.proxy,
                0i32,
                0i32,
            );
            if damage.list {
                wl_proxy_marshal_flags(
                    self.surface,
                    WL_SURFACE_DAMAGE,
                    null(),
                    3,
                    0,
                    0i32,
                    0i32,
                    self.width,
                    self.height - 80,
                );
            }
            if damage.footer {
                wl_proxy_marshal_flags(
                    self.surface,
                    WL_SURFACE_DAMAGE,
                    null(),
                    3,
                    0,
                    0i32,
                    self.height - 80,
                    self.width,
                    80i32,
                );
            }
            buffer.busy.set(true);
            wl_proxy_marshal_flags(self.surface, WL_SURFACE_COMMIT, null(), 3, 0);
        }
        self.dirty = false;
        Ok(())
    }

    fn timeout(&self) -> i32 {
        let now = Instant::now();
        let timer = self.app.next_wake(now, !self.frame.is_null());
        let repeat = self
            .repeat
            .map(|(_, deadline)| deadline.saturating_duration_since(now));
        match timer.into_iter().chain(repeat).min() {
            None => -1,
            Some(delay) => delay
                .as_millis()
                .saturating_add(u128::from(delay.subsec_nanos() % 1_000_000 != 0))
                .min(i32::MAX as u128) as i32,
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        // SAFETY: ya no se despachan callbacks. Buffers eliminan sus listeners y
        // mapeos antes de desconectar; todos los proxies son propios y distintos.
        self.buffers.clear();
        unsafe {
            for proxy in [
                self.frame,
                self.toplevel,
                self.xdg,
                self.surface,
                self.keyboard,
                self.pointer,
                self.seat,
                self.wm,
                self.shm,
                self.compositor,
                self.registry,
            ] {
                destroy(proxy);
            }
            if !self.xkb_state.is_null() {
                xkb_state_unref(self.xkb_state);
            }
            if !self.keymap.is_null() {
                xkb_keymap_unref(self.keymap);
            }
            if !self.xkb_context.is_null() {
                xkb_context_unref(self.xkb_context);
            }
            if !self.display.is_null() {
                wl_display_disconnect(self.display);
            }
        }
    }
}

/// Construye la conexión y sus objetos. Los roundtrips se invocan sin mantener
/// un préstamo del State que volverán a pedir los callbacks.
fn connect() -> Result<Box<State>, String> {
    let mut state = Box::<State>::default();
    let data = &mut *state as *mut State as *mut c_void;
    // SAFETY: creación FFI con cadenas estáticas terminadas y Box estable.
    unsafe {
        state.xkb_context = xkb_context_new(0);
        if state.xkb_context.is_null() {
            return Err("No se pudo crear el contexto xkbcommon".into());
        }
        state.display = wl_display_connect(null());
        if state.display.is_null() {
            return Err(
                "No se pudo conectar a Wayland; revisar WAYLAND_DISPLAY y XDG_RUNTIME_DIR".into(),
            );
        }
        state.registry = wl_proxy_marshal_flags(
            state.display,
            WL_DISPLAY_GET_REGISTRY,
            &raw const wl_registry_interface,
            1,
            0,
            null::<c_void>(),
        );
        events::listen(state.registry, &events::REGISTRY, data)?;
    }
    let display = state.display;
    state.font = Some(Font::new()?);
    // SAFETY: sin referencias al contenido de State a través de dispatch.
    if unsafe { wl_display_roundtrip(display) } < 0 || unsafe { wl_display_roundtrip(display) } < 0
    {
        return Err("Falló la negociación inicial de Wayland".into());
    }
    state.take_error()?;
    if state.compositor.is_null()
        || state.shm.is_null()
        || state.wm.is_null()
        || state.seat.is_null()
        || !state.argb
    {
        return Err(
            "Se requieren wl_compositor v3, wl_shm ARGB8888, xdg_wm_base y wl_seat v5".into(),
        );
    }
    // SAFETY: constructores e interfaces negociadas; listeners permanecen estáticos.
    unsafe {
        state.surface = wl_proxy_marshal_flags(
            state.compositor,
            WL_COMPOSITOR_CREATE_SURFACE,
            &raw const wl_surface_interface,
            3,
            0,
            null::<c_void>(),
        );
        if state.surface.is_null() {
            return Err("No se pudo crear wl_surface".into());
        }
        state.xdg = wl_proxy_marshal_flags(
            state.wm,
            XDG_WM_BASE_GET_XDG_SURFACE,
            &raw const xdg_surface_interface,
            1,
            0,
            null::<c_void>(),
            state.surface,
        );
        events::listen(state.xdg, &events::SURFACE, data)?;
        state.toplevel = wl_proxy_marshal_flags(
            state.xdg,
            XDG_SURFACE_GET_TOPLEVEL,
            &raw const xdg_toplevel_interface,
            1,
            0,
            null::<c_void>(),
        );
        events::listen(state.toplevel, &events::TOPLEVEL, data)?;
        wl_proxy_marshal_flags(
            state.toplevel,
            XDG_TOPLEVEL_SET_TITLE,
            null(),
            1,
            0,
            c"Timer-Rust".as_ptr(),
        );
        wl_proxy_marshal_flags(
            state.toplevel,
            XDG_TOPLEVEL_SET_APP_ID,
            null(),
            1,
            0,
            c"timer-rust".as_ptr(),
        );
        wl_proxy_marshal_flags(
            state.toplevel,
            XDG_TOPLEVEL_SET_MIN_SIZE,
            null(),
            1,
            0,
            240i32,
            150i32,
        );
        wl_proxy_marshal_flags(state.surface, WL_SURFACE_COMMIT, null(), 3, 0);
    }
    Ok(state)
}

/// Entrada privada del ayudante de audio: no inicializa fuente ni Wayland.
pub fn sound_helper() -> Result<(), String> {
    audio::play_local()
}

/// Bucle monohilo y sin sondeo activo. Solo este hilo lee el display; no hay EGL,
/// colas auxiliares ni librerías que lean su socket por detrás del poll.
pub fn run() -> Result<(), String> {
    let mut state = connect()?;
    let data = &mut *state as *mut State as *mut c_void;
    let display = state.display;
    let fd = unsafe { wl_display_get_fd(display) };
    loop {
        // SAFETY: el contenido de State no está prestado durante callbacks.
        if unsafe { wl_display_dispatch_pending(display) } < 0 {
            return Err("Wayland se desconectó".into());
        }
        if !state.running {
            return Ok(());
        }
        state.update()?;
        state.render(data)?;
        let flushed = unsafe { wl_display_flush(display) };
        let blocked =
            flushed < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock;
        if flushed < 0 && !blocked {
            return Err("No se pudieron enviar mensajes Wayland".into());
        }
        if state.sound_pending {
            state.sound_pending = false;
            if let Err(error) = audio::play() {
                state.status = error;
                state.footer_version = state.footer_version.wrapping_add(1);
                state.dirty = true;
            }
            // Se relee el reloj tras audio síncrono, nunca se alargan los plazos.
            continue;
        }
        let mut descriptor = PollFd {
            fd,
            events: POLLIN | if blocked { POLLOUT } else { 0 },
            revents: 0,
        };
        // SAFETY: descriptor de conexión vivo y un único lector. Timeout -1 espera
        // indefinidamente cuando no hay cuentas activas ni tecla en repetición.
        let result = unsafe { poll(&mut descriptor, 1, state.timeout()) };
        if result < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.to_string());
        }
        if descriptor.revents & (POLLERR | POLLHUP) != 0 {
            return Err("El compositor cerró la conexión".into());
        }
        if descriptor.revents & POLLIN != 0 && unsafe { wl_display_dispatch(display) } < 0 {
            return Err("Error al recibir eventos Wayland".into());
        }
    }
}
