# Timer-Rust

Temporizadores de cuenta regresiva con GUI nativa, entrada de comandos y sonido
del sistema. Implementaciones para **Windows 11 x64** y **Linux Wayland** (probada
localmente en Void/glibc/Sway), escritas en Rust con `std` y FFI a bibliotecas del
sistema, sin crates externos.

## Créditos

Todos los créditos conceptuales de este proyecto corresponden a
[elhacker1000](https://gitlab.com/elhacker1000).

La implementación del código en Rust corresponde a
[Walter Nuñez (wnunezc)](https://github.com/wnunezc).

## Estado

La validación manual cubre creación, pausa, continuación, detención, vencimiento,
reconfiguración y rechazo de continuación de un finalizado sin repetir su alarma.
La **0.1.1 Alpha** añade soporte básico de Linux x86_64 con glibc y Wayland,
validado en Void/Sway, incluido el uso manual dentro de WSLg. Conserva el backend
Windows x64. La **0.1.2 Alpha** corrige la lectura del mapa de teclado Wayland que
podía impedir el inicio con `XKB-822`. Ver [notas de versión](docs/releases/0.1.2-alpha.md) y
[compilación, ejecución y límites Linux](docs/LINUX.md).

Los distribuibles incluyen un EXE Windows, un paquete Linux y checksums SHA-256.
Están disponibles en [las versiones de GitHub](https://github.com/wnunezc/timers-control-rust/releases).

## Compilación y ejecución

Requiere Rust, Cargo y un linker/SDK compatibles con el destino Windows MSVC.
El entorno de la primera prueba usa Rust/Cargo 1.98.1 y `x86_64-pc-windows-msvc`.
La edición del código es Rust 2024. No se ha establecido una versión mínima de Rust.

Desde PowerShell en la carpeta del proyecto:

```powershell
cargo build
& .\target\debug\timer-rust.exe
```

El binario contiene código máquina; no necesita Cargo ni un intérprete para
ejecutarse. Sí utiliza DLL de Windows; «sin dependencias» significa aquí sin crates
externos, no independencia de las bibliotecas del sistema.

El perfil `release` usa optimización por tamaño, LTO, una unidad de compilación,
eliminación de símbolos y abort en panic. `.cargo/config.toml` incluye estáticamente
el runtime C para `x86_64-pc-windows-msvc`, evitando requerir VCRUNTIME140.dll.
Las DLL del sistema Windows siguen siendo necesarias. Compilar el distribuible:

```powershell
cargo build --release --locked --target x86_64-pc-windows-msvc
```

El resultado está en `target/x86_64-pc-windows-msvc/release/timer-rust.exe`.
La primera Alpha tiene pruebas unitarias optimizadas e inspección de dependencias;
la ejecución gráfica manual de este binario concreto sigue pendiente.

En Linux se compila con `cargo build --release --locked` y se ejecuta
`./target/release/timer-rust` desde una sesión Wayland. Requiere las bibliotecas
nativas y herramientas indicadas en [docs/LINUX.md](docs/LINUX.md); no usa XWayland.

## Comandos

Escribir en el campo inferior y pulsar Enter. Los nombres no contienen espacios,
deben ser únicos y distinguen mayúsculas. Los comandos también distinguen mayúsculas.
La entrada gráfica admite hasta 1024 unidades UTF-16. No se ejecutan comandos del SO.

| Comando | Comportamiento |
| --- | --- |
| `crear prueba 01:30` | Crea e inicia una cuenta de 90 segundos. |
| `configurar prueba 5m` | Reemplaza su duración e inicia desde cinco minutos. |
| `pausar prueba` | Congela el tiempo restante de una cuenta en marcha. |
| `continuar prueba` | Reanuda una cuenta pausada o detenida. |
| `parar prueba` | Detiene y conserva el tiempo restante y la fila. |
| `sonido` | Prueba el sonido del sistema. |
| `ayuda` | Muestra los comandos en el área de estado. |

Al vencer, la cuenta pasa a `finalizado` y queda en la lista. `continuar` no revive
una cuenta finalizada; puede volver a iniciarse con `configurar`. Ejecutar `parar`
sobre un finalizado conserva ese estado y no vuelve a habilitar la alarma.

Un comando incorrecto muestra el error y conserva el texto escrito. Una duración
inválida no reemplaza la configuración de un temporizador existente. No hay
persistencia: cerrar el proceso descarta las cuentas.

## Duraciones

| Formato | Ejemplo | Duración |
| --- | --- | --- |
| `MM:SS` | `01:30` | 90 segundos |
| `HH:MM:SS` | `01:02:03` | 1 hora, 2 minutos y 3 segundos |
| `MMSS` | `0130` | 90 segundos |
| `HHMMSS` | `010203` | 1 hora, 2 minutos y 3 segundos |
| `xh` | `2h` | 2 horas |
| `xm` | `90m` | 90 minutos |
| `xs` | `30s` | 30 segundos |

Los campos de reloj tienen exactamente dos dígitos. Minutos y segundos van de
`00` a `59`; para más minutos se usa el sufijo, por ejemplo `90m`. Las cantidades
con unidad son enteros positivos ASCII. Se rechazan cero, negativos, signos,
decimales, unidades desconocidas y desbordamientos. Un plazo que `Instant` no pueda
representar también se rechaza. La visualización redondea hacia arriba a segundos.

Esta versión implementa duraciones de cuenta regresiva. No incorpora recurrencias,
secuencias de trabajo/descanso ni horarios de inicio/fin; esos comportamientos no
han sido definidos por el usuario.

## Interfaz, audio y consumo

- Lista vertical arriba a la izquierda, estado y entrada de comandos abajo.
- Fuente Consolas en Windows y monospace en Linux, de altura lógica fija de 16 unidades, sin adaptarse al tamaño de
  la ventana. Windows puede sustituirla si falta o escalarla según la configuración
  de pantalla; no se ha implementado un contrato de DPI por monitor.
- Controles Win32 `LISTBOX`, `STATIC` y `EDIT`; Linux dibuja con FreeType y píxeles Rust sobre Wayland,
  sin GTK ni webview. El backend gráfico compilado se selecciona por plataforma.
- En Windows `GetMessageW` bloquea el hilo cuando no hay mensajes. Sin cuentas activas se
  cancela el timer de Windows. Minimizado, se espera al siguiente vencimiento.
- Los plazos se calculan con `Instant`; retrasar un mensaje no alarga una cuenta.
  No hay garantía de tiempo real ni se ha verificado suspensión/hibernación.
- El código no crea hilos, funciones async ni runtime asíncrono. Windows y sus
  bibliotecas pueden crear hilos internos: no se afirma que el proceso tenga uno solo.
- En Windows, el alias `SystemExclamation` se reproduce con `PlaySoundW` síncrono. **La GUI
  queda bloqueada durante el sonido.** La reproducción depende del esquema de sonido,
  dispositivo y volumen configurados por el usuario; no hay fallback intencional.
- Linux reproduce `/usr/share/sounds/freedesktop/stereo/complete.oga` del tema
  del sistema mediante libsndfile y PulseAudio/PipeWire-Pulse,
  también síncrono. La GUI espera a un ayudante temporal del mismo ejecutable; las bibliotecas
  de audio se cargan allí solo mientras suena, con un hilo GUI antes/después.
- Si se detectan varios vencimientos en una misma actualización, la implementación
  actual emite un sonido y lista sus nombres. La política definitiva de concurrencia
  de alarmas sigue pendiente de definición.
- Linux lee ese archivo del sistema por bloques; faltan rutas de audio elegibles
  por comando/configuración y extracción de segmentos. No hay file-dialogs.

En Linux, liberar buffers de tamaños anteriores recuperó unos 29,1 MiB de RSS
en la prueba documentada en [docs/LINUX.md](docs/LINUX.md). Son muestras locales;
no se han establecido cifras de consumo representativas entre equipos.
La revisión conserva estructuras simples y evita asignar un vector de tokens por
comando. Windows todavía construye sus cadenas de filas en cada actualización visible.
Linux conserva las cadenas y solo las reescribe al cambiar segundos o estados.

Consultar la [revisión del consumo Linux](docs/MEMORY-REVIEW.md) para los costes
de bibliotecas, buffers y asignaciones corregidos y sus mediciones antes/después.

## Arquitectura y revisión

Consultar [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) para responsabilidades,
aplicación de POO/DRY/SOLID, recursos nativos y decisiones de diseño.
Consultar [docs/TESTING.md](docs/TESTING.md) para pruebas y pendientes.
El código tiene documentación de módulos, contratos de métodos y
justificaciones de los bloques `unsafe` del adaptador Win32.
El adaptador Linux tiene sus propios contratos de memoria, callbacks y bibliotecas
en [docs/LINUX.md](docs/LINUX.md), manteniendo el dominio compartido.

## Líneas y peso local

Medición del 2026-10-16 después de la revisión, sin abrir la aplicación:

| Módulo | Líneas físicas |
| --- | ---: |
| `src/windows.rs` y `src/windows/native.rs` | 600 |
| `src/app.rs`, `timers.rs`, `duration.rs`, `command.rs`: núcleo compartido | 619 |
| `src/linux.rs` y `src/linux/`: Wayland, ABI, dibujo, entrada y audio | 2307 |
| `src/main.rs`: entrada | 43 |
| `build.rs`: metadatos del protocolo en compilación | 80 |
| **Total Rust** | **3649** |

De ese total, 393 líneas pertenecen a módulos de pruebas, con sus comentarios y
espacios: no se incluyen en el ejecutable normal. El total contiene 224 líneas
completas de comentarios/documentación y 152 vacías; estas categorías se superponen
con las pruebas y no deben restarse dos veces. Las otras 3273 son líneas de sintaxis
Rust, incluidas llaves, atributos y declaraciones; no equivalen a instrucciones de CPU.
Los archivos Rust ocupan en conjunto 126 303 bytes  aproximadamente 123,3 KiB.

GUI y declaraciones Win32 suman 600 líneas; el nuevo backend Linux añade listeners,
gestión explícita de teclado y buffers que Win32 ofrece como controles preparados.
El acceso directo al sistema exige
declarar manualmente estructuras, firmas y constantes que un crate normalmente
proporcionaría. La documentación, el formato y las pruebas aumentan las líneas sin
implicar por sí mismos mayor consumo de la aplicación.

| Artefacto | Tamaño medido |
| --- | ---: |
| Primera versión debug probada manualmente | 257 536 bytes = 251,5 KiB |
| Versión refactorizada debug, `target/review/debug/timer-rust.exe` | 249 856 bytes = 244 KiB |
| Referencia release anterior a la revisión | 130 560 bytes = 127,5 KiB |
| Release 0.1.0 Alpha con runtime C estático | 227 840 bytes = 222,5 KiB |
| Backend Linux anterior a la corrección de RAM | 345 184 bytes = 337,1 KiB |
| Backend Linux corregido, bibliotecas compartidas | 402 528 bytes = 393,1 KiB |
| Windows debug recompilado tras añadir selección de plataforma | 349 184 bytes = 341 KiB |
| Release 0.1.1 Alpha Windows x64 | 228 352 bytes = 223 KiB |
| Paquete 0.1.1 Alpha Linux x86_64 tar.gz, con guía y avisos | 200 766 bytes = 196,1 KiB |

Los tamaños son mediciones de builds distintos, no un presupuesto de RAM. El
release Alpha Windows ocupa aproximadamente 228 KB y el Linux corregido unos 403 KB;
la diferencia con el release histórico se debe también a incluir el runtime C.
El tamaño puede cambiar con el compilador y las opciones de enlace. Las carpetas de
build, PDB, herramientas y documentación generada no forman parte del peso del exe.

La RAM es otra medida: una lectura previa aislada de debug, en condiciones no controladas,
registró 20 893 696 bytes de working set (19,9 MiB) y 2 985 984 bytes privados
(2,85 MiB). No se suman: representan conceptos distintos. Incluyen costes del proceso
y bibliotecas nativas, no solo su código Rust. Esta muestra no permite asegurar el
consumo esperado en reposo, con audio o con más cuentas; falta un benchmark reproducible.
Tampoco permite deducir el consumo a partir de la cantidad de líneas o del tamaño del exe.
En Linux, con una cuenta pausada y las mismas dimensiones, la corrección redujo
RSS de 12 924 a 7008 KiB en 660×420 y de 18 764 a 9920 KiB en 1280×800. Tras tres alarmas
otra ejecución conservó 7008 KiB de RSS antes/después; el audio usa un ayudante
temporal cuyo coste también se midió. El informe [MEMORY-REVIEW.md](docs/MEMORY-REVIEW.md)
distingue bibliotecas, buffers, PSS/RSS y memoria del ayudante. No equipara esos
contadores con los 2,4 MB mostrados en la captura Windows.

## Compatibilidad pendiente

Para preparar el desarrollo nativo de Linux existe un
[laboratorio Void Linux con glibc y Sway/Wayland](docs/LINUX-LAB.md), con pantalla
virtual aislada y las 12 pruebas del núcleo ejecutadas correctamente en Linux.
La GUI Wayland está implementada y comprobada en ese laboratorio.

Hay adaptadores Windows y Linux Wayland; faltan otros sistemas y la validación
Linux en el equipo físico del destinatario. Windows XP de 32 bits sigue pendiente;
compilar para x86 por sí solo no acredita XP. El código de dominio está separado
para reutilizarlo al implementar otros adaptadores. Las plataformas distintas de
Windows y Linux todavía no tienen GUI.

El soporte oficial actual de Rust no incluye XP. La ruta de toolchain, biblioteca
estándar, linker, CPU, imports y pruebas reales en XP requiere una investigación
específica antes de prometer compatibilidad:
[plataformas de Rust](https://doc.rust-lang.org/rustc/platform-support.html).
