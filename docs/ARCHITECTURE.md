# Arquitectura y buenas prácticas

## Responsabilidades

| Archivo | Responsabilidad | Dependencias |
| --- | --- | --- |
| `src/main.rs` | Arranque y diagnóstico de salida | Adaptador Windows o Linux según destino |
| `src/duration.rs` | Validar y convertir duraciones | `std::time::Duration` |
| `src/command.rs` | Convertir texto a `Command` tipado | Parser de duración |
| `src/timers.rs` | Estado y transiciones de una cuenta | `std::time` |
| `src/app.rs` | Casos de uso, colección, filas y próxima actualización | Comandos y temporizadores |
| `src/windows.rs` | Propiedad de recursos, presentación, audio y eventos | Dominio y ABI Win32 |
| `src/windows/native.rs` | Declaraciones C y constantes del SDK | DLL nativas |
| `src/linux.rs` | Conexión, eventos y presentación Wayland | Dominio y módulos Linux |
| `src/linux/events.rs` | Listeners y conversión de teclado | Wayland/xkbcommon |
| `src/linux/render.rs` | Memoria compartida y texto | Wayland/Cairo |
| `src/linux/entry.rs` | Edición UTF-8 y límites | std, sin unsafe |
| `src/linux/audio.rs` | Archivo del tema por bloques, síncrono | libsndfile y PulseAudio simple |
| `src/linux/native.rs` | ABI y requests con nombres | Bibliotecas nativas |
| `build.rs` | Tablas ABI xdg-shell generadas, solo Linux | wayland-scanner, cc/ar, pkg-config |

Flujo de entrada: texto → `Command::parse` → `TimerApp::execute` → `Outcome` → GUI/audio.
Flujo de reloj: mensaje Win32 → `TimerApp::advance` → notificación → filas → siguiente plazo.

`Outcome` contiene texto y una solicitud de sonido. No llama al SO. Por ello se
pueden probar los casos de uso sin escritorio ni dispositivo de audio. La vista
posee el caché de filas; la lógica de temporización no conoce HWND ni el reproductor.
El dominio y los parsers prohíben `unsafe` mediante `#![forbid(unsafe_code)]`.
El adaptador Linux comparte ese dominio sin introducir Win32 allí. Sus contratos
de memoria, eventos y comprobaciones se detallan en [LINUX.md](LINUX.md).

## POO, DRY y SOLID en Rust

| Principio | Aplicación concreta y límite |
| --- | --- |
| POO | `Timer`, `TimerApp` y `Ui` encapsulan estado y comportamiento en estructuras y bloques `impl`. Se usa composición y propiedad; no se simulan jerarquías de clases. |
| DRY | Una implementación de duraciones, una gramática, una búsqueda mutable por nombre y un punto de presentación de efectos. Los dos modos de entrada temporal comparten el mismo parser. |
| S: responsabilidad única | Parsing, transiciones de estado, casos de uso y plataforma están separados. `Ui` concentra recursos y operaciones visuales; el planificador portable vive en `TimerApp`. |
| O: abierto/cerrado | Otro adaptador puede reutilizar el dominio sin añadir Win32 allí. Nuevos comandos todavía requieren cambios explícitos y exhaustivos en el enum y su ejecución; no se introduce un sistema de plugins. |
| L: sustitución | No hay herencia ni implementaciones intercambiables; no se afirma un cumplimiento artificial. Las invariantes del constructor se mantienen en los métodos. |
| I: segregación de interfaces | Métodos pequeños y resultados de datos; no hay una interfaz grande de GUI, almacenamiento y audio que el dominio deba implementar. |
| D: inversión de dependencias | Los casos de uso no invocan Win32 ni audio: describen efectos y el adaptador los ejecuta. No hace falta un trait con una sola implementación para lograr esta separación. |

No se añaden trait objects, contenedores de inyección, `Arc`, locks, singletons ni
factories sin una necesidad actual. El alcance minimalista prevalece sobre aplicar
patrones de manera ceremonial. La presentación de filas sigue en `TimerApp` porque
ambas partes son pequeñas; un presentador independiente solo se justificaría con
otra presentación real o requisitos distintos.

## Estado e invariantes

`Timer` oculta nombre y estado. Su constructor exige nombre no vacío sin espacios
ni caracteres de control, duración positiva y plazo representable. `TimerApp`
mantiene nombres únicos y orden de creación. Un comando de reconfiguración crea
el reemplazo válido antes de sustituir la cuenta anterior.

| Estado | Información conservada | Transiciones admitidas |
| --- | --- | --- |
| En marcha | `Instant` de vencimiento | Pausar, parar, finalizar, configurar |
| Pausado | `Duration` restante | Continuar, parar, configurar |
| Detenido | `Duration` restante | Continuar, parar, configurar |
| Finalizado | Sin plazo activo | Configurar; parar conserva finalizado |

Pausar y parar conservan la misma duración; su diferencia actual es el estado
visible. No se inventa una política de eliminación ni de restablecimiento.
El controlador comprueba vencimientos antes y después de ejecutar comandos.
El modelo supone instantes no decrecientes suministrados por el llamador.

## Tiempo y coste

El tiempo restante es `deadline.saturating_duration_since(now)`, no un contador
de eventos. Todas las filas de una actualización usan el mismo instante. La
fecha/hora civil del sistema no se usa para calcular las cuentas.

El planificador calcula el próximo cambio de segundo redondeado de cualquier
cuenta visible. Minimizado calcula el próximo vencimiento; sin cuentas activas
devuelve `None`. El adaptador redondea a milisegundos hacia arriba y aplica los
límites de `SetTimer`, de 10 a 0x7FFFFFFF ms. Si el plazo excede el máximo, un
despertar intermedio recalcula lo restante. No se cambia la resolución global del SO.

Hay un único ID de timer Win32 para todas las cuentas. Búsqueda, representación y
planificación son O(n). El vector evita estructuras adicionales para una lista
pequeña; no hay un límite arbitrario del número de temporizadores. Su escalabilidad
y consumo con listas grandes no han sido medidos.

## Contratos FFI y recursos

- `repr(C)` conserva los layouts de WNDCLASSW, RECT y MSG. Los punteros y WPARAM
  usan ancho nativo; los LONG de coordenadas son `i32`, incluso en x64.
- `extern "system"` utiliza la convención de llamada Windows. Las declaraciones
  y constantes están aisladas en `native.rs`, sin crates de bindings.
- Las cadenas son UTF-16 terminadas en cero. Las llamadas usadas para textos son
  síncronas y sus buffers viven hasta el retorno. LISTBOX copia cada cadena.
- Los callbacks no acceden a `TimerApp` ni capturan referencias mutables. La
  entrega síncrona de mensajes puede ser reentrante sin prestar ese estado al SO.
- `Ui` posee ventana y fuente. `Drop` destruye primero la ventana y sus hijos y
  después elimina la fuente que usaban. Los errores posteriores a crear la ventana
  también pasan por `Drop`. Se comprueba si la ventana sigue existiendo al liberar.
- Cursor y brocha de sistema son compartidos y no se destruyen. La clase de ventana
  se registra una vez por proceso y queda registrada hasta terminar el proceso;
  `run` se llama una sola vez desde `main`.
- `WM_CLOSE` solicita salir del bucle. La destrucción efectiva queda en el propietario
  RAII, evitando intentar destruirla tanto desde el cierre como desde `Drop`.
- Se comprueban los fallos de inicialización, `GetMessageW`, `SetTimer` y carga de
  filas. El redibujado se reactiva aunque falle la reserva del LISTBOX.
- El audio usa `SND_ALIAS | SND_NODEFAULT | SND_SYNC`. Bloquea el hilo GUI y puede
  retrasar la detección de otros vencimientos. No se ha añadido un hilo para ocultarlo.
- La limpieza y las actualizaciones auxiliares de estado/posición no hacen panic;
  no todos sus fallos se convierten en diagnósticos. Un fallo de asignación de memoria
  de Rust puede terminar el proceso; esta versión no ofrece recuperación ante OOM.

Contratos consultados en Microsoft: [SendMessageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessagew),
[DestroyWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-destroywindow),
[SetTimer](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-settimer).

## Límites deliberadamente pendientes

El adaptador mantiene el comportamiento visual existente. Ventanas extremadamente
pequeñas pueden solapar controles; la ayuda y nombres largos pueden quedar recortados.
No hay navegación completa de diálogo con Tab ni historial de comandos. El estado
de selección de la lista no determina el objetivo: el nombre del comando lo determina.

Audio configurable y extracción, persistencia, recurrencias, otras plataformas, DPI por monitor,
suspensión del sistema y XP x86 necesitan requisitos o validación propios. La revisión
no los presenta como funciones implementadas.

La GUI Linux Wayland está implementada y comprobada en Sway headless con glibc.
La validación en hardware físico sigue pendiente; no acredita todo entorno Linux.
