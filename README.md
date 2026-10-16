# Timer-Rust

Temporizadores de cuenta regresiva con GUI nativa, entrada de comandos y sonido
del sistema. Implementación inicial para **Windows 11 de 64 bits**, escrita en
Rust con `std` y FFI a bibliotecas del sistema, sin crates externos.

La validación manual cubre creación, pausa, continuación, detención, vencimiento,
reconfiguración y rechazo de continuación de un finalizado sin repetir su alarma.
Release y métricas reproducibles siguen pendientes.

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

Existe un perfil `release` con optimización por tamaño, LTO, una unidad de
compilación, eliminación de símbolos y abort en panic. La primera ejecución de ese
perfil no pudo validarse porque Windows indicó que el archivo estaba en uso.
No se atribuye ese bloqueo a ningún proceso sin evidencia.

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
- Fuente Consolas de altura lógica fija de 16 unidades, sin adaptarse al tamaño de
  la ventana. Windows puede sustituirla si falta o escalarla según la configuración
  de pantalla; no se ha implementado un contrato de DPI por monitor.
- Controles nativos `LISTBOX`, `STATIC` y `EDIT`; sin webview ni framework gráfico.
- `GetMessageW` bloquea el hilo cuando no hay mensajes. Sin cuentas activas se
  cancela el timer de Windows. Minimizado, se espera al siguiente vencimiento.
- Los plazos se calculan con `Instant`; retrasar un mensaje no alarga una cuenta.
  No hay garantía de tiempo real ni se ha verificado suspensión/hibernación.
- El código no crea hilos, funciones async ni runtime asíncrono. Windows y sus
  bibliotecas pueden crear hilos internos: no se afirma que el proceso tenga uno solo.
- El alias `SystemExclamation` se reproduce con `PlaySoundW` síncrono. **La GUI
  queda bloqueada durante el sonido.** La reproducción depende del esquema de sonido,
  dispositivo y volumen configurados por el usuario; no hay fallback intencional.
- Si se detectan varios vencimientos en una misma actualización, la implementación
  actual emite un sonido y lista sus nombres. La política definitiva de concurrencia
  de alarmas sigue pendiente de definición.
- No hay lecturas de archivos de audio, extracción de segmentos ni file-dialogs.

No se han demostrado ahorros de al menos 10 MB ni cifras de consumo representativas.
La revisión conserva estructuras simples y evita asignar un vector de tokens por
comando. La lista se reconstruye solo cuando cambia su texto; en cada actualización
visible aún se construyen cadenas de las filas, con coste lineal en el número de cuentas.

## Arquitectura y revisión

Consultar [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) para responsabilidades,
aplicación de POO/DRY/SOLID, recursos nativos y decisiones de diseño.
Consultar [docs/TESTING.md](docs/TESTING.md) para pruebas y pendientes.
El código tiene documentación de módulos, contratos de métodos y
justificaciones de los bloques `unsafe` del adaptador Win32.

## Líneas y peso local

Medición del 2026-10-16 después de la revisión, sin abrir la aplicación:

| Módulo | Líneas físicas |
| --- | ---: |
| `src/windows.rs`: ventana, recursos, audio y eventos | 426 |
| `src/windows/native.rs`: ABI y constantes Win32 | 174 |
| `src/app.rs`: comandos ejecutados, filas y planificación | 195 |
| `src/timers.rs`: estado de cada temporizador | 179 |
| `src/duration.rs`: formatos de duración | 116 |
| `src/command.rs`: gramática de comandos | 67 |
| `src/main.rs`: entrada | 20 |
| **Total Rust** | **1177** |

De ese total, 211 líneas pertenecen a módulos de pruebas, con sus comentarios y
espacios: no se incluyen en el ejecutable normal. El total contiene 109 líneas
completas de comentarios/documentación y 81 vacías; estas categorías se superponen
con las pruebas y no deben restarse dos veces. Las otras 987 son líneas de sintaxis
Rust, incluidas llaves, atributos y declaraciones; no equivalen a instrucciones de CPU.
Los archivos Rust ocupan en conjunto 41 566 bytes, aproximadamente 40,6 KiB.

GUI y declaraciones Win32 suman 600 líneas: el acceso directo al sistema exige
declarar manualmente estructuras, firmas y constantes que un crate normalmente
proporcionaría. La documentación, el formato y las pruebas aumentan las líneas sin
implicar por sí mismos mayor consumo de la aplicación.

| Artefacto | Tamaño medido |
| --- | ---: |
| Primera versión debug probada manualmente | 257 536 bytes = 251,5 KiB |
| Versión refactorizada debug, `target/review/debug/timer-rust.exe` | 249 856 bytes = 244 KiB |
| Referencia release anterior a la revisión | 130 560 bytes = 127,5 KiB |

Para esta versión local de desarrollo, el ejecutable esperado es por tanto de
**aproximadamente 250 KB en disco**. El release de referencia fue medido antes de
la revisión; el archivo ya no estaba presente al hacer este inventario. No se ha
recompilado release ni se garantiza el tamaño de su próxima compilación. Las carpetas
de build, PDB, herramientas y documentación generada no forman parte del peso del exe.

La RAM es otra medida: una lectura previa aislada de debug, en condiciones no controladas,
registró 20 893 696 bytes de working set (19,9 MiB) y 2 985 984 bytes privados
(2,85 MiB). No se suman: representan conceptos distintos. Incluyen costes del proceso
y bibliotecas nativas, no solo su código Rust. Esta muestra no permite asegurar el
consumo esperado en reposo, con audio o con más cuentas; falta un benchmark reproducible.
Tampoco permite deducir el consumo a partir de la cantidad de líneas o del tamaño del exe.

## Compatibilidad pendiente

La GUI multiplataforma y Windows XP de 32 bits siguen siendo objetivos. No están
implementados ni validados. Solo hay adaptador Windows y pruebas locales x64;
compilar para x86 por sí solo no acredita XP. El código de dominio está separado
para reutilizarlo al implementar otros adaptadores. Fuera de Windows, el punto de
entrada actual solo informa que la GUI no está implementada.

El soporte oficial actual de Rust no incluye XP. La ruta de toolchain, biblioteca
estándar, linker, CPU, imports y pruebas reales en XP requiere una investigación
específica antes de prometer compatibilidad:
[plataformas de Rust](https://doc.rust-lang.org/rustc/platform-support.html).
