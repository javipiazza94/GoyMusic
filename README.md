# GoyMusic

Reproductor de música de código abierto para **tu propia música**. Biblioteca, playlists, cola y búsqueda con la comodidad de un reproductor en streaming, pero sobre los archivos que ya tienes en el disco. Sin cuentas, sin servidores y sin enviar nada fuera de tu equipo.

> Estado: v0.1, escritorio en Linux. Licencia GPL-3.0-or-later.

## Características

**Reproducción**
- Formatos: MP3, FLAC, Ogg Vorbis, WAV, AAC/M4A y ALAC.
- Reproducción **gapless**: la siguiente pista se precarga para que no haya cortes entre canciones.
- Cola con prioridad de usuario ("añadir a la cola"), aleatorio, repetición (no, todo, una) y "anterior" que reinicia la pista si llevas más de 3 s.
- Al cerrar se guarda la cola y la posición; se restauran al abrir.
- Cambio de frecuencia automático (resampleo) según tu dispositivo de salida.

**Biblioteca**
- Escaneo paralelo e incremental de tus carpetas; solo relee los archivos que cambiaron.
- Vigilancia de cambios en vivo: añade o mueve archivos y la biblioteca se actualiza sola.
- Las pistas conservan su identidad si las mueves, así que **tus playlists no se rompen**.
- Álbumes agrupados por MusicBrainz o por artista y título; los recopilatorios no se parten.
- Carátulas embebidas o `cover.*` / `folder.*`, con miniaturas en caché.

**Playlists y búsqueda**
- Playlists, carpetas de playlists, "Me gusta", arrastrar y soltar, menú contextual.
- Importar y exportar M3U / M3U8.
- Búsqueda instantánea (FTS5) con tolerancia a erratas y resultados agrupados en canciones, álbumes y artistas.

**Integración con el escritorio (Linux)**
- Controles multimedia **MPRIS**: teclas del teclado, GNOME Shell, pantalla de bloqueo y `playerctl`.
- Notificación con carátula al cambiar de canción.
- Bandeja del sistema: cerrar la ventana no corta la música. Instancia única.
- Mini-reproductor compacto, opcionalmente siempre encima (solo en X11; Wayland no lo permite).
- Tema claro y oscuro.

## Instalación

Cada commit en `main` genera un `.deb` y un `.AppImage` en los artefactos de [Actions](https://github.com/javipiazza94/GoyMusic/actions).

Para compilarlo tú mismo necesitas Rust estable, Node 24 y pnpm 11, más estas dependencias en Debian/Ubuntu:

```bash
sudo apt install libasound2-dev pkg-config ffmpeg libwebkit2gtk-4.1-dev \
  libsoup-3.0-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev dbus
```

```bash
pnpm install
pnpm tauri dev              # modo desarrollo
pnpm tauri build --bundles deb appimage   # paquetes instalables
```

## Arquitectura

```
crates/
  core/      servicios (traits), tipos de dominio y bus de eventos. Sin dependencias de UI.
  audio/     symphonia → rubato → ganancia → búfer circular → cpal
  library/   escaneo, vigilancia, SQLite, carátulas, playlists, búsqueda
  services/  implementación de los servicios del núcleo
apps/
  desktop/   app Tauri: comandos y eventos, MPRIS, bandeja, mini-reproductor
ui/          React + TypeScript + Vite
```

El núcleo no depende de Tauri: la UI habla con él por servicios tipados y eventos (nunca por polling). Los tipos de TypeScript se generan desde Rust, así que un cambio incompatible falla al compilar. El diseño completo está en [`docs/superpowers/specs`](docs/superpowers/specs/2026-10-16-goymusic-design.md).

## Contenido y límites

GoyMusic solo reproduce archivos locales. No usa catálogos ni audio de ningún servicio de streaming, y su identidad visual es propia.

## Desarrollo

```bash
scripts/check-quality.sh   # fmt, clippy, complejidad, secretos, vulnerabilidades, dependencias sin usar, tipos
cargo test --workspace
pnpm --dir ui test
```

Qué significa "terminado" en este proyecto: [`DONE.md`](DONE.md).

## Licencia

[GPL-3.0-or-later](LICENSE).
