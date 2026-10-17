# Desarrollo

Timer-Rust usa la biblioteca estándar de Rust y FFI a las bibliotecas nativas de
cada plataforma. No tiene dependencias de crates ni un runtime asíncrono.
El código compartido resuelve comandos y temporizadores; cada adaptador se encarga
de la ventana, la entrada y el audio.

## Documentación técnica

- [Arquitectura](development/ARCHITECTURE.md): módulos, estados y recursos nativos.
- [Pruebas](development/TESTING.md): comandos de verificación y cobertura.
- [Historial técnico](development/HISTORY.md): evolución, correcciones y tamaños.
- [Memoria Linux](development/MEMORY.md): análisis y mediciones de la optimización.
- [Laboratorio Linux](development/LINUX-LAB.md): entorno usado para integración.
- [Avisos de componentes](THIRD-PARTY-NOTICES.md).

## Compilar en Windows

Se necesitan Rust, Cargo y las herramientas de enlace/SDK del destino MSVC x64.
El proyecto usa la edición Rust 2024; todavía no tiene una MSRV establecida.
La compilación Windows se ha verificado con Rust/Cargo 1.98.1.

Desde PowerShell, en la raíz del repositorio:

```powershell
cargo build --release --locked --target x86_64-pc-windows-msvc
& .\target\x86_64-pc-windows-msvc\release\timer-rust.exe
```

`.cargo/config.toml` incluye el runtime C estáticamente para ese destino. Las DLL
del sistema siguen siendo necesarias. No se requiere VCRUNTIME140.dll.

Para mantener un ejecutable de revisión separado:

```powershell
cargo build --target-dir target/review
```

## Compilar en Linux

Se necesitan Rust/Cargo, compilador C, `ar`, pkg-config, wayland-scanner, los
protocolos Wayland y los headers de las bibliotecas gráficas.

En Void Linux, como administrador:

```sh
xbps-install -S gcc pkg-config rust cargo wayland-devel wayland-protocols freetype-devel fontconfig-devel libxkbcommon-devel libpulseaudio libsndfile sound-theme-freedesktop dejavu-fonts-ttf
```

Desde el repositorio, como usuario normal:

```sh
cargo build --release --locked
./target/release/timer-rust
```

El entorno de referencia usa Rust/Cargo 1.99.0, glibc 2.41 y Sway 1.12. La ejecución
requiere una sesión Wayland. Los requisitos del usuario final están en la
[guía Linux](LINUX.md).

`build.rs` genera las tablas ABI de xdg-shell y contrasta los offsets de FreeType
usados por Rust con los headers instalados. El XML predeterminado es
`/usr/share/wayland-protocols/stable/xdg-shell/xdg-shell.xml`.
`XDG_SHELL_XML` permite indicar otra ruta; `CC` y `AR` seleccionan las herramientas.
Estas variables no configuran por sí solas una toolchain de compilación cruzada.

## Criterios del proyecto

- Mantener el núcleo independiente de las API gráficas y sin `unsafe`.
- Concentrar FFI y propiedad de recursos en los adaptadores de plataforma.
- Compartir validación, comandos y transiciones de estado.
- Preferir composición y tipos concretos; añadir abstracciones cuando haya un uso real.
- Evitar trabajo periódico sin cuentas activas y asignaciones en el camino de dibujo.
- Medir RAM y CPU antes de atribuir un ahorro a un cambio.

El perfil release optimiza por tamaño, usa LTO y una unidad de compilación,
elimina símbolos y aborta ante panic.

## Preparar una versión

1. Actualizar `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md` y las notas en `docs/releases/`.
2. Ejecutar las [comprobaciones](development/TESTING.md) en ambos sistemas.
3. Compilar los distribuibles y probar el ejecutable extraído del paquete Linux.
4. Incluir la guía, los avisos de componentes y un SHA-256 interno del ELF en el tar.gz.
5. Generar `SHA256SUMS.txt` para las descargas, verificar permisos y conservar los créditos.
6. Crear el commit y el tag firmados; publicar como pre-release mientras siga en alpha.

Las notas de versión describen cambios de uso y cómo actualizar. Los detalles de
implementación y las mediciones se mantienen en esta sección técnica.
