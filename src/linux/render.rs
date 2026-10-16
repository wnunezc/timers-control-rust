//! Rasterización Rust sobre wl_shm: RGB565 negociado y ARGB32 de respaldo.
//! Cada buffer conserva revisiones de lista/pie; solo se repintan regiones cambiadas.
use super::{entry::Entry, font::Font, native::*};
use std::{
    cell::Cell,
    ffi::c_void,
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    ptr::{null, null_mut},
};

pub struct View<'a> {
    pub rows: &'a [String],
    pub status: &'a str,
    pub entry: &'a Entry,
    pub scroll: usize,
    pub rows_version: u64,
    pub footer_version: u64,
}
pub struct Damage {
    pub list: bool,
    pub footer: bool,
}
/// Solo el hilo GUI escribe el mapping, tras wl_buffer.release. Box mantiene
/// estable el dato del listener; cada buffer recuerda su contenido por revisiones.
pub struct Buffer {
    pub proxy: Proxy,
    pub width: i32,
    pub height: i32,
    pub busy: Cell<bool>,
    data: *mut c_void,
    bytes: usize,
    stride: usize,
    rgb565: bool,
    list: Option<(u64, usize)>,
    footer: Option<u64>,
}
unsafe extern "C" fn released(data: *mut c_void, _: Proxy) {
    // SAFETY: listener eliminado antes de liberar el Box que posee este dato.
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
    pub fn new(shm: Proxy, width: i32, height: i32, rgb565: bool) -> Result<Box<Self>, String> {
        let shm_format = if rgb565 {
            WL_SHM_FORMAT_RGB565
        } else {
            WL_SHM_FORMAT_ARGB8888
        };
        let stride = width
            .checked_mul(if rgb565 { 2 } else { 4 })
            .and_then(|n| n.checked_add(3))
            .map(|n| n & !3)
            .ok_or("Ancho fuera de rango")?;
        if width <= 0 || stride <= 0 {
            return Err("Ancho inválido".into());
        }
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
            stride: stride as usize,
            rgb565,
            list: None,
            footer: None,
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
                shm_format,
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
        }
        Ok(buffer)
    }

    pub fn draw(&mut self, view: &View<'_>, font: &mut Font) -> Result<Damage, String> {
        assert!(
            !self.busy.get(),
            "No escribir un buffer prestado al compositor"
        );
        let damage = Damage {
            list: self.list != Some((view.rows_version, view.scroll)),
            footer: self.footer != Some(view.footer_version),
        };
        let width = self.width;
        let height = self.height;
        // SAFETY: mapping propio y exclusivo por &mut self, tamaño validado en new.
        let pixels = unsafe { std::slice::from_raw_parts_mut(self.data.cast(), self.bytes) };
        let mut canvas = Canvas {
            pixels,
            stride: self.stride,
            width,
            height,
            rgb565: self.rgb565,
            clip: (0, 0, width, height),
            color: [0, 0, 0],
        };
        if damage.list {
            canvas.color = [255; 3];
            canvas.rect(0, 0, width, height - 80);
            canvas.clip = (8, 6, width - 8, height - 88);
            canvas.color = [13; 3];
            for (i, row) in view.rows.iter().skip(view.scroll).enumerate() {
                let y = 22 + i as i32 * 24;
                if y > height - 88 {
                    break;
                }
                canvas.text(font, 8, y, row)?;
            }
            canvas.clip = (0, 0, width, height);
            let visible = ((height - 96).max(24) / 24) as usize;
            if view.rows.len() > visible {
                let track = height - 96;
                let thumb =
                    ((track as i64 * visible as i64 / view.rows.len() as i64) as i32).max(8);
                let maximum = view.rows.len().saturating_sub(visible).max(1);
                let y = 8 + ((track - thumb) as i64 * view.scroll as i64 / maximum as i64) as i32;
                canvas.color = [153; 3];
                canvas.rect(width - 5, y, 3, thumb);
            }
        }
        if damage.footer {
            canvas.clip = (0, height - 80, width, height);
            canvas.color = [255; 3];
            canvas.rect(0, height - 80, width, 80);
            canvas.color = [240; 3];
            canvas.rect(8, height - 72, width - 16, 32);
            canvas.clip = (8, height - 72, width - 8, height - 40);
            canvas.color = [26; 3];
            for (i, line) in view.status.lines().take(2).enumerate() {
                canvas.text(font, 10, height - 59 + i as i32 * 16, line)?;
            }
            canvas.clip = (0, height - 80, width, height);
            canvas.color = [128; 3];
            canvas.rect(8, height - 34, width - 16, 26);
            canvas.color = [255; 3];
            canvas.rect(9, height - 33, width - 18, 24);
            canvas.clip = (11, height - 32, width - 11, height - 9);
            let caret = font.measure(&view.entry.text[..view.entry.cursor])?;
            let offset = (caret - (width - 30)).max(0);
            if view.entry.selected && !view.entry.text.is_empty() {
                canvas.color = [0, 89, 179];
                let length = font.measure(&view.entry.text)?;
                canvas.rect(12 - offset, height - 31, length, 21);
                canvas.color = [255; 3];
            } else {
                canvas.color = [13; 3];
            }
            canvas.text(font, 12 - offset, height - 15, &view.entry.text)?;
            canvas.color = [13; 3];
            canvas.rect(12 + caret - offset, height - 30, 1, 19);
        }
        self.list = Some((view.rows_version, view.scroll));
        self.footer = Some(view.footer_version);
        Ok(damage)
    }
}
impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: se elimina listener antes del Box; el servidor mantiene su mapping.
        unsafe {
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
/// Lienzo prestado sin asignaciones; recorta antes de indexar cualquier píxel.
struct Canvas<'a> {
    pixels: &'a mut [u8],
    stride: usize,
    width: i32,
    height: i32,
    rgb565: bool,
    clip: (i32, i32, i32, i32),
    color: [u8; 3],
}
impl Canvas<'_> {
    fn rect(&mut self, x: i32, y: i32, width: i32, height: i32) {
        let x0 = x.max(self.clip.0).max(0);
        let y0 = y.max(self.clip.1).max(0);
        let x1 = x.saturating_add(width).min(self.clip.2).min(self.width);
        let y1 = y.saturating_add(height).min(self.clip.3).min(self.height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        // Relleno opaco: no lee ni mezcla el destino por cada píxel.
        if self.rgb565 {
            let value = ((self.color[0] as u16 >> 3) << 11)
                | ((self.color[1] as u16 >> 2) << 5)
                | (self.color[2] as u16 >> 3);
            let bytes = value.to_ne_bytes();
            for y in y0..y1 {
                let start = y as usize * self.stride + x0 as usize * 2;
                for pixel in self.pixels[start..start + (x1 - x0) as usize * 2]
                    .as_chunks_mut::<2>()
                    .0
                {
                    pixel.copy_from_slice(&bytes);
                }
            }
        } else {
            let value = 0xff000000u32
                | ((self.color[0] as u32) << 16)
                | ((self.color[1] as u32) << 8)
                | self.color[2] as u32;
            let bytes = value.to_ne_bytes();
            for y in y0..y1 {
                let start = y as usize * self.stride + x0 as usize * 4;
                for pixel in self.pixels[start..start + (x1 - x0) as usize * 4]
                    .as_chunks_mut::<4>()
                    .0
                {
                    pixel.copy_from_slice(&bytes);
                }
            }
        }
    }

    fn pixel(&mut self, x: i32, y: i32, alpha: u8) {
        let offset = y as usize * self.stride + x as usize * if self.rgb565 { 2 } else { 4 };
        let old = if self.rgb565 {
            let value = u16::from_ne_bytes([self.pixels[offset], self.pixels[offset + 1]]);
            [
                (((value >> 11) & 31) as u32 * 255 / 31) as u8,
                (((value >> 5) & 63) as u32 * 255 / 63) as u8,
                ((value & 31) as u32 * 255 / 31) as u8,
            ]
        } else {
            let value = u32::from_ne_bytes(self.pixels[offset..offset + 4].try_into().unwrap());
            [(value >> 16) as u8, (value >> 8) as u8, value as u8]
        };
        let mut mixed = [0u8; 3];
        for i in 0..3 {
            mixed[i] =
                ((self.color[i] as u32 * alpha as u32 + old[i] as u32 * (255 - alpha as u32) + 127)
                    / 255) as u8;
        }
        if self.rgb565 {
            let value = ((mixed[0] as u16 >> 3) << 11)
                | ((mixed[1] as u16 >> 2) << 5)
                | (mixed[2] as u16 >> 3);
            self.pixels[offset..offset + 2].copy_from_slice(&value.to_ne_bytes());
        } else {
            let value = 0xff000000u32
                | ((mixed[0] as u32) << 16)
                | ((mixed[1] as u32) << 8)
                | mixed[2] as u32;
            self.pixels[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
        }
    }
    fn text(&mut self, font: &mut Font, mut x: i32, y: i32, text: &str) -> Result<(), String> {
        for character in text.chars() {
            let glyph = font.glyph(character)?;
            for gy in 0..glyph.height {
                for gx in 0..glyph.width {
                    let px = x + glyph.left + gx as i32;
                    let py = y - glyph.top + gy as i32;
                    if px >= self.clip.0.max(0)
                        && px < self.clip.2.min(self.width)
                        && py >= self.clip.1.max(0)
                        && py < self.clip.3.min(self.height)
                    {
                        let alpha = glyph.pixels[gy * glyph.width + gx];
                        if alpha != 0 {
                            self.pixel(px, py, alpha);
                        }
                    }
                }
            }
            x = x.saturating_add(glyph.advance);
            if x >= self.clip.2 {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editing_footer_preserves_list_and_unchanged_frame_writes_nothing() {
        let mut font = Font::new().unwrap();
        for rgb565 in [false, true] {
            let width = 241;
            let height = 150;
            let stride = if rgb565 { 484 } else { 964 };
            let bytes = stride * height as usize;
            // SAFETY: mapping anónimo propio del test, destruido por Buffer::drop.
            let data = unsafe { mmap(null_mut(), bytes, 3, 0x22, -1, 0) };
            assert_ne!(data as isize, -1);
            let mut buffer = Buffer {
                proxy: null_mut(),
                width,
                height,
                busy: Cell::new(false),
                data,
                bytes,
                stride,
                rgb565,
                list: None,
                footer: None,
            };
            let rows = vec!["00:00:10  café  [pausado]".into()];
            let mut entry = Entry::default();
            let initial_view = View {
                rows: &rows,
                status: "Prueba",
                entry: &entry,
                scroll: 0,
                rows_version: 1,
                footer_version: 1,
            };
            let initial = buffer.draw(&initial_view, &mut font).unwrap();
            assert!(initial.list && initial.footer);
            let upper =
                unsafe { std::slice::from_raw_parts(data.cast::<u8>(), stride * 70) }.to_vec();
            entry.insert("sonido");
            let view = View {
                rows: &rows,
                status: "Prueba",
                entry: &entry,
                scroll: 0,
                rows_version: 1,
                footer_version: 2,
            };
            let changed = buffer.draw(&view, &mut font).unwrap();
            assert!(!changed.list && changed.footer);
            assert_eq!(
                unsafe { std::slice::from_raw_parts(data.cast::<u8>(), stride * 70) },
                upper
            );
            let stable = buffer.draw(&view, &mut font).unwrap();
            assert!(!stable.list && !stable.footer);
        }
    }
    #[test]
    fn alpha_blending_is_consistent_between_formats() {
        for rgb565 in [false, true] {
            let mut pixels = [255u8; 4];
            let mut canvas = Canvas {
                pixels: &mut pixels,
                stride: 4,
                width: 1,
                height: 1,
                rgb565,
                clip: (0, 0, 1, 1),
                color: [0; 3],
            };
            canvas.pixel(0, 0, 128);
            if rgb565 {
                let p = u16::from_ne_bytes(pixels[..2].try_into().unwrap());
                assert_eq!(p, (15 << 11) | (31 << 5) | 15);
            } else {
                assert_eq!(u32::from_ne_bytes(pixels), 0xff7f7f7f);
            }
        }
    }
    #[test]
    fn clipped_fill_preserves_padding_in_both_formats() {
        for rgb565 in [false, true] {
            let bpp = if rgb565 { 2 } else { 4 };
            let stride = if rgb565 { 8 } else { 12 };
            let mut pixels = vec![0xaa; stride * 2];
            let mut canvas = Canvas {
                pixels: &mut pixels,
                stride,
                width: 3,
                height: 2,
                rgb565,
                clip: (0, 0, 3, 2),
                color: [255; 3],
            };
            canvas.rect(-5, -5, i32::MAX, i32::MAX);
            for y in 0..2 {
                for p in 0..3 {
                    assert!(
                        pixels[y * stride + p * bpp..y * stride + (p + 1) * bpp]
                            .iter()
                            .all(|&v| v == 255)
                    );
                }
                assert!(
                    pixels[y * stride + 3 * bpp..(y + 1) * stride]
                        .iter()
                        .all(|&v| v == 0xaa)
                );
            }
        }
    }
}
