# Pruebas

Entorno de validación inicial: Windows 11 Pro x64, Rust/Cargo 1.98.1 y destino
`x86_64-pc-windows-msvc`. Las pruebas unitarias no abren ventanas ni reproducen audio.

## Comprobaciones automatizadas

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --target-dir target/review
$env:RUSTDOCFLAGS = '-D warnings'
cargo doc --no-deps --document-private-items
```

`target/review` separa el ejecutable de comprobación del ejecutable de desarrollo
predeterminado. RUSTDOCFLAGS afecta solo a la sesión actual de PowerShell.

Resultado registrado el 2026-10-16: 12 pruebas pasan; formato, Clippy, compilación
de desarrollo y documentación privada con advertencias como errores correctos.
Para la Alpha también pasan las 12 pruebas con optimización y runtime C estático:

```powershell
cargo test --release --locked --target x86_64-pc-windows-msvc
cargo build --release --locked --target x86_64-pc-windows-msvc
```

La inspección PE confirma x64 y subsistema Windows GUI. Con CRT estático, las
dependencias importadas son USER32, WINMM, KERNEL32, GDI32, ntdll y el contrato
de API de sincronización de Windows; no se importa VCRUNTIME140.dll ni UCRT dinámico.
El ejecutable Alpha concreto aún no tiene prueba manual de GUI registrada.

La suite cubre formatos de duración, Unicode y overflow, argumentos faltantes y
extra, invariantes del constructor, pausa/continuación, detención, vencimiento
tardío sin repetición, aislamiento por nombre, nombres duplicados, reconfiguración
válida e inválida, vencimientos conjuntos, planificación en reposo y minimizado,
y solicitud de audio como datos sin efectos externos. El tiempo se suministra
mediante `Instant`; no se usan sleeps para simular cuentas reales.

## Comprobación manual

En la entrada inferior, ejecutar cada comando pulsando Enter:

```text
crear uno 02:00
crear dos 0200
pausar uno
```

`uno` debe conservar su tiempo y mostrar pausado; `dos` debe continuar descontando.
Al cambiar el tamaño de ventana, la fuente mantiene su tamaño y la entrada sigue abajo.

```text
continuar uno
parar dos
configurar uno 00:05
```

`dos` permanece detenido en la lista. `uno` inicia cinco segundos, termina y emite
una alarma. Después:

```text
parar uno
continuar uno
```

`uno` conserva finalizado y no vuelve a sonar; continuar muestra un error.
Estos casos de comandos, estados y alarma tienen confirmación manual local.

## Linux Wayland

El [backend Linux](LINUX.md) tiene 20 pruebas: las 12 compartidas y 8 de entrada, cachés,
fuente, píxeles/regiones y carga opcional de bibliotecas.
Ejecutar en Linux con los paquetes de desarrollo indicados en esa guía:

```sh
cargo test --release --locked
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --document-private-items
```

La GUI se probó en [Sway headless](LINUX-LAB.md), sin XWayland: comandos, estados,
vencimiento, alarma, UTF-8, entrada inválida/corrección, lista desplazable,
redimensionado y fallos de conexión/audio. Las mediciones están en LINUX.md.
El audio se capturó en salida nula; escuchar sonido físico sigue pendiente.
El usuario confirmó además la prueba manual de la GUI Linux en Sway anidado
en WSLg antes de preparar la 0.1.1 Alpha. Esa confirmación no acredita otra distro,
GPU o hardware Linux físico.

## Pendientes y límites de la evidencia

- Prueba específica de cierre, minimizar/restaurar y DPI por monitor.
- Ejecución manual de la GUI de release. Un intento histórico recibió un error de
  archivo en uso; esta publicación verifica build, pruebas e imports, sin abrir la GUI.
- Medición reproducible de memoria y CPU en reposo, activo, minimizado y con audio,
  indicando cantidad de cuentas, duración de muestreo y herramientas usadas.
- Ensayos de ABI, imports y ejecución real si se implementa Windows XP x86.
- Fallos de memoria del LISTBOX: comprobados en código, sin inyección de errores.

La muestra histórica de RAM del README no es un benchmark. Working set y memoria
privada son métricas distintas y no se suman. El tamaño del exe y las líneas de
código no permiten deducir RAM ni ahorro de 10 MB.

## Corrección de memoria Linux

Las pruebas específicas comprueban reutilización de capacidad de filas/glifos,
mezcla de píxeles y padding en RGB565/ARGB32, preservación del área de lista al
editar solo el pie y ausencia de dibujo en un frame sin cambios. La carga de una
biblioteca opcional inexistente devuelve error. Estos tests no abren ventanas.
La medición del proceso y del ayudante de audio está en MEMORY-REVIEW.md; no usa
capturas de pantalla para atribuir memoria. La nueva rasterización queda pendiente
de validación manual del usuario.
