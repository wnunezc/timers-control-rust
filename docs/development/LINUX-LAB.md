# Laboratorio Linux

La integración Linux se ha probado con Void Linux x86_64/glibc en WSL 2 y Sway.
Se usan dos sesiones separadas: una salida headless para automatización y una
sesión Wayland anidada en WSLg para la prueba manual.

## Entorno de referencia

| Componente | Versión o configuración |
| --- | --- |
| Distribución | Void Linux x86_64, glibc 2.41 |
| Rust/Cargo | 1.99.0 |
| Compositor | Sway 1.12, sin XWayland |
| Renderizador | pixman, por software |
| Wayland | 1.26.0 |
| xkbcommon | 1.13.2 |
| Herramientas | wayland-info, wtype, grim, jq |
| Audio | Servidor PulseAudio de WSLg; salida nula para capturas de PCM |

Las herramientas de prueba no son dependencias de Timer-Rust. Los scripts de
arranque usados en ese entorno son locales y no forman parte del repositorio.

## Sesión headless

La sesión debe tener un `XDG_RUNTIME_DIR` propio, con permisos 0700, y sockets
Wayland/Sway distintos de la sesión de uso diario. Se inicia Sway con backend
headless y pixman, sin heredar `DISPLAY` ni `WAYLAND_SOCKET` de otra sesión.

La configuración de prueba desactiva XWayland, barras y lanzadores. Se utiliza
una salida a escala 1; el tamaño de referencia para integración es 1280×800.
Para las mediciones de memoria se emplean además las dimensiones indicadas en
[MEMORY.md](MEMORY.md).

Antes de enviar entrada con wtype, comprobar el árbol de Sway y enfocar la ventana
con `app_id` igual a `timer-rust`. La automatización debe usar exclusivamente los
sockets de esa sesión. La espera inicial de 250 ms usada en las pruebas evita
perder el primer carácter al establecer el foco del teclado virtual.

Al terminar, cerrar el cliente antes de detener Sway. Cerrar primero el compositor
provoca el diagnóstico esperado `El compositor cerró la conexión` en el cliente.

## Audio

Para comprobar el audio sin reproducirlo por los altavoces se usa una salida nula
de PulseAudio y se captura PCM. Se selecciona esa salida solo para el proceso de
prueba; se conserva la salida predeterminada y se retira el módulo temporal al
terminar. Esta comprobación verifica reproducción y datos, no el hardware físico.

## Sesión manual

La segunda sesión usa Sway anidado en WSLg, con backend Wayland y pixman. Abre una
ventana visible para interactuar con teclado y ratón. Se mantiene separada de la
sesión automatizada para no mezclar el foco ni los eventos de entrada.

## Límites

WSL y el renderizado por software permiten comprobar protocolo, entrada, dibujo
y errores de conexión. No verifican DRM, GPU, entrada física, suspensión ni el
audio de una instalación Linux independiente. Las pruebas en el equipo de destino
siguen siendo necesarias.
