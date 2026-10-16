//! Sonido Freedesktop decodificado por libsndfile y enviado a PulseAudio síncrono.
//! El proceso GUI espera un ayudante temporal del mismo ELF: así ninguna
//! biblioteca/caché de audio queda residente en la GUI tras sonar. Sin shell.

use super::library::Library;
use std::ffi::{CStr, c_char, c_void};
use std::ptr::{null, null_mut};

/// Evento de finalización del paquete sound-theme-freedesktop de la distribución.
/// Ruta explícita: no consulta preferencias de un escritorio ni extrae segmentos.
const SOUND: &CStr = c"/usr/share/sounds/freedesktop/stereo/complete.oga";

#[repr(C)]
#[derive(Default)]
struct SoundInfo {
    frames: i64,
    samplerate: i32,
    channels: i32,
    format: i32,
    sections: i32,
    seekable: i32,
}

// La macro conserva las firmas ABI en un solo sitio. Las funciones y sus
// bibliotecas viven en el mismo objeto; File/Stream se destruyen antes de la API.
macro_rules! api {
    ($name:ident,$soname:literal,{$(fn $function:ident($($arg:ident:$ty:ty),* $(,)?) $(->$ret:ty)?;)*}) => {
        struct $name { _library: Library, $($function: unsafe extern "C" fn($($ty),*) $(->$ret)?,)* }
        impl $name {
            fn load()->Result<Self,String> {
                let library=Library::open($soname)?;
                // SAFETY: firmas de sndfile.h/pulse/simple.h verificadas; cada
                // puntero solo se usa mientras esta estructura conserva Library.
                unsafe { Ok(Self {
                    $($function:library.symbol(CStr::from_bytes_with_nul(concat!(stringify!($function),"\0").as_bytes()).expect("Símbolo literal"))?,)*
                    _library:library,
                }) }
            }
        }
    };
}
api!(Sndfile, c"libsndfile.so.1", {
    fn sf_open(path: *const c_char, mode: i32, info: *mut SoundInfo) -> *mut c_void;
    fn sf_readf_short(file: *mut c_void, samples: *mut i16, frames: i64) -> i64;
    fn sf_error(file: *mut c_void) -> i32;
    fn sf_strerror(file: *mut c_void) -> *const c_char;
    fn sf_close(file: *mut c_void) -> i32;
});
struct SoundFile<'a> {
    pointer: *mut c_void,
    api: &'a Sndfile,
}
impl Drop for SoundFile<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.api.sf_close)(self.pointer);
        }
    }
}
fn file_error(api: &Sndfile, file: *mut c_void) -> String {
    // SAFETY: NULL tras fallo de apertura o un archivo vivo de esta misma API.
    let detail = unsafe { CStr::from_ptr((api.sf_strerror)(file)) }.to_string_lossy();
    format!("Audio Linux: {}: {detail}", SOUND.to_string_lossy())
}

#[repr(C)]
struct SampleSpec {
    format: i32,
    rate: u32,
    channels: u8,
}
#[repr(C)]
struct BufferAttr {
    maxlength: u32,
    tlength: u32,
    prebuf: u32,
    minreq: u32,
    fragsize: u32,
}

api!(Pulse, c"libpulse-simple.so.0", {
    fn pa_simple_new(
        server: *const c_char,
        name: *const c_char,
        direction: i32,
        device: *const c_char,
        stream: *const c_char,
        spec: *const SampleSpec,
        map: *const c_void,
        attributes: *const BufferAttr,
        error: *mut i32,
    ) -> *mut c_void;
    fn pa_simple_write(
        stream: *mut c_void,
        data: *const c_void,
        bytes: usize,
        error: *mut i32,
    ) -> i32;
    fn pa_simple_drain(stream: *mut c_void, error: *mut i32) -> i32;
    fn pa_simple_free(stream: *mut c_void);
    fn pa_strerror(error: i32) -> *const c_char;
});
struct Stream<'a> {
    pointer: *mut c_void,
    api: &'a Pulse,
}
impl Drop for Stream<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.api.pa_simple_free)(self.pointer);
        }
    }
}
fn error_text(api: &Pulse, error: i32) -> String {
    // SAFETY: la API permanece cargada durante la copia de su mensaje estático.
    format!(
        "Audio Linux: {}",
        unsafe { CStr::from_ptr((api.pa_strerror)(error)) }.to_string_lossy()
    )
}

/// Ejecuta únicamente el modo de sonido del propio binario y espera su fin.
/// El loader puede retener dependencias tras dlclose; el proceso temporal acota
/// ese coste a la alarma. No requiere instalar un reproductor ni crea una GUI.
pub fn play() -> Result<(), String> {
    // El enlace del kernel mantiene la misma versión incluso si se recompila o
    // reemplaza el archivo mientras la ventana está abierta.
    let output = std::process::Command::new("/proc/self/exe")
        .arg("--internal-system-sound")
        .output()
        .map_err(|e| format!("Audio Linux: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr);
    let detail = detail.trim();
    if detail.is_empty() {
        Err(format!(
            "Audio Linux: el reproductor terminó con {}",
            output.status
        ))
    } else {
        Err(detail.chars().take(1024).collect())
    }
}

/// Reproduce completo el evento del tema, sin cargar todo el PCM en memoria.
/// Bloquea la GUI hasta drenar; PULSE_SERVER/PULSE_SINK seleccionan servidor/salida.
/// Los errores de archivo o dispositivo llegan al estado visible de la ventana.
pub fn play_local() -> Result<(), String> {
    let sndfile = Sndfile::load()?;
    let mut info = SoundInfo::default();
    // SAFETY: ruta C estática, SF_INFO inicializado y ABI de sndfile.h; SFM_READ.
    let pointer = unsafe { (sndfile.sf_open)(SOUND.as_ptr(), 0x10, &mut info) };
    if pointer.is_null() {
        return Err(file_error(&sndfile, null_mut()));
    }
    let file = SoundFile {
        pointer,
        api: &sndfile,
    };
    // PulseAudio admite hasta 32 canales y 384 kHz. Se valida antes de convertir.
    if !(1..=32).contains(&info.channels)
        || !(1..=384000).contains(&info.samplerate)
        || info.frames <= 0
    {
        return Err("Audio Linux: parámetros del archivo no admitidos o sonido vacío".into());
    }
    let pulse = Pulse::load()?;
    let spec = SampleSpec {
        format: if cfg!(target_endian = "little") { 3 } else { 4 },
        rate: info.samplerate as u32,
        channels: info.channels as u8,
    };
    // 16 KiB de PCM reutilizables, independientemente de la duración del sonido.
    let mut samples = [0i16; 8192];
    let channels = info.channels as usize;
    let frames_per_block = (samples.len() / channels) as i64;
    let attributes = BufferAttr {
        maxlength: u32::MAX,
        tlength: (spec.rate / 10).max(1) * spec.channels as u32 * 2,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: u32::MAX,
    };
    let mut error = 0;
    // SAFETY: ABI de libpulse, estructuras repr(C), punteros vivos durante cada
    // llamada. La API simple espera buffers síncronos y copia antes de retornar.
    unsafe {
        let pointer = (pulse.pa_simple_new)(
            null(),
            c"Timer-Rust".as_ptr(),
            1,
            null(),
            c"Temporizador".as_ptr(),
            &spec,
            null(),
            &attributes,
            &mut error,
        );
        if pointer.is_null() {
            return Err(error_text(&pulse, error));
        }
        let stream = Stream {
            pointer,
            api: &pulse,
        };
        loop {
            // sf_readf_short escribe hasta frames_per_block * channels muestras.
            // Su retorno es un número de frames, no bytes ni muestras individuales.
            let frames =
                (sndfile.sf_readf_short)(file.pointer, samples.as_mut_ptr(), frames_per_block);
            if (sndfile.sf_error)(file.pointer) != 0 {
                return Err(file_error(&sndfile, file.pointer));
            }
            if frames == 0 {
                break;
            }
            if !(0..=frames_per_block).contains(&frames) {
                return Err("Audio Linux: longitud PCM inválida".into());
            }
            let bytes = frames as usize * channels * std::mem::size_of::<i16>();
            if (pulse.pa_simple_write)(stream.pointer, samples.as_ptr().cast(), bytes, &mut error)
                < 0
            {
                return Err(error_text(&pulse, error));
            }
        }
        if (pulse.pa_simple_drain)(stream.pointer, &mut error) < 0 {
            return Err(error_text(&pulse, error));
        }
    }
    Ok(())
}
