# Avisos de componentes nativos

## Metadatos del protocolo xdg-shell

build.rs genera tablas ABI desde el XML xdg-shell del paquete wayland-protocols.
La implementación de la aplicación está escrita en Rust. El aviso del XML usado
para generar esos metadatos se conserva a continuación; no es una licencia del
código propio de Timer-Rust.

```text
Copyright © 2008-2013 Kristian Høgsberg
Copyright © 2013      Rafael Antognolli
Copyright © 2013      Jasper St. Pierre
Copyright © 2010-2013 Intel Corporation
Copyright © 2015-2017 Samsung Electronics Co., Ltd
Copyright © 2015-2017 Red Hat Inc.

Permission is hereby granted, free of charge, to any person obtaining a
copy of this software and associated documentation files (the "Software"),
to deal in the Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom the
Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice (including the next
paragraph) shall be included in all copies or substantial portions of the
Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

Origen: `/usr/share/wayland-protocols/stable/xdg-shell/xdg-shell.xml` del entorno
de compilación, wayland-protocols 1.49. Las bibliotecas dinámicas Wayland, Cairo,
xkbcommon y PulseAudio se obtienen del sistema; no se copian al paquete local.

## Bibliotecas del sistema

La versión Linux corregida utiliza FreeType para rasterizar fuentes:
This software is based in part on the work of the FreeType Team.
También usa Fontconfig, xkbcommon y Wayland; para audio carga libsndfile y
PulseAudio solo en el ayudante temporal. Estas bibliotecas se instalan desde
la distribución y no se incluyen en el paquete del proyecto.
