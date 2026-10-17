# Memoria Linux: causas y correcciones aplicadas

Revisión y corrección del 2026-10-16. La base es e6036bf, preparación inicial de
0.1.1-alpha. Los cuatro hallazgos se corrigieron en el código y se recompiló el
release. Las cifras siguientes proceden de mediciones de los procesos con /proc.

## Correcciones

| Causa en la base | Cambio implementado |
| --- | --- |
| sndfile y PulseAudio enlazados/cargados antes de sonar | Ya no aparecen en DT_NEEDED. Se cargan bajo demanda en un ayudante temporal del mismo ELF. |
| Cairo general y sus backends X11 | Cairo eliminado. Texto mediante FreeType y rasterización Rust; Fontconfig solo localiza la fuente, destruye sus patrones y cierra el catálogo. |
| Dos imágenes ARGB32 completas y repintado total | RGB565 cuando se anuncia, respaldo ARGB32; buffers reutilizables. Cada buffer conserva revisiones independientes de lista y pie y solo repinta las regiones cambiadas. |
| Vec/String nuevos para filas y copias al dibujar/ejecutar | Caché de filas con String reutilizable y actualización al cambiar segundo/estado; sin CString ni clonación de entrada. Eventos de texto usan un array UTF-8 de 64 bytes, sin String por tecla. |

Dos buffers siguen siendo necesarios para poder dibujar sin escribir sobre una
imagen todavía prestada al compositor. No se los elimina a costa de romper la
sincronización. Se mantienen máximo dos buffers y el límite de 32 MiB por buffer;
los tamaños anteriores se retiran cuando quedan libres. RGB565 reduce la precisión
de color y divide por dos los bytes de píxeles frente a ARGB32. Se conserva ARGB32
si el servidor no anuncia RGB565.

El fondo de cada buffer se inicializa al crearlo. Escribir en la entrada cambia
solo el pie; un tick cambia solo la lista. Ambos buffers recuerdan por separado
su contenido, por lo que alternarlos no restaura filas viejas. Rellenos opacos
escriben directamente; solo los glifos antialias mezclan el píxel de destino.

La fuente se mantiene a 16 px. Los bitmaps de glifos se reutilizan con límite de
256 glifos y 128 KiB de píxeles en total, sin caché de textos duplicados. build.rs
verifica los offsets ABI FreeType contra los headers instalados; no añade lógica
GUI escrita en C. La implementación conserva std y FFI nativas.

## Audio y vida de sus recursos

Se comprobó que dlclose no retiraba todas las bibliotecas de audio en esta
instalación. Por eso la GUI espera síncronamente a un proceso temporal del mismo
binario, iniciado sin shell en un modo privado que no abre Wayland ni fuentes.
El ayudante carga sndfile/PulseAudio, decodifica por bloques de 16 KiB y termina.
El cierre del proceso libera su memoria, incluso la retenida por el loader.

No hay un segundo ejecutable distribuido, reproductor externo requerido ni
proceso de audio permanente. La GUI mantiene un hilo antes/después del sonido;
libpulse puede crear hilos internos en el ayudante mientras reproduce. Este es
un coste temporal explícito, no una afirmación de que todo el sistema use un hilo.
La GUI espera hasta terminar, igual que antes. Los fallos vuelven a su estado
visible sin cerrarla. No tener instaladas las bibliotecas de audio ya no impide
abrir la ventana.

## Medición comparable antes y después

Void glibc 2.41, Sway 1.12 headless/pixman, Rust/Cargo 1.99.0. Un temporizador de
una hora pausado y el comando ayuda; mismas dimensiones, proceso nuevo, sin audio.
Se tomó /proc/PID/smaps, incluyendo memoria compartida. RSS en KiB:

| Ventana | Base e6036bf | Corrección | Reducción |
| --- | ---: | ---: | ---: |
| 660×420 | 12924 | 7008 | 5916 |
| 1280×800 | 18764 | 9920 | 8844 |

A 1280×800, la base tenía 8000 KiB de buffers, 9312 KiB en archivos de
bibliotecas y 1452 KiB restantes. La cadena de audio sumaba 1444 KiB y X11/xcb
572 KiB; ambos subconjuntos estaban incluidos en 9312, no son sumas adicionales.

En la versión corregida los buffers de 1280×800 suman 4000 KiB. El descenso de
RSS a ese tamaño es aproximadamente 8,64 MiB. Se midió también Sway: su PSS fue
15486 KiB en la base y 11708 KiB con la corrección a 1280×800. La reducción no
se obtuvo trasladando ese coste al compositor.

En 660×420 se observaron 1104 KiB privados residentes y 2414 KiB de PSS del
proceso corregido. Esos contadores cambian según quién comparta las páginas;
no son directamente equivalentes a la columna de memoria del Administrador de tareas de Windows.
No se hizo una nueva comparación Windows en iguales condiciones.

## Comprobación del sonido y del coste temporal

Otra ejecución a 660×420, con una cuenta pausada:

- GUI antes: RSS 7008 KiB, PSS 2435 KiB, un hilo.
- GUI después de tres alarmas: RSS 7008 KiB, PSS 2261 KiB y un hilo.
- Sin Cairo, X11/xcb, PulseAudio o sndfile en los mapas de la GUI antes/después.
- En otro muestreo del ayudante: pico observado de RSS 6304 KiB.
- En ese muestreo, pico de PSS agregado GUI+ayudante: 4629 KiB. Incluye la memoria del ayudante.
- PCM capturado en salida nula: 91851 muestras no nulas, pico 12843.
- Reposo con cuenta pausada: cero ticks de CPU adicionales en cinco segundos.

Son muestras locales, no garantías universales. FreeType/Fontconfig y sus
dependencias transitivas siguen siendo bibliotecas del sistema. El ELF corregido
mide 402528 bytes, frente a 345184 de la base: 56 KiB adicionales de archivo a cambio de reducir
memoria residente y controlar la vida del audio. Peso del archivo y RAM se
informan por separado.

## Verificación

En la revisión de memoria pasaron 20 pruebas Linux y 12 Windows. Las nuevas
comprueban reutilización de filas y glifos, UTF-8, límites/mezcla de píxeles,
padding con ancho impar en ambos formatos,
que editar el pie conserva los píxeles de la lista, que un frame sin cambios no
repinta y que una biblioteca opcional ausente devuelve error. También pasaron
formato, Clippy/Rustdoc con advertencias como errores y builds release.

Las pruebas visuales manuales registradas corresponden al renderizador anterior.
La rasterización corregida tiene pruebas de píxeles y de integración; queda
pendiente su aceptación visual manual.
