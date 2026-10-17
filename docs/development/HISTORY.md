# Historial técnico

Este documento recoge la evolución de la implementación. Las novedades para
usuarios están en [CHANGELOG.md](../../CHANGELOG.md).

## 0.1.0 Alpha: Windows

La primera implementación utilizaba controles Win32 y sonido del sistema. La
revisión separó el parser de duraciones, los comandos, el modelo de cuenta y los
casos de uso del adaptador gráfico. Se añadieron 12 pruebas del núcleo compartido.

La planificación pasó a usar plazos con `Instant` y un solo timer Win32. Sin
cuentas activas se cancela el timer; minimizado se espera al próximo vencimiento.
La publicación x64 incluyó el runtime C estático.

## 0.1.1 Alpha: Linux y memoria

Se incorporó Wayland/xdg-shell, edición de texto UTF-8, desplazamiento de filas y
sonido con libsndfile/PulseAudio. El núcleo de temporizadores se mantuvo compartido.
Las pruebas de integración se realizaron con Void/glibc/Sway en WSL.

La implementación Linux inicial tenía costes evitables en cuatro puntos:

- Bibliotecas de audio cargadas desde el arranque.
- Cairo y sus componentes gráficos adicionales.
- Buffers ARGB32 completos y repintado de toda la imagen.
- Asignaciones y copias de cadenas en las actualizaciones.

La corrección sustituyó Cairo por FreeType y píxeles Rust, negoció RGB565 con
respaldo ARGB32, separó las regiones de dibujo y reutilizó filas/glifos. El audio
pasó a un proceso temporal del mismo binario. Las pruebas Linux aumentaron a 20.
El [informe de memoria](MEMORY.md) conserva el desglose y los resultados.

## 0.1.2 Alpha: keymap Wayland

Se reprodujo `XKB-822` con un mapa válido y el descriptor situado al final. La
lectura secuencial dependía del offset compartido y entregaba una cadena vacía a
xkbcommon. Se cambió por lectura posicional desde cero y se validaron tamaño y NUL.

La prueba del callback real verifica los offsets final, intermedio e inicial,
la interpretación de Escape y la conservación del offset. La suite Linux pasó a
21 pruebas; Windows conserva sus 12 pruebas y no tuvo cambios funcionales.

## Tamaños de las publicaciones

| Versión | Windows x64 EXE | Linux x86_64 ELF | Linux tar.gz |
| --- | ---: | ---: | ---: |
| 0.1.0 Alpha | 227 840 bytes | — | — |
| 0.1.1 Alpha | 228 352 bytes | 402 528 bytes | 200 766 bytes |
| 0.1.2 Alpha | 228 352 bytes | 402 528 bytes | 200 107 bytes |

Los paquetes Linux incluyen guía, avisos y checksum, pero no las bibliotecas
compartidas del sistema. El tamaño del ejecutable no representa su consumo de RAM.
En el ELF de referencia se observaron imports hasta GLIBC_2.34; esto no establece
un mínimo de ejecución porque las bibliotecas compartidas tienen requisitos propios.
El ELF anterior a la corrección de memoria medía 345 184 bytes; el incremento de
56 KiB permitió reducir el coste residente y acotar la vida de los recursos de audio.

## Inventario de código de la revisión de memoria

Referencia del 2026-10-16, commit `627dc38`, anterior al parche del teclado:

| Área | Líneas físicas Rust |
| --- | ---: |
| Adaptador Windows y ABI | 600 |
| Núcleo compartido | 619 |
| Adaptador Linux y módulos | 2307 |
| Punto de entrada | 43 |
| build.rs | 80 |
| Total | 3649 |

En esa revisión había 393 líneas dentro de módulos de pruebas, 224 líneas
completas de comentarios/documentación y 152 vacías. Las categorías se superponen:
no deben restarse entre sí. Los archivos Rust sumaban 126 303 bytes. Es un inventario
histórico, no el recuento de la versión actual ni una estimación de RAM.

## Trabajo pendiente

Persistencia, recurrencias y selección/extracción de audio siguen sin implementar.
También faltan portapapeles e IME Linux, DPI por monitor, pruebas de suspensión y
validación física Linux. Windows XP x86 requiere una toolchain y pruebas propias;
no se considera compatible a partir de una compilación de 32 bits.
