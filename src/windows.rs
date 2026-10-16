//! Adaptador Win32: recursos nativos, presentación, audio y bucle de mensajes.
//!
//! Toda llamada FFI queda en este adaptador. Se ejecuta en el hilo que crea la
//! ventana. Los callbacks no capturan ni acceden al estado de `TimerApp`, para
//! evitar préstamos reentrantes durante `SendMessageW` o `DestroyWindow`.

mod native;

use crate::app::{Outcome, TimerApp};
use crate::command::Command;
use native::*;
use std::ptr::{null, null_mut};
use std::time::Instant;

const LIST: i32 = 101;
const STATUS: i32 = 102;
const INPUT: i32 = 103;
const WAKE_TIMER: usize = 1;
const INPUT_LIMIT: usize = 1024;
const FONT_HEIGHT: i32 = -16;

/// UTF-16 con terminador. Los buffers permanecen vivos durante llamadas síncronas.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// Captura el último error inmediatamente después de una API que lo documenta.
fn win_error(operation: &str) -> String {
    format!("{operation}: {}", std::io::Error::last_os_error())
}

/// Diagnóstico fatal, necesario porque el ejecutable GUI no tiene consola.
pub fn show_error(error: &str) {
    // SAFETY: cadenas terminadas y vivas durante la llamada; propietario nulo permitido.
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(error).as_ptr(),
            wide("Timer-Rust").as_ptr(),
            MB_ICONERROR,
        );
    }
}

/// Reposiciona controles ya creados; no modifica la fuente al redimensionar.
///
/// # Safety
/// `window` pertenece al hilo actual. Durante creación puede no haber hijos todavía.
unsafe fn layout(window: Handle) {
    let mut rect = Rect::default();
    // SAFETY: Win32 suministra el HWND; rect es un buffer C válido de salida.
    unsafe {
        if GetClientRect(window, &mut rect) == 0 {
            return;
        }
        let width = (rect.right - 16).max(1);
        for (id, y, height) in [
            (LIST, 8, (rect.bottom - 88).max(1)),
            (STATUS, (rect.bottom - 72).max(8), 32),
            (INPUT, (rect.bottom - 34).max(8), 26),
        ] {
            let control = GetDlgItem(window, id);
            if !control.is_null() {
                MoveWindow(control, 8, y, width, height, 1);
            }
        }
    }
}

/// Callback sin estado Rust compartido ni operaciones que puedan hacer panic.
///
/// # Safety
/// Solo Windows lo invoca con la ABI y los parámetros de WNDPROC.
unsafe extern "system" fn window_proc(
    window: Handle,
    kind: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    // SAFETY: handles y mensajes provienen de Windows, siempre en el hilo GUI.
    unsafe {
        match kind {
            WM_SIZE => {
                layout(window);
                0
            }
            WM_SETFOCUS => {
                SetFocus(GetDlgItem(window, INPUT));
                0
            }
            // El cierre solicita salir. Ui::drop destruye la ventana una sola vez.
            WM_CLOSE => {
                PostQuitMessage(0);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, kind, wparam, lparam),
        }
    }
}

/// Propietario RAII de ventana y fuente; los hijos pertenecen a la ventana.
///
/// Los handles crudos impiden trasladarlo de hilo. Cursor y brocha son recursos
/// compartidos del sistema y no deben destruirse. El caché visual pertenece a la
/// vista; el dominio desconoce controles y estado de redibujado.
struct Ui {
    window: Handle,
    list: Handle,
    status: Handle,
    input: Handle,
    font: Handle,
    displayed: Vec<String>,
}

impl Drop for Ui {
    fn drop(&mut self) {
        // SAFETY: recursos creados en este hilo. Se destruyen controles antes de
        // liberar la fuente seleccionada en ellos. Un fallo de limpieza no hace panic.
        unsafe {
            if IsWindow(self.window) != 0 {
                DestroyWindow(self.window);
            }
            if !self.font.is_null() {
                DeleteObject(self.font);
            }
        }
    }
}

impl Ui {
    /// Inicializa recursos; después de crear la ventana, cualquier error usa Drop.
    fn create() -> Result<Self, String> {
        // SAFETY: todas las cadenas tienen terminador y duran toda la llamada.
        // Windows copia los nombres y conserva solo el puntero estático WNDPROC.
        // Los HWND y la fuente son usados y destruidos únicamente en este hilo.
        unsafe {
            let instance = GetModuleHandleW(null());
            if instance.is_null() {
                return Err(win_error("GetModuleHandleW"));
            }
            let class_name = wide("TimerRustNative");
            let class = WindowClass {
                style: 0,
                procedure: Some(window_proc),
                class_extra: 0,
                window_extra: 0,
                instance,
                icon: null_mut(),
                cursor: LoadCursorW(null_mut(), IDC_ARROW as *const u16),
                background: COLOR_WINDOW_BRUSH as Handle,
                menu_name: null(),
                class_name: class_name.as_ptr(),
            };
            if class.cursor.is_null() {
                return Err(win_error("LoadCursorW"));
            }
            // La clase tiene vida de proceso: run se llama una sola vez desde main.
            if RegisterClassW(&class) == 0 {
                return Err(win_error("RegisterClassW"));
            }
            let window = CreateWindowExW(
                0,
                class_name.as_ptr(),
                wide("Timer-Rust").as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
                i32::MIN,
                i32::MIN,
                660,
                420,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            );
            if window.is_null() {
                return Err(win_error("CreateWindowExW"));
            }
            let mut ui = Self {
                window,
                list: null_mut(),
                status: null_mut(),
                input: null_mut(),
                font: null_mut(),
                displayed: Vec::new(),
            };
            // Altura lógica fija; el SO puede sustituir Consolas si no está instalada.
            ui.font = CreateFontW(
                FONT_HEIGHT,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                1,
                0,
                0,
                0,
                0,
                wide("Consolas").as_ptr(),
            );
            if ui.font.is_null() {
                return Err(win_error("CreateFontW"));
            }
            let child = |class: &str, id: i32, style: u32, ex_style: u32| {
                CreateWindowExW(
                    ex_style,
                    wide(class).as_ptr(),
                    wide("").as_ptr(),
                    WS_CHILD | WS_VISIBLE | style,
                    0,
                    0,
                    1,
                    1,
                    window,
                    id as Handle,
                    instance,
                    null_mut(),
                )
            };
            ui.list = child("LISTBOX", LIST, WS_VSCROLL | LBS_NOINTEGRALHEIGHT, 0);
            ui.status = child("STATIC", STATUS, SS_LEFTNOWORDWRAP, 0);
            ui.input = child("EDIT", INPUT, WS_TABSTOP | ES_AUTOHSCROLL, WS_EX_CLIENTEDGE);
            if ui.list.is_null() || ui.status.is_null() || ui.input.is_null() {
                return Err(win_error("Crear controles"));
            }
            for control in [ui.list, ui.status, ui.input] {
                SendMessageW(control, WM_SETFONT, ui.font as usize, 0);
            }
            SendMessageW(ui.input, EM_SETLIMITTEXT, INPUT_LIMIT, 0);
            ui.set_status("Prueba Windows 11 · escribe ayuda y pulsa Enter");
            layout(window);
            ShowWindow(window, SW_SHOW);
            SetFocus(ui.input);
            Ok(ui)
        }
    }

    /// Actualiza el mensaje de estado; el control copia la cadena síncronamente.
    fn set_status(&self, status: &str) {
        // SAFETY: HWND propio y cadena UTF-16 viva durante SetWindowTextW.
        unsafe {
            SetWindowTextW(self.status, wide(status).as_ptr());
        }
    }

    /// Se borra solo tras un comando válido; los errores conservan la entrada.
    fn clear_input(&self) {
        // SAFETY: HWND propio y cadena terminada viva durante la llamada.
        unsafe {
            SetWindowTextW(self.input, wide("").as_ptr());
        }
    }

    /// Lee como máximo INPUT_LIMIT unidades UTF-16, más espacio para el terminador.
    fn command(&self) -> String {
        let mut text = [0u16; INPUT_LIMIT + 1];
        // SAFETY: buffer escribible cuyo tamaño exacto se pasa a Windows.
        let count = unsafe { GetWindowTextW(self.input, text.as_mut_ptr(), text.len() as i32) };
        String::from_utf16_lossy(&text[..count.max(0) as usize])
    }

    /// Consulta necesaria para omitir repintado y esperar hasta el vencimiento.
    fn minimized(&self) -> bool {
        // SAFETY: consulta de un HWND propio, en su hilo.
        unsafe { IsIconic(self.window) != 0 }
    }

    /// Solo reconstruye la lista si cambió; conserva selección y desplazamiento.
    fn rows(&mut self, rows: Vec<String>) -> Result<(), String> {
        if rows == self.displayed {
            return Ok(());
        }
        // SAFETY: mensajes del control LISTBOX propio; LB_ADDSTRING copia cada
        // cadena antes de volver. Ningún puntero temporal queda almacenado.
        unsafe {
            let top = SendMessageW(self.list, LB_GETTOPINDEX, 0, 0);
            let selected = SendMessageW(self.list, LB_GETCURSEL, 0, 0);
            SendMessageW(self.list, WM_SETREDRAW, 0, 0);
            SendMessageW(self.list, LB_RESETCONTENT, 0, 0);
            let mut failed = false;
            for row in &rows {
                let result = SendMessageW(self.list, LB_ADDSTRING, 0, wide(row).as_ptr() as isize);
                if result == LB_ERR || result == LB_ERRSPACE {
                    failed = true;
                    break;
                }
            }
            if selected >= 0 {
                SendMessageW(self.list, LB_SETCURSEL, selected as usize, 0);
            }
            if top >= 0 {
                SendMessageW(self.list, LB_SETTOPINDEX, top as usize, 0);
            }
            // También se restablece el redibujado si falla la reserva del control.
            SendMessageW(self.list, WM_SETREDRAW, 1, 0);
            InvalidateRect(self.list, null(), 1);
            if failed {
                return Err("Windows no pudo actualizar la lista de temporizadores".into());
            }
        }
        self.displayed = rows;
        Ok(())
    }

    /// Aplica los límites de SetTimer. Sin plazo, KillTimer deja la GUI en espera.
    fn schedule(&self, app: &TimerApp) -> Result<(), String> {
        // SAFETY: ID local a esta ventana. Callback nulo envía WM_TIMER al hilo GUI.
        unsafe {
            KillTimer(self.window, WAKE_TIMER);
        }
        if let Some(delay) = app.next_wake(Instant::now(), self.minimized()) {
            let ms = delay
                .as_millis()
                .saturating_add(u128::from(delay.subsec_nanos() % 1_000_000 != 0));
            // SAFETY: HWND propio, intervalo dentro de USER_TIMER_MINIMUM/MAXIMUM.
            if unsafe {
                SetTimer(
                    self.window,
                    WAKE_TIMER,
                    ms.clamp(10, 0x7FFF_FFFF) as u32,
                    null(),
                )
            } == 0
            {
                return Err(win_error("SetTimer"));
            }
        }
        Ok(())
    }
}

/// Audio provisional del sistema. SND_SYNC bloquea la GUI hasta terminar.
/// No hay reintentos ni sonidos alternativos ocultos si falla el alias configurado.
fn sound() -> Result<(), String> {
    // SAFETY: alias UTF-16 válido y vivo hasta que vuelve la reproducción síncrona.
    if unsafe {
        PlaySoundW(
            wide("SystemExclamation").as_ptr(),
            null_mut(),
            SND_ALIAS | SND_NODEFAULT | SND_SYNC,
        )
    } == 0
    {
        Err("Windows no pudo reproducir SystemExclamation; revisa el esquema de sonido".into())
    } else {
        Ok(())
    }
}

/// Único lugar que convierte resultados del dominio en presentación y audio.
fn present(ui: &Ui, outcome: Outcome) {
    ui.set_status(&outcome.status);
    if outcome.sound
        && let Err(error) = sound()
    {
        ui.set_status(&error);
    }
}

/// Actualiza vencimientos antes de dibujar. Tras el audio se relee el reloj, pues
/// el reproductor síncrono pudo consumir tiempo; no se usa un instante por fila.
fn refresh(app: &mut TimerApp, ui: &mut Ui) -> Result<(), String> {
    if let Some(outcome) = app.advance(Instant::now()) {
        present(ui, outcome);
    }
    if !ui.minimized() {
        ui.rows(app.rows(Instant::now()))?;
    }
    Ok(())
}

/// Ejecuta una sola ventana en el hilo llamador. No crea hilos, tareas ni runtimes.
/// GetMessageW bloquea el hilo cuando no hay mensajes; todos los errores se
/// propagan a main y Ui libera los recursos también en las salidas tempranas.
pub fn run() -> Result<(), String> {
    let mut ui = Ui::create()?;
    let mut app = TimerApp::default();
    let mut message = Message::default();
    loop {
        // SAFETY: buffer MSG inicializado y válido; sin filtro se recibe WM_QUIT.
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result == -1 {
            return Err(win_error("GetMessageW"));
        }
        if result == 0 {
            return Ok(());
        }
        if message.window == ui.input && message.kind == WM_KEYDOWN && message.wparam == VK_RETURN {
            let input = ui.command();
            refresh(&mut app, &mut ui)?;
            let result =
                Command::parse(&input).and_then(|command| app.execute(command, Instant::now()));
            match result {
                Ok(outcome) => {
                    present(&ui, outcome);
                    ui.clear_input();
                }
                Err(error) => ui.set_status(&error),
            }
            refresh(&mut app, &mut ui)?;
            ui.schedule(&app)?;
            // No se traduce Return: evita insertar un carácter o generar un beep.
            continue;
        }
        if message.window == ui.window && message.kind == WM_TIMER && message.wparam == WAKE_TIMER {
            refresh(&mut app, &mut ui)?;
            ui.schedule(&app)?;
            continue;
        }
        // SAFETY: mensaje devuelto por GetMessageW; callback sin estado prestado.
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if message.window == ui.window && message.kind == WM_SIZE {
            refresh(&mut app, &mut ui)?;
            ui.schedule(&app)?;
        }
    }
}
