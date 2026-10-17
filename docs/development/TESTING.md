# Pruebas

Las pruebas unitarias no abren ventanas ni reproducen audio. El dominio recibe
instantes explícitos para probar las cuentas sin esperar tiempos reales.

## Ejecutar las comprobaciones

En Windows, desde PowerShell:

```powershell
cargo fmt --check
cargo test --release --locked --target x86_64-pc-windows-msvc
cargo clippy --locked --all-targets --target x86_64-pc-windows-msvc -- -D warnings
$env:RUSTDOCFLAGS = '-D warnings'
cargo doc --locked --no-deps --document-private-items --target x86_64-pc-windows-msvc
cargo build --release --locked --target x86_64-pc-windows-msvc
```

En Linux, con las [dependencias de desarrollo](../DEVELOPMENT.md#compilar-en-linux):

```sh
cargo fmt --check
cargo test --release --locked
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps --document-private-items
cargo build --release --locked
```

## Cobertura

La versión 0.1.2 Alpha pasa 12 pruebas en Windows y 21 en Linux.

| Área | Casos |
| --- | --- |
| Duraciones y comandos | Formatos, argumentos, Unicode, cero, desbordamiento y errores sin modificar cuentas existentes |
| Temporizadores | Constructor, pausa, continuación, detención y vencimiento sin repetir alarmas |
| Aplicación | Aislamiento por nombre, orden, reconfiguración, vencimientos conjuntos y próximo despertar |
| Entrada Linux | Edición UTF-8 y límites |
| Dibujo Linux | Reutilización de filas y glifos, RGB565/ARGB32, mezcla, padding y regiones sin cambios |
| Bibliotecas Linux | Error al cargar una biblioteca opcional inexistente |
| Teclado Wayland | Lectura del keymap independiente de la posición compartida del descriptor |

## Regresión XKB-822

`keymap_ignores_shared_file_offset` usa el callback real con un mapa válido y un
descriptor situado al final, en un punto intermedio y al principio. Comprueba
Escape y que el offset no cambie.

Con el código de 0.1.1, el caso situado al final reproduce `Failed to parse input
xkb string` y `xkbcommon rechazó el keymap`. La lectura posicional de 0.1.2 pasa
los tres casos. La prueba no necesita un compositor.

## Integración y prueba manual

En una ventana de Timer-Rust:

```text
crear uno 02:00
crear dos 0200
pausar uno
continuar uno
parar dos
configurar uno 00:05
```

Comprobar que las cuentas son independientes, que parar conserva la fila y que
`uno` finaliza con una alarma. Después, `continuar uno` debe mostrar un error;
`parar uno` debe mantenerlo finalizado sin volver a sonar.

Comprobar también un comando inválido, su corrección con Ctrl+A, un nombre con
acentos, el desplazamiento de una lista larga y la fuente al redimensionar.

En Linux, la integración usa [Sway aislado](LINUX-LAB.md). Para 0.1.2 se extrajo el
tar.gz, se verificaron permisos/checksum y se comprobó el arranque y cierre del
ELF con su registro vacío. La inspección PE de Windows confirma x64, subsistema
GUI y runtime C estático.

## Alcance de la validación

Hay pruebas manuales de comandos y estados en Windows y en el backend Linux
inicial. La rasterización Linux que sustituyó a Cairo tiene pruebas de píxeles e
integración; su aceptación visual manual sigue pendiente. También falta confirmar
el parche de teclado en la instalación que reportó el fallo.

Quedan por probar Linux físico, audio físico Linux, DPI por monitor,
suspensión/hibernación y escalabilidad con listas grandes. No hay pruebas de
inyección de fallos de asignación ni validación de Windows XP x86.

Las mediciones de recursos y sus condiciones están en [MEMORY.md](MEMORY.md).
