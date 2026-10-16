//! Fuente fija del sistema: Fontconfig localiza el archivo una vez y se libera.
//! FreeType rasteriza glifos individuales; no hay Cairo ni backend X11.
use std::{
    collections::HashMap,
    ffi::{CStr, CString, c_char, c_int, c_long, c_short, c_ulong, c_void},
    ptr::null_mut,
};

type Handle = *mut c_void;
#[repr(C)]
struct Generic {
    data: Handle,
    finalizer: Handle,
}
/// Prefijo público de FT_FaceRec hasta glyph, verificado contra headers al compilar.
#[repr(C)]
struct Face {
    counts: [c_long; 5],
    names: [*mut c_char; 2],
    fixed_count: c_int,
    fixed_sizes: Handle,
    charmap_count: c_int,
    charmaps: Handle,
    generic: Generic,
    bbox: [c_long; 4],
    sizes: [c_short; 8],
    glyph: *mut Slot,
}
#[repr(C)]
struct Bitmap {
    rows: u32,
    width: u32,
    pitch: c_int,
    buffer: *mut u8,
    grays: u16,
    mode: u8,
    palette_mode: u8,
    palette: Handle,
}
/// Prefijo público de FT_GlyphSlotRec. FT_Pos/FT_Fixed son long en esta ABI Linux64.
#[repr(C)]
struct Slot {
    library: Handle,
    face: Handle,
    next: Handle,
    index: u32,
    generic: Generic,
    metrics: [c_long; 8],
    linear_advance: [c_long; 2],
    advance: [c_long; 2],
    format: u32,
    bitmap: Bitmap,
    left: c_int,
    top: c_int,
}
const _: () = assert!(std::mem::offset_of!(Face, glyph) == 152);
const _: () = assert!(std::mem::offset_of!(Slot, bitmap) == 152);
const _: () = assert!(std::mem::offset_of!(Slot, left) == 192);

#[link(name = "freetype")]
unsafe extern "C" {
    fn FT_Init_FreeType(library: *mut Handle) -> c_int;
    fn FT_New_Face(
        library: Handle,
        path: *const c_char,
        index: c_long,
        face: *mut *mut Face,
    ) -> c_int;
    fn FT_Set_Pixel_Sizes(face: *mut Face, width: u32, height: u32) -> c_int;
    fn FT_Load_Char(face: *mut Face, code: c_ulong, flags: c_int) -> c_int;
    fn FT_Done_Face(face: *mut Face) -> c_int;
    fn FT_Done_FreeType(library: Handle) -> c_int;
}
#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcNameParse(name: *const u8) -> Handle;
    fn FcConfigSubstitute(config: Handle, pattern: Handle, kind: c_int) -> c_int;
    fn FcDefaultSubstitute(pattern: Handle);
    fn FcFontMatch(config: Handle, pattern: Handle, result: *mut c_int) -> Handle;
    fn FcPatternGetString(
        pattern: Handle,
        name: *const c_char,
        index: c_int,
        value: *mut *mut u8,
    ) -> c_int;
    fn FcPatternDestroy(pattern: Handle);
    fn FcFini();
}
struct Pattern(Handle);
impl Drop for Pattern {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { FcPatternDestroy(self.0) }
        }
    }
}
fn font_path() -> Result<CString, String> {
    // SAFETY: patrones propios y cadenas estáticas. La ruta se copia antes de
    // liberar el patrón. Ningún otro módulo de esta aplicación usa Fontconfig.
    unsafe {
        let requested = Pattern(FcNameParse(c"monospace:pixelsize=16".as_ptr().cast()));
        if requested.0.is_null() || FcConfigSubstitute(null_mut(), requested.0, 0) == 0 {
            return Err("No se pudo localizar la fuente monospace".into());
        }
        FcDefaultSubstitute(requested.0);
        let mut result = 0;
        let matched = Pattern(FcFontMatch(null_mut(), requested.0, &mut result));
        let mut path = null_mut();
        if matched.0.is_null()
            || FcPatternGetString(matched.0, c"file".as_ptr(), 0, &mut path) != 0
            || path.is_null()
        {
            return Err("No hay una fuente monospace instalada".into());
        }
        Ok(CStr::from_ptr(path.cast()).to_owned())
    }
}

pub struct Glyph {
    pub width: usize,
    pub height: usize,
    pub left: i32,
    pub top: i32,
    pub advance: i32,
    pub pixels: Vec<u8>,
}
/// Caché acotada: 256 glifos y 128 KiB de bitmaps en total como máximo.
/// Las dimensiones habituales a 16 px son mucho menores; no se guarda texto duplicado.
pub struct Font {
    library: Handle,
    face: *mut Face,
    _path: CString,
    glyphs: HashMap<char, Glyph>,
    cache_bytes: usize,
}
impl Font {
    pub fn new() -> Result<Self, String> {
        // Las pruebas Rust corren en paralelo; FcFini solo puede cerrar el
        // catálogo cuando ningún otro constructor de prueba lo está consultando.
        #[cfg(test)]
        let _catalog_guard = {
            static CATALOG: std::sync::Mutex<()> = std::sync::Mutex::new(());
            CATALOG.lock().unwrap()
        };
        let path = font_path();
        // SAFETY: font_path ya destruyó ambos patrones; esta GUI es el único usuario.
        unsafe { FcFini() };
        let path = path?;
        let mut font = Self {
            library: null_mut(),
            face: null_mut(),
            _path: path,
            glyphs: HashMap::new(),
            cache_bytes: 0,
        };
        // SAFETY: handles inicializados, ruta viva durante FT_New_Face. El objeto
        // conserva ruta, fuente y biblioteca hasta Drop, incluso si falla la configuración.
        unsafe {
            if FT_Init_FreeType(&mut font.library) != 0
                || FT_New_Face(font.library, font._path.as_ptr(), 0, &mut font.face) != 0
                || FT_Set_Pixel_Sizes(font.face, 0, 16) != 0
            {
                return Err("No se pudo abrir la fuente fija de 16 px".into());
            }
        }
        Ok(font)
    }
    pub fn glyph(&mut self, character: char) -> Result<&Glyph, String> {
        if !self.glyphs.contains_key(&character) {
            if self.glyphs.len() == 256 {
                self.glyphs.clear();
                self.cache_bytes = 0;
            }
            // SAFETY: face propio; FT_LOAD_RENDER produce el bitmap del slot.
            // La copia queda en Rust antes de que otra carga reemplace ese slot.
            let glyph = unsafe {
                if FT_Load_Char(self.face, character as c_ulong, 4) != 0 {
                    return Err("No se pudo dibujar un carácter".into());
                }
                let slot = &*(*self.face).glyph;
                let bitmap = &slot.bitmap;
                let width = bitmap.width as usize;
                let height = bitmap.rows as usize;
                let pitch = bitmap.pitch.unsigned_abs() as usize;
                let bytes = width
                    .checked_mul(height)
                    .filter(|&n| n <= 65536)
                    .ok_or("Glifo demasiado grande")?;
                let mut pixels = Vec::with_capacity(bytes);
                if bytes > 0 {
                    let minimum_pitch = if bitmap.mode == 1 {
                        width.div_ceil(8)
                    } else {
                        width
                    };
                    if bitmap.buffer.is_null()
                        || pitch < minimum_pitch
                        || ![1, 2].contains(&bitmap.mode)
                        || (bitmap.mode == 2 && bitmap.grays != 256)
                    {
                        return Err("Formato de glifo no admitido".into());
                    }
                    for y in 0..height {
                        let row = bitmap.buffer.offset(y as isize * bitmap.pitch as isize);
                        for x in 0..width {
                            pixels.push(if bitmap.mode == 1 {
                                if *row.add(x / 8) & (0x80 >> (x % 8)) != 0 {
                                    255
                                } else {
                                    0
                                }
                            } else {
                                *row.add(x)
                            });
                        }
                    }
                }
                Glyph {
                    width,
                    height,
                    left: slot.left,
                    top: slot.top,
                    advance: (slot.advance[0] / 64) as i32,
                    pixels,
                }
            };
            if self.cache_bytes + glyph.pixels.len() > 128 * 1024 {
                self.glyphs.clear();
                self.cache_bytes = 0;
            }
            self.cache_bytes += glyph.pixels.len();
            self.glyphs.insert(character, glyph);
        }
        Ok(&self.glyphs[&character])
    }
    pub fn measure(&mut self, text: &str) -> Result<i32, String> {
        text.chars()
            .try_fold(0i32, |x, ch| Ok(x.saturating_add(self.glyph(ch)?.advance)))
    }
}
impl Drop for Font {
    fn drop(&mut self) {
        // SAFETY: orden inverso de creación; ningún glifo Rust depende del slot.
        unsafe {
            if !self.face.is_null() {
                FT_Done_Face(self.face);
            }
            if !self.library.is_null() {
                FT_Done_FreeType(self.library);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_font_handles_utf8_and_reuses_glyph_bitmaps() {
        let mut font = Font::new().unwrap();
        let width = font.measure("café").unwrap();
        assert!(width > 0);
        let pointer = font.glyph('é').unwrap().pixels.as_ptr();
        let bytes = font.cache_bytes;
        assert_eq!(font.measure("café").unwrap(), width);
        assert_eq!(font.glyph('é').unwrap().pixels.as_ptr(), pointer);
        assert_eq!(font.cache_bytes, bytes);
        assert!(bytes <= 128 * 1024);
    }
}
