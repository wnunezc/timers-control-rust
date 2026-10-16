# GUI Linux nativa

## Alcance

Backend implementado y probado localmente en Void Linux x86_64, glibc 2.41 y Sway
1.12. Se conecta directamente a Wayland/xdg-shell, sin Wine, XWayland, GTK, webview
ni crates externos. Comparte TimerApp, comandos, duraciones y estado con Windows.
La versión 0.1.1 Alpha incorpora este soporte básico. El usuario confirmó la
prueba manual en Sway anidado en WSLg; la instalación física Void/Sway sigue
pendiente. La publicación anterior 0.1.0 Alpha contiene solo el EXE Windows.

El audio Linux reproduce el archivo completo
`/usr/share/sounds/freedesktop/stereo/complete.oga` del paquete
`sound-theme-freedesktop`. Es una ruta fija explícita para esta prueba; no consulta
el tema seleccionado por un escritorio. libsndfile decodifica por bloques de hasta
16 KiB de PCM y PulseAudio/PipeWire-Pulse lo reproduce síncronamente. El archivo
instalado dura aproximadamente 1,089 s, estéreo a 44 100 Hz. No se sintetiza un tono
si falta el archivo: el fallo aparece en el estado de la GUI. No implementa aún
rutas elegibles por comando/configuración, extracción, persistencia o recurrencias.

## Compilación en Void

Se necesitan Rust/Cargo (edición 2024), cc/ar para los metadatos del protocolo,
pkg-config, headers y bibliotecas nativas. Como administrador:

```sh
xbps-install -S gcc pkg-config rust cargo wayland-devel wayland-protocols cairo-devel libxkbcommon-devel pulseaudio-devel libsndfile-devel sound-theme-freedesktop dejavu-fonts-ttf
```

Como usuario normal, desde el proyecto:

```sh
cargo build --release --locked
./target/release/timer-rust
```

Las bibliotecas directas son libwayland-client, Cairo, xkbcommon, libpulse-simple,
libpulse, libsndfile, libc y libgcc_s. Sus dependencias transitivas dependen del paquete de
la distribución. Cairo de Void también enlaza bibliotecas X11: la aplicación no
abre una conexión X11 y su ventana se comprobó como xdg_shell sin XWayland.

build.rs invoca wayland-scanner private-code sobre el XML estable xdg-shell y
compila sus tablas ABI. No hay lógica GUI propia escrita en C. El XML se toma de
/usr/share/wayland-protocols/stable/xdg-shell/xdg-shell.xml; XDG_SHELL_XML permite
otra ruta. CC y AR eligen herramientas, no configuran una toolchain completa de
compilación cruzada. El backend exige ABI Linux de 64 bits; no se declara soporte
x86 o musl. Mantener los avisos de origen del protocolo en los metadatos generados.

## Laboratorio aislado desde PowerShell

```powershell
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-start
wsl -d Void-Dev --exec /bin/sh -lc 'CARGO_TARGET_DIR=/home/dev/projects/timer-rust-target cargo build --release --locked --manifest-path /mnt/d/OpsZone/DevWorkspace/Projects/Desktop/Timer-Rust/Cargo.toml'
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-run swaymsg exec /home/dev/projects/timer-rust-target/release/timer-rust
```

La ventana aparece en la pantalla virtual de Sway. Capturas con lab-run grim;
detención con lab-stop. Ver [LINUX-LAB.md](LINUX-LAB.md). Ejecutarlo directamente
en una sesión WSLg ordinaria sí puede mostrar una ventana en Windows.

## Interfaz y límites

Lista superior izquierda, estado y entrada abajo, fuente monospace de 16 unidades
constante al redimensionar y mínimo 240×150. Misma gramática y transiciones Windows.
Enter ejecuta; un error de comando conserva su entrada. Flechas, Home/End,
Backspace/Delete, Ctrl+A y Ctrl+U editan sin romper UTF-8. Entrada limitada a 1024
unidades UTF-16. Rueda y PageUp/PageDown desplazan filas. No hay parpadeo del caret
que despierte periódicamente al proceso inactivo.

No hay todavía portapapeles, selección parcial con ratón, IME, composición de teclas
muertas, navegación por Tab ni decoración del lado del cliente. La barra y controles
de ventana los decide el compositor. Se probó escala 1, no DPI por monitor.

## Eventos y propiedad

Solo el hilo GUI lee el display. Antes de poll se despachan eventos pendientes y
vacían solicitudes; si hay backpressure se espera también POLLOUT. Sin cuentas ni
repetición de tecla el timeout es infinito. Los callbacks encolan entrada o registran
estado; comandos y audio se ejecutan fuera, sin dispatch recursivo. State y cada
Buffer tienen dirección estable en Box hasta destruir sus listeners.

Versiones negociadas acotadas: compositor/surface v3, seat/keyboard/pointer v5,
shm v1 y xdg-shell v1. Así no llegan eventos nuevos sin listener. El dibujo usa
Cairo sobre memfd/mmap ARGB8888, hasta dos buffers reutilizables y 32 MiB máximos
por buffer. Fuera de ese límite hay diagnóstico y cierre. No se reescribe memoria
hasta wl_buffer.release. Pools destruidos tras crear buffers; imágenes de tamaños
anteriores se liberan cuando quedan libres. Drop elimina Cairo, proxies, mappings,
XKB y conexión en orden, sin despachar callbacks durante la destrucción.

Los frame callbacks limitan commits. Si una superficie oculta deja de recibirlos,
se puede esperar al vencimiento en vez de repintar cada segundo. Falta probar todos
los casos de ocultación y suspensión. Audio síncrono bloquea hasta drenar y libera
su conexión; libpulse puede crear un hilo auxiliar. Se observó un hilo inactivo,
dos durante audio y regreso a uno. No hay hilos propios ni runtime async.

## Validación local: 2026-10-16

- 14 pruebas Linux aprobadas: 12 compartidas y 2 de edición UTF-8/límites.
- Prueba manual de la GUI Linux en Sway/WSLg confirmada por el usuario.
- 12 pruebas Windows aprobadas; compilación Windows correcta.
- Formato, Clippy y Rustdoc privados con advertencias como errores correctos.
- app_id timer-rust y shell xdg_shell comprobados en Sway sin XWayland.
- Crear, pausar, continuar, parar, configurar, vencer y rechazar reactivar finalizado.
- Nombre café, rechazo de 00:60, entrada conservada y corrección con Ctrl+A.
- Lista desbordada y PageDown; resize 2560×1600 → 660×420 con fuente constante.
- Sin compositor, stderr y código 1; sin salida de audio, error visible sin cerrar GUI.

Se añadió una salida PulseAudio nula temporal, seleccionada solo para el proceso
mediante PULSE_SINK. Su monitor registró PCM no nulo del archivo del tema, sin enviarlo a los
altavoces. Se verificaron el comando sonido y un vencimiento, ambos con
91 851 muestras no nulas y pico PCM de 12 843. Después del sonido se observaron
20 120 KiB de RSS a 1280×800 y un hilo. RDPSink siguió predeterminado. El módulo se retira al terminar; esta
comprobación no sustituye escuchar audio en el equipo real del destinatario.

Las cifras de RSS y resize siguientes se midieron antes de sustituir el tono
generado por el archivo del tema; describen el renderer y esa ejecución.

Binario optimizado (recompilado con audio del tema): 345 184 bytes (337,1 KiB); no incluye bibliotecas compartidas.
En 660×420 se observaron 12 644 KiB de RSS y un hilo. En otro muestreo inactivo de
cinco segundos no aumentaron los ticks de CPU; no es garantía de CPU cero universal.
Al reducir 2560×1600 a 660×420, RSS pasó de 42 476 a 12 644 KiB, liberando unos
29,1 MiB. Los mappings memfd confirmaron la retirada de imágenes grandes. Son
mediciones puntuales de una ejecución, no un benchmark entre equipos ni memoria
exclusiva atribuible toda a Rust.

Pendientes: Void/Sway físico del destinatario, audio físico, DPI, IME/portapapeles
si se requieren. La publicación 0.1.1 Alpha se prepara separadamente.

## Paquete local

El distribuible es `timer-rust-v0.1.1-alpha-linux-x86_64.tar.gz`, preparado en
`target/dist/v0.1.1-alpha/`, con el ELF ejecutable, guía de uso, avisos xdg-shell
y SHA256SUMS.txt interno. El listado externo SHA256SUMS.txt verifica el tar y el
EXE Windows. Se comprueban checksums y permisos Unix 0755/0644 al empaquetar.
Preparar estos archivos no publica una versión en GitHub.

El ELF inspeccionado importa símbolos glibc hasta GLIBC_2.34; eso no garantiza
ejecución sobre glibc 2.34, pues sus bibliotecas compartidas tienen sus propios
requisitos. La plataforma validada es la descrita al principio de esta guía.
