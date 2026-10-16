//! Sonido Freedesktop decodificado por libsndfile y enviado a PulseAudio síncrono.
//! La conexión existe solo mientras suena; no se mantiene un hilo de audio propio.

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

#[link(name = "sndfile")]
unsafe extern "C" {
    fn sf_open(path: *const c_char, mode: i32, info: *mut SoundInfo) -> *mut c_void;
    fn sf_readf_short(file: *mut c_void, samples: *mut i16, frames: i64) -> i64;
    fn sf_error(file: *mut c_void) -> i32;
    fn sf_strerror(file: *mut c_void) -> *const c_char;
    fn sf_close(file: *mut c_void) -> i32;
}

struct SoundFile(*mut c_void);
impl Drop for SoundFile {
    fn drop(&mut self) {
        // SAFETY: único propietario del SNDFILE no nulo devuelto por sf_open.
        unsafe { sf_close(self.0) };
    }
}

fn file_error(file: *mut c_void) -> String {
    // SAFETY: sf_strerror acepta NULL tras fallo de apertura o un SNDFILE vivo;
    // su texto se copia antes de cerrar el archivo o realizar otra operación.
    let detail = unsafe { CStr::from_ptr(sf_strerror(file)) }.to_string_lossy();
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

#[link(name = "pulse-simple")]
unsafe extern "C" {
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
}
#[link(name = "pulse")]
unsafe extern "C" {
    fn pa_strerror(error: i32) -> *const c_char;
}

struct Stream(*mut c_void);
impl Drop for Stream {
    fn drop(&mut self) {
        // SAFETY: recurso no nulo creado por pa_simple_new y usado en este hilo.
        unsafe {
            pa_simple_free(self.0);
        }
    }
}

fn error_text(error: i32) -> String {
    // SAFETY: pa_strerror devuelve una cadena estática terminada para cualquier código.
    format!(
        "Audio Linux: {}",
        unsafe { CStr::from_ptr(pa_strerror(error)) }.to_string_lossy()
    )
}

/// Reproduce completo el evento del tema, sin cargar todo el PCM en memoria.
/// Bloquea la GUI hasta drenar; PULSE_SERVER/PULSE_SINK seleccionan servidor/salida.
/// Los errores de archivo o dispositivo llegan al estado visible de la ventana.
pub fn play() -> Result<(), String> {
    let mut info = SoundInfo::default();
    // SAFETY: ruta C estática, SF_INFO inicializado y ABI de sndfile.h; SFM_READ.
    let pointer = unsafe { sf_open(SOUND.as_ptr(), 0x10, &mut info) };
    if pointer.is_null() {
        return Err(file_error(null_mut()));
    }
    let file = SoundFile(pointer);
    // PulseAudio admite hasta 32 canales y 384 kHz. Se valida antes de convertir.
    if !(1..=32).contains(&info.channels)
        || !(1..=384000).contains(&info.samplerate)
        || info.frames <= 0
    {
        return Err("Audio Linux: parámetros del archivo no admitidos o sonido vacío".into());
    }
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
        let pointer = pa_simple_new(
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
            return Err(error_text(error));
        }
        let stream = Stream(pointer);
        loop {
            // sf_readf_short escribe hasta frames_per_block * channels muestras.
            // Su retorno es un número de frames, no bytes ni muestras individuales.
            let frames = sf_readf_short(file.0, samples.as_mut_ptr(), frames_per_block);
            if sf_error(file.0) != 0 {
                return Err(file_error(file.0));
            }
            if frames == 0 {
                break;
            }
            if !(0..=frames_per_block).contains(&frames) {
                return Err("Audio Linux: longitud PCM inválida".into());
            }
            let bytes = frames as usize * channels * std::mem::size_of::<i16>();
            if pa_simple_write(stream.0, samples.as_ptr().cast(), bytes, &mut error) < 0 {
                return Err(error_text(error));
            }
        }
        if pa_simple_drain(stream.0, &mut error) < 0 {
            return Err(error_text(error));
        }
    }
    Ok(())
}
