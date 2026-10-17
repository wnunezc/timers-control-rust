# Arquitectura

El dominio no depende de Win32 ni de Wayland. Los adaptadores convierten eventos
del sistema en comandos y presentan el resultado. Los módulos compartidos
prohíben `unsafe`; las declaraciones FFI y los recursos nativos quedan en cada
plataforma.

## Módulos

| Archivo | Responsabilidad |
| --- | --- |
| `src/main.rs` | Selección del adaptador y diagnóstico de arranque |
| `src/duration.rs` | Conversión de duraciones con aritmética comprobada |
| `src/command.rs` | Gramática y argumentos de los comandos |
| `src/timers.rs` | Estado y transiciones de una cuenta |
| `src/app.rs` | Colección, casos de uso, filas y próximo despertar |
| `src/windows.rs` | Ventana, controles, audio y eventos Win32 |
| `src/windows/native.rs` | Declaraciones ABI de Windows |
| `src/linux.rs` | Conexión Wayland, bucle y presentación |
| `src/linux/events.rs` | Listeners y conversión de entrada con xkbcommon |
| `src/linux/entry.rs` | Edición UTF-8 y límites de entrada |
| `src/linux/render.rs` | Buffers y dibujo por regiones |
| `src/linux/font.rs` | Fuente fija y caché de glifos |
| `src/linux/audio.rs` | Lectura del sonido y reproducción en el proceso temporal |
| `src/linux/library.rs` | Propiedad de bibliotecas cargadas con dlopen/dlsym |
| `src/linux/native.rs` | Declaraciones ABI y opcodes Wayland |
| `build.rs` | Metadatos xdg-shell y comprobaciones ABI de FreeType |

La entrada sigue este recorrido:

```text
texto → Command::parse → TimerApp::execute → Outcome → ventana / audio
```

`Outcome` contiene un mensaje y la solicitud de sonido, sin ejecutar efectos del
sistema. Esto permite probar el dominio sin una ventana o un dispositivo de audio.

## Decisiones de diseño

`Timer` y `TimerApp` encapsulan estado y comportamiento mediante `struct` e `impl`.
Se usa composición, sin simular una jerarquía de clases. El parser, la gramática y
las transiciones se comparten entre plataformas; el formato de las filas también
tiene una sola implementación.

La separación de responsabilidades y de efectos mantiene las dependencias de la
plataforma fuera del dominio. Es la aplicación práctica de POO, DRY y SOLID en este
proyecto: no se incorporan interfaces de plugins, contenedores de inyección ni
traits con una única implementación para completar una lista de patrones.

## Estado y tiempo

Los nombres son únicos, no vacíos y no contienen espacios ni caracteres de
control. Las duraciones son positivas y el plazo debe ser representable con
`Instant`. Reconfigurar valida el reemplazo antes de modificar la cuenta.

| Estado | Dato conservado | Operaciones |
| --- | --- | --- |
| En marcha | Instante de vencimiento | Pausar, parar, finalizar, configurar |
| Pausado | Duración restante | Continuar, parar, configurar |
| Detenido | Duración restante | Continuar, parar, configurar |
| Finalizado | Sin plazo activo | Configurar; parar conserva finalizado |

El tiempo restante se calcula desde el plazo, no descontando eventos recibidos.
Todas las filas de una actualización usan el mismo instante. El planificador
busca el próximo cambio de segundo visible o el próximo vencimiento cuando la
ventana está minimizada. Sin cuentas activas devuelve `None`.

La colección es un vector. Buscar, dibujar y planificar cuestan O(n); todavía no
hay mediciones de escalabilidad con listas grandes.

## Windows

La interfaz utiliza `LISTBOX`, `STATIC` y `EDIT`, una fuente propia y un único
`SetTimer` para todas las cuentas. `GetMessageW` bloquea mientras no hay mensajes;
sin cuentas activas se cancela el timer. Los plazos se redondean hacia arriba a
milisegundos y se ajustan a los límites de `SetTimer`, recalculando si hace falta.
No se cambia la resolución global del reloj.

Las estructuras FFI usan `repr(C)`, anchos nativos para punteros/WPARAM y `i32`
para LONG. Las funciones usan `extern "system"`; los textos son UTF-16 terminados
en cero. LISTBOX copia las cadenas durante llamadas síncronas.

Los callbacks no prestan referencias mutables al dominio durante la entrega
reentrante de mensajes. `Ui` posee ventana y fuente; su `Drop` destruye la ventana
y los controles antes de liberar la fuente. El cursor y la brocha del sistema son
compartidos y no se destruyen. `WM_CLOSE` termina el bucle; el propietario libera
la ventana. La clase se registra una vez durante el proceso.

Se comprueban fallos de inicialización, `GetMessageW`, `SetTimer` y carga de filas.
El redibujado se reactiva aun si falla la reserva del LISTBOX. No se ofrece
recuperación de OOM. El audio usa `SND_ALIAS | SND_NODEFAULT | SND_SYNC` y bloquea
el hilo de la ventana hasta terminar.

## Linux

Solo el hilo GUI lee el display. Antes de `poll` se despachan eventos pendientes
y se vacían solicitudes; ante backpressure se espera también `POLLOUT`. Sin
cuentas activas ni repetición de tecla, el timeout es infinito. Los callbacks
encolan entrada o registran estado; los comandos y el audio se ejecutan fuera.

`State` y los buffers tienen direcciones estables en `Box` mientras existen sus
listeners. Las versiones negociadas son compositor/surface v3,
seat/keyboard/pointer v5, shm v1 y xdg-shell v1. La GUI exige ABI Linux de 64 bits.

El keymap se lee con `read_exact_at` desde cero, sin depender del offset compartido
ni modificarlo. Se comprueban tamaño y terminador NUL antes de llamar a xkbcommon.

Los píxeles se guardan en memfd/mmap. Se negocia RGB565 y se conserva ARGB32 como
respaldo, con un máximo de dos buffers y 32 MiB por buffer. No se modifica una
imagen prestada al compositor; los buffers antiguos se liberan tras `release`.
Cada buffer registra las revisiones de lista y pie para repintar solo lo cambiado.
Los frame callbacks limitan los commits de superficie.

`RowCache` conserva las cadenas y actualiza solo segundos/estados modificados.
La entrada no se clona al ejecutar y los eventos de texto usan un array de 64 bytes.
Fontconfig localiza monospace y libera el catálogo; FreeType conserva la fuente
fija a 16 px y una caché limitada a 256 glifos y 128 KiB de bitmaps. `build.rs`
contrasta los prefijos ABI usados por Rust con los headers de FreeType.

El sonido se lee por bloques PCM de 16 KiB en un proceso temporal del mismo ELF,
iniciado mediante `/proc/self/exe` sin shell. Esa ruta conserva la versión del
binario aunque se reemplace el archivo mientras la ventana siga abierta. El modo
interno no inicializa otra GUI. Carga sndfile/PulseAudio bajo demanda y termina al
acabar; la ventana espera y recibe el error si falla. libpulse puede crear hilos
internos en ese proceso. Véase el análisis de [memoria](MEMORY.md).
