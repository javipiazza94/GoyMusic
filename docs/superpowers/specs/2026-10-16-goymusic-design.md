# GoyMusic — diseño v1

Fecha: 2026-10-16 · Estado: pendiente de revisión · Licencia del proyecto: GPL-3.0-or-later

## 1. Objetivo

App de música **open source** con experiencia de uso tipo Spotify (biblioteca, playlists, cola, búsqueda) sobre la
**música propia del usuario**. Escritorio primero (Linux); el núcleo se diseña para exponerse más adelante como
servidor sin reescribirse.

**Criterio de éxito (finish line de la v1):** Javi la usa a diario en lugar de otro reproductor para su biblioteca
local, cumpliendo todos los puntos de la §10, y el núcleo no depende de Tauri (el modo servidor solo añadiría un adaptador).

### Límites que condicionan el diseño
- **Contenido:** solo música del usuario (ficheros locales). No se usa el catálogo ni el audio de Spotify: su
  Developer Policy prohíbe "products or services that mimic, or replicate or attempt to replace a core user
  experience of Spotify" sin permiso escrito, y saltarse su DRM es ilegal.
- **Proyecto `spotify-re`:** se reutiliza solo el **conocimiento de arquitectura** (UI ↔ núcleo por servicios
  tipados; reparto player/biblioteca/playlists; modelo contexto + cola). No se copia su código, sus `.proto` ni sus
  nombres; `spotify-re` sigue siendo un repositorio privado de investigación.
- **Marca e identidad:** nombre propio (GoyMusic) e identidad visual propia; no se imita el aspecto de Spotify.

## 2. Decisiones

| Tema | Decisión |
|---|---|
| Forma | App de escritorio local; modo servidor después reutilizando el mismo núcleo |
| UI | Tauri 2 + React + TypeScript + Vite |
| Núcleo | Rust (workspace de Cargo) |
| Comunicación UI ↔ núcleo | **Enfoque A**: núcleo como librería con servicios tipados y transporte intercambiable (hoy comandos/eventos de Tauri; después WebSocket/HTTP) |
| Plataforma v1 | Linux (multiplataforma después) |
| Licencia | GPL-3.0-or-later |

## 3. Arquitectura

```
crates/
  core/      servicios (traits), tipos de dominio, errores, bus de eventos. Sin dependencias de UI ni Tauri.
  audio/     motor de reproducción: symphonia → rubato → ganancia → búfer circular → cpal
  library/   escaneo, vigilancia de cambios, SQLite, carátulas, playlists, favoritos, búsqueda
  desktop/   MPRIS (zbus), notificaciones
apps/
  desktop/   app Tauri: adaptador servicios ↔ comandos/eventos; generación de tipos TS (tauri-specta)
ui/          React + TS + Vite
```

Reglas de dependencia: `core` no depende de ningún otro crate del workspace. `audio`, `library` y `desktop`
implementan o consumen interfaces de `core`. Solo `apps/desktop` conoce Tauri. El futuro `apps/server` (axum +
WebSocket) se añadirá sin tocar `core`, `audio` ni `library`.

### Flujo
UI → comando tipado → servicio → cambio de estado → **evento** en el bus del núcleo → el adaptador lo reenvía a la UI →
la UI actualiza su store. La UI nunca hace polling.

Eventos v1: `PlayerStateChanged`, `PositionChanged` (~4 Hz), `QueueChanged`, `PlaybackError`, `LibraryUpdated`,
`ScanProgress`, `PlaylistsChanged`, `LikesChanged`.

## 4. Servicios del núcleo

- **Player**: `play(context, start_index)`, `pause`, `resume`, `seek(position_ms)`, `next`, `previous`,
  `set_shuffle(bool)`, `set_repeat(off|all|one)`, `set_volume(0.0–1.0)`, `queue_add(tracks)`, `queue_remove(index)`,
  `queue_move(from, to)`, `state()`.
  - `context` = la lista de la que se reproduce: álbum, artista, playlist, "Me gusta", resultados de búsqueda o lista
    explícita de pistas.
- **Library**: `add_folder`, `remove_folder`, `folders`, `rescan`, `albums`, `album(id)`, `artists`, `artist(id)`,
  `tracks(filtro, orden, página)`, `track(id)`.
- **Playlists**: `create`, `rename`, `delete`, `list`, `get(id)`, `add_tracks`, `remove_items`, `move_item`,
  `create_folder`, `move_to_folder`, `like(track)`, `unlike(track)`, `liked_tracks`, `import_m3u(path)`,
  `export_m3u(id, path)`.
- **Search**: `search(query, límite)` → `{ top_result, tracks, albums, artists }`.

## 5. Motor de audio

- **Hilo dedicado** fuera del runtime async; `Player` le envía órdenes por canal. Estados:
  `Idle → Loading → Playing ⇄ Paused → Ended`. La posición se calcula a partir del contador de muestras.
- **Cadena**: archivo → symphonia (decodificación) → rubato (resampleo si la frecuencia del archivo ≠ la del
  dispositivo) → ganancia → búfer circular sin bloqueos → cpal.
- **Formatos v1**: MP3, FLAC, Ogg Vorbis, WAV, AAC/M4A, ALAC.
- **Gapless**: cuando quedan ~10 s de la pista actual se abre y precarga el decodificador de la siguiente; sus
  muestras se escriben a continuación en el mismo búfer, recortando el retardo y el relleno del codificador.
- **Cola**:
  - Contexto + **cola de usuario** ("añadir a la cola"), que tiene prioridad sobre el contexto.
  - Shuffle: permutación del contexto que conserva la pista actual en su sitio.
  - Repeat: off / all / one.
  - "Anterior": si la posición > 3 s, reinicia la pista; si no, vuelve a la anterior del historial.
  - Al salir se guardan la cola y la posición (`queue_state`); se restauran al abrir, en pausa.
- **Volumen**: ganancia por software. ReplayGain por pista como ajuste opcional, desactivado por defecto.
- La salida se abstrae tras `AudioSink` (implementación cpal + implementación de prueba que captura muestras).

## 6. Biblioteca, datos y búsqueda

- **Escaneo**: recorrido paralelo de las carpetas añadidas, filtrado por extensión. **Incremental**: si tamaño y
  mtime no cambian, el archivo no se relee. Etiquetas con lofty (título, artista, artista del álbum, álbum,
  pista/disco, año, género, duración, IDs de MusicBrainz, carátula embebida). Sin etiquetas, el título sale del
  nombre de archivo. Progreso por `ScanProgress`.
- **Vigilancia**: notify (inotify) con debounce de 2 s → reescaneo de las carpetas afectadas. Si se alcanza el límite
  de inotify: reescaneo periódico cada 10 min y aviso en ajustes.
- **Identidad estable de pistas**: ID interno (no la ruta). En un reescaneo, un archivo nuevo que coincide con uno
  desaparecido (mismo ID de grabación de MusicBrainz o, si no hay, mismo tamaño + duración ±1 s + título + artista +
  álbum) se trata como movido: conserva su ID y se actualiza la ruta. Las playlists sobreviven a mover archivos.
- **Álbumes**: agrupados por ID de release de MusicBrainz si existe; si no, por artista del álbum (o artista) + título
  normalizado. Los recopilatorios no se parten.
- **SQLite** (rusqlite, modo WAL, migraciones versionadas). Tablas: `folders`, `tracks`, `albums`, `artists`,
  `track_artists`, `playlists`, `playlist_items(pos)`, `playlist_folders`, `likes(liked_at)`, `play_history`,
  `queue_state`, `settings`. Ruta: `~/.local/share/goymusic/library.db`.
- **Carátulas**: embebidas, o `cover.*` / `folder.*` en la carpeta del álbum. Miniaturas WebP de 200 y 600 px en
  `~/.cache/goymusic/art/`, servidas a la UI con el protocolo propio `art://` (sin base64).
- **Playlists**: posiciones enteras; reordenar en una transacción. Importar M3U/M3U8 resuelve rutas relativas y
  devuelve la lista de entradas no encontradas. Exportar genera M3U8.
- **Búsqueda**: FTS5 (`unicode61`, `remove_diacritics 2`) con búsqueda por prefijo mientras se escribe. Si FTS5
  devuelve menos de 5 resultados, se aplica coincidencia difusa (Jaro-Winkler ≥ 0,85) sobre los nombres de artistas,
  álbumes y títulos. Resultados agrupados: mejor resultado, canciones, álbumes y artistas. Objetivo: < 50 ms con
  50 000 pistas.

## 7. Interfaz

- **Estado**: TanStack Query para los datos de servicios, invalidado por los eventos del núcleo; Zustand para el
  estado del reproductor, alimentado por `PlayerStateChanged` / `PositionChanged` / `QueueChanged`.
- **Disposición**: barra lateral (biblioteca + playlists), vista principal, barra inferior de reproducción, panel
  derecho con la cola.
- **Rutas**: `/`, `/album/:id`, `/artist/:id`, `/playlist/:id`, `/collection/tracks` (Me gusta), `/search?q=`,
  `/settings`.
- **Interacción**:
  - Listas virtualizadas (TanStack Virtual).
  - Arrastrar y soltar a playlists y a la cola.
  - Menú contextual: añadir a la cola, a una playlist, ir al álbum o al artista.
  - Atajos: espacio (play/pausa), `Ctrl+F` (buscar), `Ctrl+←/→` (anterior/siguiente).
  - Tema oscuro y claro.
- **Diseño visual**: identidad propia, definida con la skill Hallmark en la fase de implementación de la UI.
- **Tipos**: los tipos TS se generan desde Rust (tauri-specta); un cambio incompatible falla al compilar.

## 8. Integración de escritorio (Linux)

- **MPRIS** (zbus): teclas multimedia, controles de GNOME Shell y de la pantalla de bloqueo, `playerctl`.
- **Notificación** al cambiar de canción, con carátula.
- **Bandeja del sistema**: cerrar la ventana no detiene la música; "Salir" sí.
- **Instancia única**: abrirla otra vez enfoca la existente.
- **Mini-reproductor**: ventana compacta, opcionalmente siempre encima. Límite de plataforma: Wayland no deja a las apps quedarse encima, así que allí no se ofrece el pin (sí bajo X11 o con `GDK_BACKEND=x11`).
- **Bandeja**: solo cuenta si hay un host StatusNotifier en el bus (`org.kde.StatusNotifierWatcher`); sin él (GNOME sin la extensión AppIndicator), cerrar la ventana sale de la app.

## 9. Errores, logs y pruebas

- **Errores**: cada servicio tiene su tipo de error (`thiserror`) con código y mensaje, exportado a TS como enum
  tipado. La UI muestra avisos discretos para lo recuperable y un panel para lo grave.
  - Archivo ilegible: se salta a la siguiente pista y se emite `PlaybackError { track, reason }`.
  - Dispositivo de audio perdido: pausa, evento y reintento con el dispositivo por defecto.
- **Logs**: `tracing` en `~/.local/state/goymusic/logs/`, rotación diaria, se conservan 7 días.
- **Pruebas** (TDD en la lógica pura):
  - `core`: cola, shuffle, repeat, "anterior", restauración del estado.
  - `audio`: con el `AudioSink` de prueba se verifica que gapless no deja silencio entre pistas, que el seek cae en la
    muestra correcta y el resampleo. Audio de prueba generado en el propio test (senoidales WAV y FLAC); no se suben
    archivos con copyright.
  - `library`: carpetas temporales con archivos etiquetados al vuelo. Se prueban escaneo, incremental, detección de
    movidos, migraciones y orden de la búsqueda (incluidas erratas).
  - UI: Vitest + Testing Library con bindings simulados.
  - CI (GitHub Actions): `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `pnpm test`, build de la app.
- **Empaquetado**: AppImage y `.deb` con el bundler de Tauri.

## 10. Criterios de aceptación de la v1

1. Añadir una carpeta de ~10 000 pistas: el primer escaneo termina y un reescaneo sin cambios tarda < 5 s.
2. Mover una carpeta de álbum en disco: el álbum sigue en sus playlists tras el reescaneo automático.
3. Reproducir un álbum gapless (FLAC y MP3): sin silencio entre pistas (verificado también por test).
4. Cola: añadir, quitar, reordenar; shuffle, repeat y "anterior" según la §5; la cola se restaura al reabrir.
5. Playlists: crear, renombrar, reordenar, carpetas, "Me gusta", importar y exportar M3U8.
6. Búsqueda: resultados mientras se escribe en < 50 ms con 50 000 pistas, y una errata ("radioheaf") encuentra el artista.
7. Teclas multimedia y controles de GNOME funcionan; notificación al cambiar de canción; cerrar la ventana no corta la música.
8. CI en verde; AppImage y `.deb` instalables.

## 11. Fuera de la v1

Modo servidor; Opus; crossfade; ecualizador; letras; importador de playlists de Spotify (lectura por la API oficial,
sin reproducción); Windows y macOS; Flatpak; smoke E2E con tauri-driver.
