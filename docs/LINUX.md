# Linux

Timer-Rust necesita Linux x86_64 con glibc y una sesión Wayland. Se ha probado en
Void Linux con Sway, también dentro de WSLg. El soporte sigue en alpha.

## Requisitos

Para abrir la aplicación se necesitan Wayland, FreeType, Fontconfig, xkbcommon,
libgcc_s y una fuente monospace instalada. Para el sonido se necesitan libsndfile,
las bibliotecas cliente de PulseAudio y un servidor PulseAudio o PipeWire con
compatibilidad PulseAudio.

En Void Linux, instala los paquetes como administrador:

```sh
xbps-install -S wayland freetype fontconfig libxkbcommon libsndfile libpulseaudio sound-theme-freedesktop dejavu-fonts-ttf
```

En otras distribuciones, los nombres de los paquetes pueden variar. Los paquetes
de desarrollo y Rust solo hacen falta para [compilar desde el código](DEVELOPMENT.md).

## Descargar y ejecutar

Descarga `timer-rust-v0.1.2-alpha-linux-x86_64.tar.gz` desde
[Releases](https://github.com/wnunezc/timers-control-rust/releases/tag/v0.1.2-alpha).
En la carpeta de descarga:

```sh
mkdir timer-rust
tar -xzf timer-rust-v0.1.2-alpha-linux-x86_64.tar.gz -C timer-rust
cd timer-rust
sha256sum -c SHA256SUMS.txt
./timer-rust
```

El checksum incluido en el paquete comprueba el ejecutable extraído. Para
comprobar la descarga completa, usa el `SHA256SUMS.txt` publicado junto al tar.gz
en la página de la versión, antes de extraerlo.

Ejecuta la aplicación como usuario normal desde tu sesión Wayland. No necesita
instalación y no usa XWayland ni Wine. Los comandos están en el [README](../README.md#uso).

## Teclado

Enter ejecuta el comando. Puedes editar con las flechas, Home, End, Backspace y
Delete. `Ctrl+A` selecciona toda la entrada y `Ctrl+U` la borra. Usa la rueda o
PageUp/PageDown para desplazarte por la lista.

La entrada admite texto UTF-8. El portapapeles, IME, la composición de teclas
muertas y la selección parcial con el ratón todavía no están implementados.

## Sonido

La alarma usa este archivo:

```text
/usr/share/sounds/freedesktop/stereo/complete.oga
```

Pertenece a `sound-theme-freedesktop`. Se usa esa ruta, independientemente del
tema de sonidos seleccionado en el escritorio. Para probarla, escribe `sonido`.

Si falta el archivo, una biblioteca de audio o la conexión al servidor de sonido,
la aplicación muestra el error y mantiene los temporizadores funcionando.

## Problemas conocidos

**La aplicación no abre fuera de Wayland.** Iníciala desde una sesión Wayland
activa. Las sesiones X11 no están soportadas.

**Aparece `XKB-822` o `xkbcommon rechazó el keymap`.** La versión 0.1.2 corrige un
fallo de lectura del mapa de teclado presente en 0.1.1. Actualiza primero. Si
persiste, abre un [issue](https://github.com/wnunezc/timers-control-rust/issues)
con la salida completa, versión de la aplicación, distribución, compositor y
versión de xkbcommon.

**No se oye la alarma.** Prueba `sonido` y comprueba el archivo indicado, el volumen
y la disponibilidad de PulseAudio o PipeWire-Pulse.

La prueba de referencia usa glibc 2.41 y Sway 1.12. No se ha establecido una
versión mínima de glibc para el paquete completo; las bibliotecas de cada
distribución también tienen sus propios requisitos.
