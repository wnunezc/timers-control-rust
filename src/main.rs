#![cfg_attr(windows, windows_subsystem = "windows")]
#![deny(unsafe_op_in_unsafe_fn)]
//! Punto de entrada. La primera implementación gráfica está limitada a Windows.

mod app;
mod command;
mod duration;
mod timers;
#[cfg(windows)]
mod windows;

fn main() {
    #[cfg(windows)]
    if let Err(error) = windows::run() {
        windows::show_error(&error);
    }

    #[cfg(not(windows))]
    eprintln!("Esta primera prueba implementa únicamente Windows.");
}
