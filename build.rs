//! Genera únicamente los metadatos ABI de xdg-shell con herramientas del sistema.
//! La lógica de ventana, entrada, dibujo y temporizadores sigue escrita en Rust.
use std::{env, path::PathBuf, process::Command};

fn checked(command: &mut Command) {
    let status = command
        .status()
        .expect("No se pudo ejecutar una herramienta de construcción");
    assert!(status.success(), "Falló la herramienta: {command:?}");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }
    assert_eq!(
        env::var("CARGO_CFG_TARGET_POINTER_WIDTH").as_deref(),
        Ok("64"),
        "El adaptador Linux actual requiere ABI de 64 bits"
    );
    println!("cargo:rerun-if-env-changed=XDG_SHELL_XML");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=AR");
    let xml = env::var("XDG_SHELL_XML")
        .unwrap_or_else(|_| "/usr/share/wayland-protocols/stable/xdg-shell/xdg-shell.xml".into());
    println!("cargo:rerun-if-changed={xml}");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR de Cargo"));
    let source = out.join("xdg-shell-protocol.c");
    let object = out.join("xdg-shell-protocol.o");
    checked(
        Command::new("wayland-scanner")
            .arg("private-code")
            .arg(&xml)
            .arg(&source),
    );
    let flags = Command::new("pkg-config")
        .args(["--cflags", "wayland-client"])
        .output()
        .expect("Se requiere pkg-config");
    assert!(
        flags.status.success(),
        "Instalar los headers de wayland-client"
    );
    checked(
        Command::new(env::var("CC").unwrap_or_else(|_| "cc".into()))
            .args(["-O2", "-fPIC", "-c"])
            .arg(&source)
            .arg("-o")
            .arg(&object)
            .args(String::from_utf8_lossy(&flags.stdout).split_whitespace()),
    );
    checked(
        Command::new(env::var("AR").unwrap_or_else(|_| "ar".into()))
            .arg("crs")
            .arg(out.join("libtimer_xdg.a"))
            .arg(&object),
    );
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=timer_xdg");
}
