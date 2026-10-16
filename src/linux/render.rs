//! Dibujo Cairo sobre wl_shm. Dos buffers como máximo, reutilizados tras release.
//! No hay GPU, hilos de render ni asignación de una imagen nueva por cada tick.

use super::{entry::Entry, native::*};
use std::{
    cell::Cell,
    ffi::{CString, c_void},
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    ptr::{null, null_mut},
};

/// Todos los campos son recursos del mismo hilo GUI. El Box fija la dirección del
/// dato entregado al listener release mientras el vector de buffers cambia.
pub struct Buffer {
    pub proxy: Proxy,
    pub width: i32,
    pub height: i32,
    pub busy: Cell<bool>,
    data: *mut c_void,
    bytes: usize,
    image: Proxy,
}

unsafe extern "C" fn released(data: *mut c_void, _buffer: Proxy) {
    // SAFETY: el listener solo vive dentro del Box y se elimina antes de liberarlo.
    unsafe {
        (&*(data as *const Buffer)).busy.set(false);
    }
}

#[repr(C)]
struct BufferListener {
    release: unsafe extern "C" fn(*mut c_void, Proxy),
}
static BUFFER_LISTENER: BufferListener = BufferListener { release: released };

impl Buffer {
    pub fn new(shm: Proxy, width: i32, height: i32) -> Result<Box<Self>, String> {
        let stride = width
            .checked_mul(4)
            .filter(|_| width > 0)
            .ok_or("Ancho inválido")?;
        let bytes = (stride as usize)
            .checked_mul(height.max(0) as usize)
            .filter(|&n| n > 0 && n <= 32 * 1024 * 1024)
            .ok_or("Ventana demasiado grande (máximo 32 MiB por buffer)")?;
        // SAFETY: memfd nuevo y privado; File posee su descriptor desde este punto.
        let fd = unsafe { memfd_create(c"timer-rust".as_ptr(), 1) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let file = unsafe { File::from_raw_fd(fd) };
        file.set_len(bytes as u64).map_err(|e| e.to_string())?;
        // SAFETY: mapeo compartido del tamaño ya reservado, lectura/escritura, offset 0.
        let data = unsafe { mmap(null_mut(), bytes, 3, 1, file.as_raw_fd(), 0) };
        if data as isize == -1 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut buffer = Box::new(Self {
            proxy: null_mut(),
            width,
            height,
            busy: Cell::new(false),
            data,
            bytes,
            image: null_mut(),
        });
        // SAFETY: firmas/opcodes de wl_shm v1. El servidor recibe su propio fd;
        // el pool puede destruirse tras crear el buffer y File puede cerrarse.
        unsafe {
            let pool = wl_proxy_marshal_flags(
                shm,
                WL_SHM_CREATE_POOL,
                &raw const wl_shm_pool_interface,
                1,
                0,
                null::<c_void>(),
                file.as_raw_fd(),
                bytes as i32,
            );
            if pool.is_null() {
                return Err("No se pudo crear wl_shm_pool".into());
            }
            buffer.proxy = wl_proxy_marshal_flags(
                pool,
                WL_SHM_POOL_CREATE_BUFFER,
                &raw const wl_buffer_interface,
                1,
                0,
                null::<c_void>(),
                0i32,
                width,
                height,
                stride,
                0u32,
            );
            wl_proxy_marshal_flags(
                pool,
                WL_SHM_POOL_DESTROY,
                null(),
                1,
                WL_MARSHAL_FLAG_DESTROY,
            );
            if buffer.proxy.is_null() {
                return Err("No se pudo crear wl_buffer".into());
            }
            if wl_proxy_add_listener(
                buffer.proxy,
                &BUFFER_LISTENER as *const _ as *const c_void,
                &mut *buffer as *mut Buffer as *mut c_void,
            ) != 0
            {
                return Err("No se pudo registrar wl_buffer.release".into());
            }
            buffer.image =
                cairo_image_surface_create_for_data(data.cast(), 0, width, height, stride);
            if cairo_surface_status(buffer.image) != 0 {
                return Err("Cairo no pudo crear la imagen".into());
            }
        }
        Ok(buffer)
    }

    pub fn draw(
        &self,
        rows: &[String],
        status: &str,
        entry: &Entry,
        scroll: usize,
    ) -> Result<(), String> {
        let canvas = Canvas::new(self.image)?;
        let width = self.width as f64;
        let height = self.height as f64;
        canvas.color(1.0, 1.0, 1.0);
        // SAFETY: contexto válido de la imagen propia, usado solo en este hilo.
        unsafe {
            cairo_paint(canvas.0);
        }
        canvas.clip(8.0, 6.0, width - 16.0, (height - 94.0).max(0.0));
        for (index, row) in rows.iter().skip(scroll).enumerate() {
            let y = 22.0 + index as f64 * 24.0;
            if y > height - 88.0 {
                break;
            }
            canvas.color(0.05, 0.05, 0.05);
            canvas.text(8.0, y, row)?;
        }
        canvas.unclip();
        if rows.len() > self.visible_rows() {
            let track = (height - 96.0).max(1.0);
            let thumb = (track * self.visible_rows() as f64 / rows.len() as f64).max(8.0);
            let max_scroll = rows.len().saturating_sub(self.visible_rows()).max(1);
            canvas.color(0.6, 0.6, 0.6);
            canvas.rect(
                width - 5.0,
                8.0 + (track - thumb) * scroll as f64 / max_scroll as f64,
                3.0,
                thumb,
            );
        }
        canvas.color(0.94, 0.94, 0.94);
        canvas.rect(8.0, height - 72.0, width - 16.0, 32.0);
        canvas.clip(8.0, height - 72.0, width - 16.0, 32.0);
        canvas.color(0.1, 0.1, 0.1);
        for (index, line) in status.lines().take(2).enumerate() {
            canvas.text(10.0, height - 59.0 + index as f64 * 16.0, line)?;
        }
        canvas.unclip();
        canvas.color(0.5, 0.5, 0.5);
        canvas.rect(8.0, height - 34.0, width - 16.0, 26.0);
        canvas.color(1.0, 1.0, 1.0);
        canvas.rect(9.0, height - 33.0, width - 18.0, 24.0);
        canvas.clip(11.0, height - 32.0, width - 22.0, 23.0);
        let caret = canvas.measure(&entry.text[..entry.cursor])?;
        let offset = (caret - (width - 30.0)).max(0.0);
        if entry.selected && !entry.text.is_empty() {
            canvas.color(0.0, 0.35, 0.7);
            canvas.rect(
                12.0 - offset,
                height - 31.0,
                canvas.measure(&entry.text)?,
                21.0,
            );
            canvas.color(1.0, 1.0, 1.0);
        } else {
            canvas.color(0.05, 0.05, 0.05);
        }
        canvas.text(12.0 - offset, height - 15.0, &entry.text)?;
        canvas.color(0.05, 0.05, 0.05);
        canvas.rect(12.0 + caret - offset, height - 30.0, 1.0, 19.0);
        canvas.unclip();
        // SAFETY: flush entrega a wl_shm todos los píxeles terminados por Cairo.
        unsafe {
            cairo_surface_flush(self.image);
        }
        if unsafe { cairo_status(canvas.0) } != 0 {
            return Err("Falló el dibujo Cairo".into());
        }
        Ok(())
    }

    pub fn visible_rows(&self) -> usize {
        ((self.height - 96).max(24) / 24) as usize
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: se elimina la imagen antes de su memoria, y el proxy antes de
        // liberar el dato del listener. El mapeo del compositor es independiente.
        unsafe {
            if !self.image.is_null() {
                cairo_surface_destroy(self.image);
            }
            if !self.proxy.is_null() {
                wl_proxy_marshal_flags(
                    self.proxy,
                    WL_BUFFER_DESTROY,
                    null(),
                    1,
                    WL_MARSHAL_FLAG_DESTROY,
                );
            }
            munmap(self.data, self.bytes);
        }
    }
}

/// Contexto Cairo de vida corta; la imagen y sus píxeles pertenecen a Buffer.
struct Canvas(Proxy);
impl Canvas {
    fn new(image: Proxy) -> Result<Self, String> {
        // SAFETY: imagen viva y válida de Buffer.
        let canvas = Self(unsafe { cairo_create(image) });
        if unsafe { cairo_status(canvas.0) } != 0 {
            return Err("No se pudo crear el contexto Cairo".into());
        }
        unsafe {
            cairo_select_font_face(canvas.0, c"monospace".as_ptr(), 0, 0);
            cairo_set_font_size(canvas.0, 16.0);
        }
        Ok(canvas)
    }
    fn color(&self, r: f64, g: f64, b: f64) {
        unsafe {
            cairo_set_source_rgb(self.0, r, g, b);
        }
    }
    fn rect(&self, x: f64, y: f64, w: f64, h: f64) {
        unsafe {
            cairo_rectangle(self.0, x, y, w, h);
            cairo_fill(self.0);
        }
    }
    fn clip(&self, x: f64, y: f64, w: f64, h: f64) {
        unsafe {
            cairo_save(self.0);
            cairo_rectangle(self.0, x, y, w, h);
            cairo_clip(self.0);
        }
    }
    fn unclip(&self) {
        unsafe {
            cairo_restore(self.0);
        }
    }
    fn text(&self, x: f64, y: f64, text: &str) -> Result<(), String> {
        let text = CString::new(text.replace('\0', "�")).map_err(|e| e.to_string())?;
        unsafe {
            cairo_move_to(self.0, x, y);
            cairo_show_text(self.0, text.as_ptr());
        }
        Ok(())
    }
    fn measure(&self, text: &str) -> Result<f64, String> {
        let text = CString::new(text.replace('\0', "�")).map_err(|e| e.to_string())?;
        let mut extents = TextExtents::default();
        unsafe {
            cairo_text_extents(self.0, text.as_ptr(), &mut extents);
        }
        Ok(extents.x_advance)
    }
}
impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            cairo_destroy(self.0);
        }
    }
}
