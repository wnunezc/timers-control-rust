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
xbps-install -S gcc pkg-config rust cargo wayland-devel wayland-protocols freetype-devel fontconfig-devel libxkbcommon-devel libpulseaudio libsndfile sound-theme-freedesktop dejavu-fonts-ttf
```

Como usuario normal, desde el proyecto:

```sh
cargo build --release --locked
./target/release/timer-rust
```

La GUI enlaza libwayland-client, FreeType, Fontconfig, xkbcommon, libc y libgcc_s.
No enlaza Cairo, X11 ni audio al arrancar. Las dependencias transitivas de las
bibliotecas de fuentes dependen de la distribución. Fontconfig localiza la fuente
monospace una vez y libera su catálogo; FreeType mantiene la fuente fija a 16 px.
El audio carga libsndfile y libpulse-simple/libpulse bajo demanda en un ayudante
temporal del mismo ELF. No se necesita instalar un reproductor externo. Si faltan
sus bibliotecas, la GUI abre y muestra el error cuando se solicita sonido.

build.rs invoca wayland-scanner private-code sobre el XML estable xdg-shell y
compila sus tablas ABI. También contrasta offsets públicos de FreeType contra
los headers instalados; no hay lógica GUI propia escrita en C. El XML se toma de
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
shm v1 y xdg-shell v1. Los píxeles se dibujan en Rust sobre memfd/mmap. Se usa
RGB565 si el servidor lo anuncia y ARGB32 como respaldo. Hasta dos buffers y
32 MiB por buffer; no se escribe un buffer ocupado. Los tamaños anteriores se
liberan tras release. Cada buffer recuerda revisiones de lista/pie y solo repinta
la región que cambió. No se vuelven a crear CString para dibujar texto.

RowCache reutiliza los String de las filas y solo actualiza segundos/estados
cambiados. El adaptador consulta esa caché al vencer el siguiente segundo o tras
un comando que modifica cuentas. La entrada se conserva en errores sin clonarla;
los eventos UTF-8 usan un array de 64 bytes. FreeType mantiene una caché acotada de
256 glifos y 128 KiB de bitmaps, sin copiar textos completos.

Los frame callbacks limitan commits. Si una superficie deja de recibirlos se
puede esperar al vencimiento. El audio es síncrono: la GUI espera al ayudante
interno sin inicializar otra ventana. El proceso temporal acota las bibliotecas,
hilos internos y cachés de audio al tiempo de reproducción. Es un coste temporal
medido, no un reproductor permanente. No hay hilos propios ni runtime async.

## Validación local: 2026-10-16

- 20 pruebas Linux aprobadas: 12 compartidas y 8 de caché, entrada, fuente,
  píxeles/regiones y bibliotecas opcionales.
- Prueba manual de la GUI Linux en Sway/WSLg confirmada por el usuario.
- 12 pruebas Windows aprobadas; compilación Windows correcta.
- Formato, Clippy y Rustdoc privados con advertencias como errores correctos.
- app_id timer-rust y shell xdg_shell comprobados en Sway sin XWayland.
- Crear, pausar, continuar, parar, configurar, vencer y rechazar reactivar finalizado.
- Nombre café, rechazo de 00:60, entrada conservada y corrección con Ctrl+A.
- Lista desbordada y PageDown; resize 2560×1600 → 660×420 con fuente constante.
- Sin compositor, stderr y código 1; sin salida de audio, error visible sin cerrar GUI.

La corrección de memoria se verificó sin capturas ni entrada en el escritorio de
Windows. A 660×420, RSS pasó de 12 924 a 7008 KiB; a 1280×800, de 18 764 a 9920 KiB,
con la misma cuenta pausada. Son muestras del laboratorio, incluyendo páginas
compartidas. El ayudante se capturó en una salida PulseAudio nula: tres alarmas no
cambiaron el RSS de la GUI (7008 KiB antes/después, un hilo). Pico de PSS agregado
GUI+ayudante 4629 KiB. RDPSink siguió siendo la salida predeterminada.

El ELF corregido mide 402 528 bytes (393,1 KiB), sin bibliotecas compartidas. El tamaño
anterior era 345 184 bytes; el incremento de archivo es 56 KiB. En cinco segundos de
reposo con cuenta pausada no se incrementaron ticks de CPU. Ver desglose, medidas y
pruebas en [MEMORY-REVIEW.md](MEMORY-REVIEW.md).

Pendientes: Void/Sway físico del destinatario, audio físico, DPI, IME/portapapeles
si se requieren. La nueva rasterización aún requiere confirmación manual del usuario; la prueba
manual anterior fue con el renderizador previo.

## Paquete local

El distribuible es `timer-rust-v0.1.1-alpha-linux-x86_64.tar.gz`, preparado en
`target/dist/v0.1.1-alpha/`, con el ELF ejecutable, guía de uso, avisos xdg-shell
y SHA256SUMS.txt interno. El listado externo SHA256SUMS.txt verifica el tar y el
EXE Windows. Se comprueban checksums y permisos Unix 0755/0644 al empaquetar.
Preparar estos archivos no publica una versión en GitHub.

El ELF inspeccionado importa símbolos glibc hasta GLIBC_2.34; eso no garantiza
ejecución sobre glibc 2.34, pues sus bibliotecas compartidas tienen sus propios
requisitos. La plataforma validada es la descrita al principio de esta guía.
