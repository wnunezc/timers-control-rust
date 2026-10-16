# Laboratorio Linux: Void, glibc y Sway

## Objetivo y alcance

Entorno de desarrollo y pruebas de aplicaciones Linux nativas: Void Linux x86_64,
glibc y Wayland con Sway. No usa Wine. La GUI nativa de Timer-Rust ya está
implementada y comprobada aquí; ver [LINUX.md](LINUX.md) para compilar y ejecutarla.

Por defecto, el laboratorio utiliza una pantalla virtual de Sway con backend headless. Sus
ventanas, teclado virtual y capturas no se muestran ni envían entrada al escritorio
de Windows. WSLg solo proporciona aquí el servidor de audio compartido; la sesión
gráfica de prueba tiene sus propios sockets y no utiliza su compositor Weston.

No hay GNOME, KDE ni Xfce. Sway tiene XWayland explícitamente desactivado y no carga
la configuración global de barras, lanzadores o accesos directos de otra sesión.

## Instalación local

| Elemento | Ubicación o versión comprobada |
| --- | --- |
| Distribución WSL | `Void-Dev`, WSL 2 |
| Disco virtual | `D:\WSL\Void-Dev\ext4.vhdx` |
| Usuario predeterminado | `dev`, UID 1000, sin privilegios administrativos adicionales |
| Configuración reproducible | `D:\WSL\Void-Lab\configure-void.sh` |
| Caché de imagen y firmas | `D:\WSL\Downloads\Void` |
| Compositor | Sway 1.12, salida `HEADLESS-1`, 1280×800, escala 1 |
| Renderizador | pixman, por software |
| Biblioteca C | glibc 2.41 |
| Compilador | Rust/Cargo 1.99.0, paquetes de Void |
| Bibliotecas de desarrollo | Wayland 1.26.0, xkbcommon 1.13.2, GCC/glibc-devel y pkg-config |
| Herramientas de prueba | grim, wtype, wayland-info, jq y foot |

La distribución se importó desde la imagen oficial
`void-x86_64-ROOTFS-20250202.tar.xz`, de 48 895 416 bytes. Se verificó la firma
minisign del listado de checksums con la clave de publicación oficial y luego
el SHA-256 del rootfs:

```text
3f48e6673ac5907a897d913c97eb96edbfb230162731b4016562c51b3b8f1876
```

Después se actualizaron XBPS y los paquetes de la distribución. La imagen base
comprimida no representa el tamaño de un entorno de desarrollo completo.

Fuentes: [imágenes oficiales de Void](https://repo-default.voidlinux.org/live/current/),
[verificación de imágenes](https://docs.voidlinux.org/installation/index.html),
[importación de distribuciones en WSL](https://learn.microsoft.com/en-us/windows/wsl/use-custom-distro).

## Uso desde PowerShell

Iniciar el compositor aislado:

```powershell
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-start
```

Ejecutar herramientas dentro de esa sesión:

```powershell
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-run swaymsg -t get_outputs
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-run wayland-info
```

`lab-run` carga los sockets reales escritos por un hijo de Sway en
`/home/dev/.local/run/sway-lab/session.env`. Quita DISPLAY y WAYLAND_SOCKET para
evitar rutas de conexión heredadas hacia X11 o WSLg. No inicia el compositor si
está detenido: primero debe ejecutarse `lab-start`.

Abrir el cliente de demostración dentro de la pantalla virtual:

```powershell
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-run swaymsg exec /home/dev/.local/bin/lab-demo
```

La demostración usa foot con app_id `lab-check` y un proceso `cat`: muestra texto
tecleado y no lo interpreta como órdenes. La doble aparición del texto corresponde
al eco de la terminal y a la salida de cat. Se lanza mediante el compositor para
que su ciclo de vida no dependa del comando breve de WSL que la solicita.

Enviar texto exclusivamente a la ventana que tenga foco dentro de Sway:

```powershell
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-run wtype -s 250 -d 20 WAYLAND-OK -k Return
```

La espera inicial de 250 ms permite que se establezca el foco del nuevo teclado
virtual; en la primera prueba sin esa espera se perdió el primer carácter.
Antes de automatizar una aplicación, consultar `swaymsg -t get_tree` y seleccionar
explícitamente su ventana para no escribir en otro cliente del laboratorio.

Capturar la pantalla virtual, sin capturar Windows:

```powershell
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-run grim /mnt/d/WSL/Void-Lab/evidence/sway-lab.png
```

Para cerrar solo el compositor, o detener por completo esta distribución:

```powershell
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-stop
wsl --terminate Void-Dev
```

No se necesita `wsl --shutdown`, que afectaría a las otras distribuciones. La
distribución predeterminada de la máquina continúa siendo docker-desktop.
Los cambios administrativos de este laboratorio se ejecutan explícitamente como root:
`wsl -d Void-Dev -u root --exec ...`; no se asignó una contraseña nueva a dev.

## Compilar el núcleo Rust en Linux

El código fuente se lee desde la carpeta del proyecto en D:, pero los resultados
de compilación Linux se guardan en el filesystem ext4 de la distribución:

```powershell
wsl -d Void-Dev --exec /bin/sh -lc 'CARGO_TARGET_DIR=/home/dev/projects/timer-rust-target cargo test --release --locked --manifest-path /mnt/d/OpsZone/DevWorkspace/Projects/Desktop/Timer-Rust/Cargo.toml'
```

Las opciones de `.cargo/config.toml` se aplican solo al destino Windows MSVC; no
activan CRT estático en el compilador Linux. No se construye aquí un ejecutable GUI
Linux por este comando de pruebas; para generar la GUI se usa cargo build --release.

## Validación y límites

Comprobado el 2026-10-16:

- Sway con una salida headless activa de 1280×800.
- Cliente de prueba identificado por Sway con `shell: xdg_shell`, no XWayland.
- Protocolos de ventanas, captura de pantalla y teclado virtual disponibles.
- Entrada virtual y captura PNG correctas dentro de la sesión aislada.
- Conexión al servidor PulseAudio de WSLg correcta, sin reproducir sonido durante
  la preparación. Esa prueba no acredita el audio de una instalación física de Void.
- Las 12 pruebas Rust del núcleo pasan compiladas y ejecutadas en Linux.
- Arranque, reinicio y cierre del compositor comprobados. `lab-stop` verifica que
  la instancia deje de responder: Sway puede cerrar su socket antes de devolver la
  respuesta del comando exit. La distribución quedó detenida al terminar la prueba.

Durante la comprobación, el VHDX ocupaba aproximadamente 2,3 GiB; el filesystem
registraba unos 2,1 GiB usados, incluyendo compiladores y unos 457 MiB de caché XBPS.
Una lectura aislada registró RSS de Sway 25 392 KiB, dbus-daemon 2668 KiB, swaybg
5852 KiB y foot 18 024 KiB. No se suman como memoria exclusiva: comparten páginas
y no incluyen todos los costes de WSL/WSLg. No constituyen un benchmark.

La sesión se puede detener cuando no se use para evitar procesos de prueba en reposo.
Este entorno verifica Sway/Wayland en un kernel WSL y con renderizado por software;
no acredita GPU, DRM, entrada física, suspensión ni audio del equipo del destinatario.
Las pruebas finales también deberán realizarse en su instalación real de Void/Sway.

## Sesión visible para pruebas manuales

Por petición del usuario se añadió una sesión Sway anidada en WSLg, visible como
ventana de Windows. Se mantiene separada del laboratorio headless; no usa XWayland.
Configuración reproducible local: `D:\WSL\Void-Lab\configure-manual.sh`;
configuración de Sway en `/home/dev/.config/sway-view/config`.

Abrir la sesión y Timer-Rust desde PowerShell:

```powershell
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-view-start
```

La configuración inicia el ELF Linux automáticamente. El backend es Wayland,
renderer pixman y salida WL-1 de 1000×700; teclado y ratón se manejan manualmente
en esa ventana. El audio usa la salida predeterminada de WSLg, sin salida nula.
Esto permite probar sonido real a través de Windows; no valida el hardware de
una instalación física de Void. No se automatiza la entrada del usuario.

Cerrar esta sesión con Ctrl+Alt+Shift+Q dentro de ella o desde PowerShell:

```powershell
wsl -d Void-Dev --exec /home/dev/.local/bin/lab-view-stop
```

`lab-view-run` ejecuta herramientas dentro de esta sesión visible, con sockets en
`/home/dev/.local/run/sway-view`. Los comandos `lab-start`, `lab-run` y `lab-stop`
siguen correspondiendo exclusivamente a la sesión headless.
