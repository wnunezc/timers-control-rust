#![cfg_attr(windows, windows_subsystem = "windows")]
#![deny(unsafe_op_in_unsafe_fn)]
//! Punto de entrada: Win32 en Windows, Wayland nativo en Linux.

mod app;
mod command;
mod duration;
#[cfg(target_os = "linux")]
mod linux;
mod timers;
#[cfg(windows)]
mod windows;

fn main() {
    #[cfg(windows)]
    if let Err(error) = windows::run() {
        windows::show_error(&error);
    }

    #[cfg(target_os = "linux")]
    if let Err(error) = linux::run() {
        eprintln!("Timer-Rust: {error}");
        std::process::exit(1);
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    eprintln!("Esta plataforma todavía no tiene adaptador gráfico.");
}
