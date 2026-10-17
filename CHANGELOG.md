# Changelog

## [0.1.2 Alpha](https://github.com/wnunezc/timers-control-rust/releases/tag/v0.1.2-alpha) — 2026-10-17

### Corregido

- Linux: fallo de inicio con `XKB-822` / `xkbcommon rechazó el keymap` al recibir un mapa de teclado válido.

El ejecutable Windows se recompiló para esta versión, sin cambios funcionales.

## [0.1.1 Alpha](https://github.com/wnunezc/timers-control-rust/releases/tag/v0.1.1-alpha) — 2026-10-16

### Añadido

- Soporte básico de Linux x86_64 con glibc y Wayland.
- Edición de comandos UTF-8 y desplazamiento de la lista en Linux.
- Alarma Linux con el sonido del paquete `sound-theme-freedesktop`.

### Mejorado

- Menor consumo de memoria en Linux mediante reutilización de filas, glifos y buffers.
- Carga de los componentes de audio Linux solo durante la reproducción.

## [0.1.0 Alpha](https://github.com/wnunezc/timers-control-rust/releases/tag/v0.1.0-alpha) — 2026-10-16

### Añadido

- Primera versión para Windows x64.
- Creación, pausa, continuación, detención y configuración de cuentas por comandos.
- Alarma con el sonido del sistema.

Los detalles de implementación están en el [historial técnico](docs/development/HISTORY.md).
