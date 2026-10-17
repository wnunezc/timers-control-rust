# Timer-Rust

Temporizadores de cuenta regresiva para Windows y Linux. Una ventana, una lista
de cuentas y un campo de comandos. La fuente mantiene su tamaño al redimensionar
la ventana y cada cuenta avisa con sonido al terminar.

Escrito en Rust, sin crates externos, con interfaces nativas del sistema.

## Descargar

La versión actual es [0.1.2 Alpha](https://github.com/wnunezc/timers-control-rust/releases/tag/v0.1.2-alpha).

| Sistema | Archivo | Requisitos |
| --- | --- | --- |
| Windows | `timer-rust-v0.1.2-alpha-windows-x86_64.exe` | Windows 11 x64, donde se ha probado |
| Linux | `timer-rust-v0.1.2-alpha-linux-x86_64.tar.gz` | x86_64, glibc y una sesión Wayland; probado en Void Linux con Sway |

En Windows, descarga y ejecuta el `.exe`. No requiere instalación ni tener Rust
instalado. Para Linux, sigue la [guía de instalación](docs/LINUX.md).

Las descargas incluyen `SHA256SUMS.txt` para comprobar su integridad.

## Uso

Escribe un comando en el campo inferior y pulsa **Enter**:

```text
crear cafe 05:00
crear descanso 15m
pausar cafe
continuar cafe
```

Los nombres deben ser únicos y no pueden contener espacios. Tanto los nombres
como los comandos distinguen mayúsculas de minúsculas.

| Comando | Qué hace |
| --- | --- |
| `crear cafe 05:00` | Crea e inicia una cuenta de cinco minutos. |
| `configurar cafe 2m` | Cambia la duración y vuelve a iniciar la cuenta. |
| `pausar cafe` | Pausa la cuenta. |
| `continuar cafe` | Reanuda una cuenta pausada o detenida. |
| `parar cafe` | Detiene la cuenta y conserva el tiempo restante en la lista. |
| `sonido` | Prueba la alarma. |
| `ayuda` | Muestra los comandos disponibles. |

Al llegar a cero, la cuenta queda como `finalizado`. Para usarla otra vez,
ejecuta `configurar`. Si un comando tiene un error, aparece un mensaje y el texto
permanece en la entrada para corregirlo.

**Las cuentas no se guardan al cerrar la aplicación.**

### Formatos de tiempo

| Formato | Ejemplo | Duración |
| --- | --- | --- |
| Minutos y segundos | `01:30` o `0130` | 90 segundos |
| Horas, minutos y segundos | `01:02:03` o `010203` | 1 hora, 2 minutos y 3 segundos |
| Horas | `2h` | 2 horas |
| Minutos | `90m` | 90 minutos |
| Segundos | `30s` | 30 segundos |

En los formatos de reloj, cada campo lleva dos dígitos y los minutos y segundos
van de `00` a `59`. Para duraciones mayores en una sola unidad, usa el sufijo:
`90m`, por ejemplo. La duración debe ser un entero positivo; no se admiten cero
ni decimales.

### Sonido

En Windows se usa el sonido de exclamación del esquema de sonidos configurado.
En Linux se usa `complete.oga`, del paquete `sound-theme-freedesktop`.

La ventana espera a que termine el sonido para volver a responder. Si varias
cuentas vencen en la misma actualización, se emite una sola alarma y aparecen
sus nombres en el área de estado.

## Limitaciones de la alpha

- No guarda cuentas entre sesiones ni incluye alarmas repetitivas.
- No permite elegir archivos de audio ni extraer fragmentos.
- La edición de texto en Linux todavía no incluye portapapeles, IME ni selección parcial con el ratón.
- Windows XP, Linux de 32 bits, musl y otros sistemas no están soportados por estas descargas.

Consulta el [historial de cambios](CHANGELOG.md) para ver las novedades de cada versión.

## Desarrollo

La [guía de desarrollo](docs/DEVELOPMENT.md) reúne compilación, arquitectura,
pruebas y notas sobre la evolución y el consumo de memoria del proyecto.

## Créditos

Todos los créditos conceptuales corresponden a
[elhacker1000](https://gitlab.com/elhacker1000).

Código Rust: [Walter Nuñez (wnunezc)](https://github.com/wnunezc).
