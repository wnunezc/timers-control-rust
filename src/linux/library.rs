//! Bibliotecas opcionales con propiedad RAII. Solo el audio las abre al necesitarse.
use std::ffi::{CStr, c_void};
#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(name: *const std::ffi::c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const std::ffi::c_char) -> *mut c_void;
    fn dlerror() -> *const std::ffi::c_char;
    fn dlclose(handle: *mut c_void) -> i32;
}
pub struct Library(*mut c_void);
impl Library {
    pub fn open(name: &CStr) -> Result<Self, String> {
        // SAFETY: cadena C viva, RTLD_NOW|RTLD_LOCAL. No expone símbolos globales.
        let handle = unsafe { dlopen(name.as_ptr(), 2) };
        if handle.is_null() {
            Err(Self::error())
        } else {
            Ok(Self(handle))
        }
    }
    fn error() -> String {
        // SAFETY: el loader devuelve NULL o una cadena válida hasta la próxima llamada.
        let message = unsafe { dlerror() };
        if message.is_null() {
            "No se pudo cargar la biblioteca de audio".into()
        } else {
            format!(
                "Audio Linux: {}",
                unsafe { CStr::from_ptr(message) }.to_string_lossy()
            )
        }
    }
    /// # Safety
    /// T debe ser exactamente el puntero a función C publicado para este símbolo.
    /// No invocar la función después de destruir Library.
    pub unsafe fn symbol<T: Copy>(&self, name: &CStr) -> Result<T, String> {
        assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<*mut c_void>());
        // SAFETY: handle propio; la firma de T y su vida se validan en el llamador.
        unsafe {
            dlerror();
            let pointer = dlsym(self.0, name.as_ptr());
            if pointer.is_null() {
                return Err(Self::error());
            }
            Ok(std::mem::transmute_copy(&pointer))
        }
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: handle propio; los recursos y llamadas de la API ya terminaron.
        unsafe {
            dlclose(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_optional_library_returns_an_error() {
        assert!(Library::open(c"libtimer_rust_nonexistent_test_library.so").is_err());
    }
}
