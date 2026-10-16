# GoyMusic P3a — Servicios, vigilancia, carátulas y app Tauri: plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unir biblioteca y motor de audio en una capa de servicios independiente de Tauri (con persistencia de la cola, historial, vigilancia de carpetas y carátulas) y exponerla en una app Tauri 2 con tipos TS generados y la capa de datos de la UI conectada a los eventos del núcleo.

**Architecture:** Nuevo crate `goymusic-services` con la fachada `Services`, que no conoce Tauri; el futuro `apps/server` la reutilizará tal cual. Usa varias conexiones SQLite en modo WAL:
- una para las órdenes de la UI;
- una propia del hilo de escaneo;
- una propia del resolver del hilo de audio.

Así, un escaneo largo nunca bloquea la reproducción ni la UI. Un hilo coordinador escucha el `EventBus` para guardar la cola y el historial en cada cambio de pista. Un hilo de escaneo agrupa las peticiones que llegan de la UI, del watcher (notify, debounce de 2 s) o del sondeo de respaldo (cada 10 min). `apps/desktop` es un adaptador fino: comandos `#[tauri::command]` sobre `Services`, reenvío de eventos y protocolo `art://`; tauri-specta genera `ui/src/bindings.ts`. `ui/` es React + Vite con la capa de datos de la §7:
- **Zustand** para el reproductor;
- **TanStack Query** invalidado por eventos;
- un **shell de desarrollo** sin diseño para probar de punta a punta.

El diseño visual y las vistas definitivas son P3b, con Hallmark.

**Tech Stack:** Rust (edition 2021; rustc local 1.99), Tauri 2.12.1, tauri-specta/specta 2.0.0-rc.25, notify 8.2.0 + notify-debouncer-mini 0.7.0, image 0.25.10, tracing. UI: React 19.3, Vite 8.3, TypeScript 7.0, Vitest 5.0, TanStack Query 5.104, Zustand 5.0, pnpm 11 y Node 24 (vía nvm).

**Spec:** `docs/superpowers/specs/2026-10-16-goymusic-design.md`. Secciones: §3 arquitectura y eventos, §4 servicios, §5 cola y restauración, §6 vigilancia, carátulas e historial, §7 estado de la UI, §9 errores, logs y CI, §10 criterios 1, 2 y 4.

**Hoja de ruta:**
- **P3b:** identidad visual con Hallmark, rutas y vistas de la §7 (barra lateral, vista principal, barra de reproducción, panel de cola, menús, arrastrar y soltar, atajos, temas), que sustituyen al shell de desarrollo.
- **P4:** MPRIS, notificaciones, bandeja, instancia única, mini-reproductor y empaquetado.

## Global Constraints

**Dependencias y versiones**
- Versiones fijadas con `=`:
  - Rust: `notify =8.2.0`, `notify-debouncer-mini =0.7.0`, `image =0.25.10` (`default-features = false`, features `png`, `jpeg`, `webp`), `tauri =2.12.1`, `tauri-build =2.7.1`, `tauri-plugin-dialog =2.8.1`, `tauri-specta =2.0.0-rc.25` (features `derive`, `typescript`), `specta =2.0.0-rc.25` (feature `derive`), `specta-typescript =0.0.12`, `tracing =0.1.44`, `tracing-subscriber =0.3.23` (features `fmt`, `env-filter`), `tracing-appender =0.2.5`, `dirs =7.0.0`.
  - npm (exactas, sin `^`): `@tauri-apps/cli 2.12.1`, `@tauri-apps/api 2.12.1`, `@tauri-apps/plugin-dialog 2.8.1`, `react`/`react-dom 19.3.0`, `@types/react`/`@types/react-dom 19.3.0`, `vite 8.3.2`, `@vitejs/plugin-react 6.1.1`, `typescript 7.0.2`, `vitest 5.0.3`, `jsdom 30.1.1`, `@testing-library/react 16.3.3`, `@testing-library/dom 10.4.2`, `@tanstack/react-query 5.104.0`, `zustand 5.0.15`.
  - Si una API no cuadra, consultar la documentación de **esa** versión (docs.rs, o el código fuente del crate descargado); no cambiar la versión.
- specta rc.25 prohíbe exportar `i64`/`u64` a TS. Se activa `Builder::dangerously_cast_bigints_to_number()`: los IDs son rowids de SQLite y las duraciones en ms, siempre < 2^53.
- `core` y `library` derivan `specta::Type` solo con su feature opcional `specta`, así que ninguno depende de Tauri. Solo `apps/desktop` depende de `tauri*`.

**Requisitos de sistema**
- Antes de la Task 7, Javi debe instalar (sudo, con `!` o en su terminal):
  ```
  sudo apt install libwebkit2gtk-4.1-dev libsoup-3.0-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev
  ```
  Esta es una parada obligatoria: nunca ejecutar sudo sin TTY.
- Node y pnpm se cargan con `source ~/.nvm/nvm.sh` en cada comando de shell no interactivo (`npm` es un alias de pnpm en la shell de Javi; usar siempre `pnpm`).

**Umbrales y rutas de la spec**
- Debounce del watcher: 2 s. Reescaneo de respaldo: cada 10 min. Miniaturas: 200 y 600 px, WebP.
- Base de datos: `~/.local/share/goymusic/library.db`. Caché de carátulas: `~/.cache/goymusic/art/`. Logs: `~/.local/state/goymusic/logs/`, rotación diaria, 7 ficheros.
- Los tests usan duraciones cortas a través de `ServicesConfig`.
- Los eventos que se reenvían son exactamente los del enum `Event` del núcleo; la UI nunca hace polling.

**Proceso**
- Al terminar cada tarea deben pasar:
  ```
  cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
  ```
  A partir de la Task 8 también deben pasar `pnpm --dir ui test` y `pnpm --dir ui build`.
- Commits en español. Nada de código, esquemas ni nombres de Spotify. Sin audio con copyright: los tests generan WAV, y las carátulas de prueba son PNG generados con `image`.
- Flujo de commits: el hook git-safety crea commits `checkpoint:` tras cada edición. Al cerrar cada tarea: `git reset --soft <commit base de la tarea>` y un único commit con el mensaje del paso final.

## Review Focus

1. **Escaneo largo en curso** (primer escaneo de miles de pistas) mientras se reproduce o se da "Me gusta": la música no se corta y la orden de la UI no falla con "database is locked". Las conexiones van separadas, con `busy_timeout`. Tests:
   - Task 2: `second_writer_waits_instead_of_failing`.
   - Task 4: `likes_work_while_a_scan_holds_the_write_lock`.
2. **Cola restaurada cuya pista actual ya no existe** (borrada o fuera de la biblioteca): no hay pánico, y la app arranca en pausa o en Idle sin quedarse colgada. Test en la Task 5: `restore_survives_a_track_that_left_the_library`.
3. **Mover en disco la carpeta de un álbum** (criterio 2 de la §10): el watcher reescanea solo y el álbum sigue en sus playlists. Test en la Task 6: `moved_album_folder_stays_in_its_playlist`.
4. **Copiar un álbum entero a una carpeta vigilada** (ráfaga de eventos): un solo escaneo, no uno por archivo. Test en la Task 6: `a_burst_of_changes_triggers_one_scan`.
5. **Cierre brusco de la app** (kill, cuelgue o corte de luz) con música sonando: al reabrir se recupera la cola de la última pista que empezó a sonar, porque la cola se guarda en cada cambio de pista y no solo al salir. Test en la Task 5: `track_change_saves_the_queue_without_shutdown`.

## Fuera de P3a (diferido, con su motivo)

- **Minors de P1:**
  - el M3U exportado no escapa `\n` en los títulos;
  - deunicode y `remove_diacritics` difieren con ß y æ;
  - `mtime = 0` si `modified()` falla;
  - el import ignora rutas relativas que empiezan por `#`.

  Ninguno bloquea el uso diario. Se agrupan en una tarea de endurecimiento de P4.
- **Minors de P2:**
  - el timeout de `clear()` en cpal;
  - posible alloc en el callback;
  - posición adelantada por la latencia;
  - E/S de apertura en el hilo de audio;
  - apertura espuria de `resume_at`;
  - tests del reintento a 2 s y del error a mitad de pista.

  Son de audio; también van a P4.
- Filtro de `tracks(filtro, …)` (§4): la búsqueda cubre el filtrado. Aquí solo se implementan orden y paginación.
- Ajustes visibles, avisos y el panel de errores: P3b. P3a ya expone los datos (`settings`, `PlaybackError`, `AudioDeviceLost`).

---

## Mapa de ficheros

```
Cargo.toml                              + crates/services, apps/desktop; nuevas deps del workspace
package.json  pnpm-workspace.yaml       raíz: CLI de Tauri y workspace pnpm (ui)
.github/workflows/ci.yml                + deps de Tauri, pnpm, tests y build de la UI, build de la app
crates/core/
  Cargo.toml                            + feature specta
  src/{ids,model,events}.rs             camelCase en serde (model) + derive specta opcional
  src/queue.rs                          derive specta opcional en RepeatMode
crates/library/
  Cargo.toml                            + image, feature specta
  src/db.rs                             busy_timeout
  src/queries.rs                        + artist, artist_tracks
  src/history.rs                        (nuevo) record_play, recent_albums
  src/art.rs                            (nuevo) carátulas: fuente, miniatura WebP, caché
  src/{search,m3u,queries}.rs           serde/specta en tipos públicos
crates/services/                        (nuevo) goymusic-services
  src/lib.rs                            Services: arranque, apagado, acceso a eventos
  src/config.rs                         ServicesConfig (rutas y tiempos)
  src/error.rs                          ServiceError { code, message }
  src/scan.rs                           hilo de escaneo con peticiones agrupadas
  src/library.rs                        servicio Library + Search + carátulas
  src/playlists.rs                      servicio Playlists (+ eventos)
  src/player.rs                         servicio Player: contextos, vista, ajustes, persistencia
  src/coordinator.rs                    hilo que guarda cola e historial en cada cambio de pista
  src/watch.rs                          watcher notify + respaldo por sondeo
  tests/common/mod.rs                   WAV etiquetados, sink de captura, espera de eventos
  tests/{library,playlists,player,watch}_service.rs
apps/desktop/                           (nuevo) goymusic-desktop, la app Tauri
  Cargo.toml build.rs tauri.conf.json capabilities/default.json icons/
  src/main.rs src/lib.rs                arranque, estado, specta builder, salida
  src/commands.rs                       comandos tipados → Services
  src/forward.rs                        EventBus → evento Tauri CoreEvent
  src/art_protocol.rs                   art://localhost/album/{id}/{200|600}
  src/logging.rs                        tracing a fichero con rotación
  tests/bindings.rs                     ui/src/bindings.ts está al día
ui/                                     (nuevo) React + Vite + Vitest
  package.json vite.config.ts tsconfig.json index.html
  src/bindings.ts                       GENERADO por tauri-specta (no editar a mano)
  src/core/{result,queries,player-store,events}.ts
  src/App.tsx src/main.tsx              shell de desarrollo (provisional, lo sustituye P3b)
  src/core/*.test.ts  src/App.test.tsx
```

---

### Task 1: Núcleo — forma JSON para TS y tipos exportables

**Files:**
- Modify: `crates/core/Cargo.toml`, `crates/core/src/ids.rs`, `crates/core/src/model.rs`, `crates/core/src/events.rs`, `crates/core/src/queue.rs` (solo `RepeatMode`), `Cargo.toml` (workspace dep `specta`)
- Test: `crates/core/tests/serde_shape.rs`

**Interfaces:**
- Consumes: tipos de P1 y P2.
- Produces:
  - feature `specta` en `goymusic-core`. Con ella, `ids::*`, `model::*`, `events::{Event, PlaybackState}` y `queue::RepeatMode` derivan `specta::Type`.
  - Los structs de `model` serializan con `#[serde(rename_all = "camelCase")]`. Ningún dato persistido usa estos structs: `queue_state` guarda `Queue`, que no cambia.

- [ ] **Step 1: Write the failing test** `crates/core/tests/serde_shape.rs`

```rust
#[test]
fn model_serializes_in_camel_case() {
    let track = Track { id: TrackId(7), path: "/m/a.flac".into(), title: "A".into(), artists: vec!["X".into()],
        album: Some("Al".into()), album_id: Some(AlbumId(3)), album_artist: None, track_no: Some(1),
        disc_no: None, year: Some(2001), genre: None, duration_ms: 1234 };
    let v = serde_json::to_value(&track).unwrap();
    assert_eq!(v["durationMs"], 1234);
    assert_eq!(v["albumId"], 3);
    assert_eq!(v["trackNo"], 1);
    assert_eq!(v["id"], 7); // ids stay plain numbers
    let p = serde_json::to_value(Playlist { id: PlaylistId(1), name: "p".into(), folder_id: None, track_count: 2 }).unwrap();
    assert_eq!(p["trackCount"], 2);
    assert!(p.get("folderId").is_some());
}

#[test]
fn saved_queues_keep_their_format() {
    // queue_state rows written before this change must still load
    let q: Queue = serde_json::from_str(&serde_json::to_string(&Queue::new()).unwrap()).unwrap();
    assert_eq!(q, Queue::new());
    assert_eq!(serde_json::to_value(RepeatMode::All).unwrap(), "all");
}
```

- [ ] **Step 2: Run** `cargo test -p goymusic-core --test serde_shape`. Expected: FAIL (`durationMs` is null).

- [ ] **Step 3: Implement**
  - Add `#[serde(rename_all = "camelCase")]` to every struct in `model.rs`.
  - Add workspace dep `specta = { version = "=2.0.0-rc.25", features = ["derive"] }`.
  - In `crates/core/Cargo.toml`: `specta = { workspace = true, optional = true }`, `[features] specta = ["dep:specta"]`.
  - Add `#[cfg_attr(feature = "specta", derive(specta::Type))]`:
    - in the `id_types!` macro;
    - on every model struct;
    - on `PlaybackState`, `Event` and `RepeatMode`.

- [ ] **Step 4: Run** `cargo test -p goymusic-core && cargo check -p goymusic-core --features specta`. Expected: PASS, and it compiles with the feature.

- [ ] **Step 5: Commit**

```bash
git commit -m "Núcleo: JSON en camelCase para la UI y tipos exportables con specta (feature opcional)"
```

---

### Task 2: Biblioteca — concurrencia, artista, historial y tipos públicos serializables

**Files:**
- Modify:
  - `crates/library/src/db.rs` (`busy_timeout`)
  - `crates/library/src/queries.rs` (+ `artist`, `artist_tracks`; serde y specta en `TrackSort`)
  - `crates/library/src/search.rs` (serde y specta en `SearchResults`, `TopResult`)
  - `crates/library/src/m3u.rs` (`ImportReport`: `Clone`, `Serialize`, specta)
  - `crates/library/src/lib.rs` (+ `pub mod history;`)
  - `crates/library/Cargo.toml` (feature `specta = ["dep:specta", "goymusic-core/specta"]`, + `serde`)
- Create: `crates/library/src/history.rs`
- Test: `crates/library/tests/db.rs`, `tests/queries.rs`, `tests/history.rs` (nuevo), `tests/search.rs`

**Interfaces:**
- Consumes: `Db`, `TRACK_SELECT`/`track_from_row`, `album_from_row` (P1).
- Produces:
  - `Db::open` and `open_in_memory` set `busy_timeout` to 5 000 ms.
  - `queries::artist(db: &Db, id: ArtistId) -> Result<Artist>`. Returns `NotFound { kind: "artist" }` if missing.
  - `queries::artist_tracks(db: &Db, id: ArtistId) -> Result<Vec<Track>>`: tracks where the artist appears in `track_artists`, ordered by album year, album title, disc (`coalesce(disc_no, 1)`), track_no, id.
  - `history::record_play(db: &Db, track: TrackId, played_at: i64) -> Result<()>`.
  - `history::recent_albums(db: &Db, limit: u32) -> Result<Vec<Album>>`: distinct albums of played tracks, most recent play first.
  - `TrackSort`: `Deserialize` + `Serialize`, `rename_all = "camelCase"`.
  - `SearchResults`: `Serialize`, camelCase.
  - `TopResult`: `Serialize`, `#[serde(tag = "kind", content = "item", rename_all = "camelCase")]`.
  - `ImportReport`: `Clone + Serialize`, camelCase.
  - With feature `specta`, all of the above derive `specta::Type`.

- [ ] **Step 1: Write the failing tests**

```rust
// tests/db.rs: two connections to the same file
#[test]
fn second_writer_waits_instead_of_failing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lib.db");
    let mut a = Db::open(&path).unwrap();
    let b = Db::open(&path).unwrap();
    let tx = a.conn_mut().transaction().unwrap();
    tx.execute("INSERT INTO settings (key, value) VALUES ('k', '1')", []).unwrap();
    let writer = std::thread::spawn(move || state::set_setting(&b, "other", "2"));
    std::thread::sleep(std::time::Duration::from_millis(300));
    tx.commit().unwrap();
    writer.join().unwrap().expect("waited for the lock instead of SQLITE_BUSY");
}

// tests/queries.rs
#[test]
fn artist_lookup_and_tracks_in_album_order() { /* 2 albums by "Ana" (years 2001, 1999) + 1 track by "Bea";
   artist(id of Ana).name == "Ana"; artist_tracks(Ana) titles are the 1999 album's tracks by track_no, then the 2001 album's;
   artist(ArtistId(999)) is Err(NotFound { kind: "artist", .. }) */ }

// tests/history.rs
#[test]
fn recent_albums_lists_most_recently_played_first_without_duplicates() {
    /* albums A, B; record_play(a1, 10), record_play(b1, 20), record_play(a2, 30)
       recent_albums(10) == [A, B]; recent_albums(1) == [A] */
}

// tests/search.rs
#[test]
fn search_results_serialize_for_the_ui() {
    /* search for an artist name; v = serde_json::to_value(results)
       v["top"]["kind"] == "artist" && v["top"]["item"]["name"] == the name; v["tracks"] is an array */
}
```

- [ ] **Step 2: Run** `cargo test -p goymusic-library`. Expected: FAIL. `second_writer_waits…` fails with `database is locked`; the rest do not compile.

- [ ] **Step 3: Implement** the signatures in Produces.
  - `busy_timeout`: call `conn.busy_timeout(Duration::from_millis(5000))` in `Db::init`.
  - `recent_albums`: `GROUP BY album_id ORDER BY max(played_at) DESC LIMIT ?`, joined to `ALBUM_SELECT`. Make `ALBUM_SELECT` and `album_from_row` `pub(crate)`.

- [ ] **Step 4: Run** `cargo test -p goymusic-library && cargo check -p goymusic-library --features specta`. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -m "Biblioteca: busy_timeout, artista y sus pistas, historial de escucha y tipos serializables"
```

---

### Task 3: Biblioteca — carátulas y miniaturas WebP

**Files:**
- Create: `crates/library/src/art.rs`
- Modify: `crates/library/src/lib.rs` (`pub mod art;`), `crates/library/Cargo.toml` (+ `image`), `Cargo.toml` (workspace dep `image`)
- Test: `crates/library/tests/art.rs`; `crates/library/tests/common/mod.rs` (+ `write_png(path, w, h)`, + `embed_cover(audio_path, png_bytes)`)

**Interfaces:**
- Consumes: `Db`, `queries::album_tracks`, lofty 0.21 (`TaggedFileExt::primary_tag`/`tags`, `Tag::pictures`, `PictureType::CoverFront`).
- Produces:
  - `pub enum ArtSize { Small, Large }` with `fn px(self) -> u32` (200 and 600) and `fn from_px(px: u32) -> Option<ArtSize>`.
  - `pub fn album_art(db: &Db, album: AlbumId, size: ArtSize, cache_dir: &Path) -> Result<Option<PathBuf>>`. Returns `Ok(None)` if the album has no art, or if its image cannot be decoded.

**Rules (spec §6):**
- **Source:**
  - the embedded picture of the album's first track in `album_tracks` order: `CoverFront` if there is one, otherwise the first picture;
  - otherwise, in that track's directory, `cover.*` then `folder.*` with extension `jpg`, `jpeg`, `png` or `webp`, case-insensitive.
- **Cache:** `{cache_dir}/{album_id}-{px}.webp`. Regenerate if it is missing or older than the source's mtime (the audio file for embedded art, the image file for folder art). Create `cache_dir` if it does not exist.
- **Resize:** fit within px×px keeping the aspect ratio, `FilterType::Lanczos3`, and never upscale. Encode as WebP with `image::codecs::webp::WebPEncoder::new_lossless`.
- Write to `{name}.tmp` and `rename`, so a concurrent request never reads a half-written file.

- [ ] **Step 1: Write the failing tests** `crates/library/tests/art.rs`

```rust
#[test] fn embedded_cover_becomes_a_webp_thumbnail()      // 400×300 PNG embedded → Small is 200×150; image::open(path) has format WebP
#[test] fn folder_cover_is_used_without_embedded_art()    // "Cover.JPG" next to the files → Some, Large is ≤ 600 and keeps aspect ratio
#[test] fn small_images_are_not_upscaled()                // 100×100 source → Large is 100×100
#[test] fn album_without_art_has_none()                   // Ok(None)
#[test] fn unreadable_cover_is_ignored()                  // cover.png with garbage bytes → Ok(None), not Err
#[test] fn cache_is_reused_then_refreshed_when_the_source_changes()
    // 1st call creates the file; a 2nd call does not rewrite it (same mtime);
    // rewriting cover.png with a newer mtime (filetime or sleep 1.1 s) regenerates it (new mtime, new size)
```

- [ ] **Step 2: Run** `cargo test -p goymusic-library --test art`. Expected: FAIL (module `art` does not exist).

- [ ] **Step 3: Implement** `art.rs` following the Rules. Add `image = { version = "=0.25.10", default-features = false, features = ["png", "jpeg", "webp"] }` to the workspace.

- [ ] **Step 4: Run** `cargo test -p goymusic-library`. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -m "Biblioteca: carátulas embebidas o de carpeta con miniaturas WebP en caché"
```

---

### Task 4: Servicios — crate, errores, escaneo en segundo plano, Library, Search y Playlists

**Files:**
- Create: `crates/services/Cargo.toml`, `src/lib.rs`, `src/config.rs`, `src/error.rs`, `src/scan.rs`, `src/library.rs`, `src/playlists.rs`, `src/player.rs` (solo el arranque del `Player`; la Task 5 lo completa)
- Modify: `Cargo.toml` (+ `crates/services` en members)
- Test: `crates/services/tests/common/mod.rs`, `tests/library_service.rs`, `tests/playlists_service.rs`

**Interfaces:**
- Consumes: `goymusic-library` (folders, scanner::scan, queries, search, playlists, m3u, art, history, state), `goymusic-audio` (`Player::spawn`, `SinkFactory`, `Resolver`), `goymusic-core` (`EventBus`, `Event`).
- Produces (all in `goymusic_services`). `Services` is `Send + Sync`, and the app shares it as `Arc<Services>`.

```rust
pub struct ServicesConfig { pub db_path: PathBuf, pub art_dir: PathBuf, pub debounce: Duration, pub poll_every: Duration }
impl ServicesConfig {
    pub fn default_paths() -> Self;        // ~/.local/share/goymusic/library.db, ~/.cache/goymusic/art, 2 s, 600 s (via `dirs`)
    pub fn in_dir(dir: &Path) -> Self;     // tests: dir/library.db, dir/art, 200 ms, 600 s
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, thiserror::Error)] #[serde(rename_all = "camelCase")]
#[error("{code:?}: {message}")]
pub struct ServiceError { pub code: ErrorCode, pub message: String }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)] #[serde(rename_all = "camelCase")]
pub enum ErrorCode { NotFound, InvalidInput, FolderOverlap, Io, Database, Internal }
impl From<LibraryError> for ServiceError   // NotFound→NotFound, InvalidPosition→InvalidInput, FolderOverlap→FolderOverlap,
                                           // Io/NonUtf8Path→Io, Db/Migration/Json/Tags→Database
pub type Result<T> = std::result::Result<T, ServiceError>;

impl Services {
    pub fn start(config: ServicesConfig, sink: SinkFactory) -> Result<Services>;
    pub fn events(&self) -> &Arc<EventBus>;
    pub fn wait_idle(&self, timeout: Duration) -> bool;     // true once no scan is running or queued
    pub fn shutdown(&self);                                 // idempotent; completed in Task 5
    // Library (library.rs)
    pub fn folders(&self) -> Result<Vec<Folder>>;
    pub fn add_folder(&self, path: &Path) -> Result<Folder>;   // queues a scan; returns without waiting
    pub fn remove_folder(&self, id: FolderId) -> Result<()>;   // publishes LibraryUpdated
    pub fn rescan(&self);
    pub fn albums(&self) -> Result<Vec<Album>>;
    pub fn album(&self, id: AlbumId) -> Result<AlbumDetail>;
    pub fn artists(&self) -> Result<Vec<Artist>>;
    pub fn artist(&self, id: ArtistId) -> Result<ArtistDetail>;
    pub fn tracks(&self, sort: TrackSort, offset: u32, limit: u32) -> Result<Vec<Track>>;
    pub fn track(&self, id: TrackId) -> Result<Track>;
    pub fn recent_albums(&self, limit: u32) -> Result<Vec<Album>>;
    pub fn search(&self, query: &str, limit: u32) -> Result<SearchResults>;
    pub fn album_art(&self, id: AlbumId, size: ArtSize) -> Result<Option<PathBuf>>;
    // Playlists (playlists.rs): each mutation publishes PlaylistsChanged; like/unlike publish LikesChanged
    pub fn playlists(&self) -> Result<Vec<Playlist>>;
    pub fn playlist_folders(&self) -> Result<Vec<PlaylistFolder>>;
    pub fn playlist(&self, id: PlaylistId) -> Result<PlaylistDetail>;
    pub fn create_playlist(&self, name: &str) -> Result<Playlist>;
    pub fn rename_playlist(&self, id: PlaylistId, name: &str) -> Result<()>;
    pub fn delete_playlist(&self, id: PlaylistId) -> Result<()>;
    pub fn add_to_playlist(&self, id: PlaylistId, tracks: &[TrackId]) -> Result<()>;
    pub fn remove_from_playlist(&self, id: PlaylistId, positions: &[u32]) -> Result<()>;
    pub fn move_playlist_item(&self, id: PlaylistId, from: u32, to: u32) -> Result<()>;
    pub fn create_playlist_folder(&self, name: &str, parent: Option<PlaylistFolderId>) -> Result<PlaylistFolder>;
    pub fn move_playlist_to_folder(&self, id: PlaylistId, folder: Option<PlaylistFolderId>) -> Result<()>;
    pub fn like(&self, track: TrackId) -> Result<()>;
    pub fn unlike(&self, track: TrackId) -> Result<()>;
    pub fn liked_tracks(&self) -> Result<Vec<Track>>;
    pub fn liked_ids(&self) -> Result<Vec<TrackId>>;
    pub fn import_m3u(&self, path: &Path) -> Result<ImportReport>;   // playlist name = file stem
    pub fn export_m3u(&self, id: PlaylistId, path: &Path) -> Result<()>;
}
#[derive(Clone, Debug, Serialize)] #[serde(rename_all = "camelCase")] pub struct AlbumDetail { pub album: Album, pub tracks: Vec<Track> }
#[derive(Clone, Debug, Serialize)] #[serde(rename_all = "camelCase")] pub struct ArtistDetail { pub artist: Artist, pub albums: Vec<Album>, pub tracks: Vec<Track> }
#[derive(Clone, Debug, Serialize)] #[serde(rename_all = "camelCase")] pub struct PlaylistDetail { pub playlist: Playlist, pub items: Vec<PlaylistItem> }
```
- Feature `specta = ["dep:specta", "goymusic-library/specta", "goymusic-core/specta"]` derives `specta::Type` on `ServiceError`, `ErrorCode`, `AlbumDetail`, `ArtistDetail` and `PlaylistDetail`.

**Decisions:**
- **Connections:** `ui_db: Mutex<Db>` (UI commands). The scan thread opens its own `Db`, and the resolver passed to `Player::spawn` opens another one (`Mutex<Db>` inside the closure, `queries::track(..).path`). All of them use the same `db_path`.
- **Scan worker (`scan.rs`):**
  - a thread with a `Sender<()>` channel;
  - after each scan it drains the pending requests, so N requests during a scan cause one more scan, not N;
  - `wait_idle` uses a pending/running counter with a `Condvar`;
  - a scan that returns `Err` is logged with `tracing::warn!` and never kills the thread.
- **No `Library`/`Playlists` traits:** the `Services` facade is the transport-agnostic interface (§3). The future `apps/server` will call the same struct.
- **Spec deviation (ruling, listed in the self-review):** the spec places "servicios (traits)" in `core`, and P3a does not create those traits. A concrete facade reached through Tauri commands, or through WebSocket later, is enough: there is only one implementation. Cost if wrong: extracting a trait later is mechanical.

- [ ] **Step 1: Write the test helpers** `crates/services/tests/common/mod.rs`
  - `write_tagged_wav` / `Meta`: include the library's helpers with `#[path = "../../../library/tests/common/mod.rs"] mod lib_common; pub use lib_common::*;`.
  - `capture_sink() -> SinkFactory`: a `CaptureSink` at 8 kHz mono that keeps playing.
  - `start(dir: &Path) -> Services`: uses `ServicesConfig::in_dir`.
  - `wait_for(rx: &Receiver<Event>, timeout, pred: impl Fn(&Event) -> bool) -> Event`: panics with the events seen if it times out.
  - `album_fixture(root, album, artist, n) -> Vec<PathBuf>`: n 1 s WAVs with track numbers.

- [ ] **Step 2: Write the failing tests**

```rust
// tests/library_service.rs
#[test] fn adding_a_folder_scans_it_in_the_background()      // add_folder returns at once; wait_idle(10 s); albums().len() == 1; LibraryUpdated seen
#[test] fn overlapping_folder_is_rejected_with_its_code()    // add the parent of an existing folder → Err(code FolderOverlap)
#[test] fn album_and_artist_details()                        // album(id).tracks in track order; artist(id).albums/tracks non-empty
#[test] fn unknown_ids_are_not_found()                       // album(AlbumId(999)).unwrap_err().code == ErrorCode::NotFound
#[test] fn rescan_requests_during_a_scan_are_coalesced()     // 10 rescan() calls in a row → at most 2 ScanProgress{done:0,..} events after the first
#[test] fn removing_a_folder_drops_its_tracks()              // remove_folder → tracks() empty, LibraryUpdated seen
#[test] fn search_finds_by_typo()                            // artist "Radiohead", search("radioheaf") → top is that artist
#[test] fn album_art_is_served_from_the_cache_dir()          // cover.png in the album folder → album_art(Small) is Some(path under config.art_dir)

// tests/playlists_service.rs
#[test] fn playlist_lifecycle_publishes_playlists_changed()  // create, rename, add, move, remove, folder, delete → a PlaylistsChanged for each
#[test] fn likes_publish_likes_changed_and_list_ids()        // like(t) → LikesChanged; liked_ids() == [t]; unlike → []
#[test] fn import_and_export_m3u_round_trip()                // export_m3u then import_m3u → same track ids, name = file stem
#[test] fn likes_work_while_a_scan_holds_the_write_lock()
    // open a second Db on the same file, hold an open write transaction for 1 s on another thread;
    // like(t) meanwhile → Ok after it is released (busy_timeout), not "database is locked"
```

- [ ] **Step 3: Run** `cargo test -p goymusic-services`. Expected: FAIL (crate does not compile; nothing is defined).

- [ ] **Step 4: Implement** the Interfaces and Decisions. `Services::start`:
  1. opens `ui_db`, which runs the migrations;
  2. creates the `EventBus`;
  3. starts the scan worker;
  4. spawns the `Player` with the resolver;
  5. queues an initial scan if any folder exists, so changes made while the app was closed are picked up.

- [ ] **Step 5: Run** `cargo test -p goymusic-services`. Expected: PASS. Then run the full gate from the Global Constraints.

- [ ] **Step 6: Commit**

```bash
git commit -m "Servicios: fachada Library/Search/Playlists con escaneo en segundo plano y errores tipados"
```

---

### Task 5: Servicios — Player: contextos, vista, ajustes, persistencia de la cola e historial

**Files:**
- Modify: `crates/services/src/player.rs`, `crates/services/src/lib.rs` (`start` restores state, `shutdown` saves)
- Create: `crates/services/src/coordinator.rs`
- Test: `crates/services/tests/player_service.rs`

**Interfaces:**
- Consumes: `Player` (P2: `play`, `pause`, `resume`, `next`, `previous`, `seek`, `set_shuffle`, `set_repeat`, `set_volume`, `set_replaygain`, `queue_add`, `queue_remove`, `queue_move`, `restore`, `state() -> Option<PlayerSnapshot>`), `Queue::{current, upcoming, user_queue, shuffle, repeat}`, `state::{save_queue, load_queue, set_setting, setting}`, `history::record_play`.
- Produces:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)] #[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum PlayContext { Album(AlbumId), Artist(ArtistId), Playlist(PlaylistId), Liked, Tracks(Vec<TrackId>) }

#[derive(Clone, Debug, Serialize)] #[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub state: PlaybackState, pub track: Option<Track>, pub position_ms: u64,
    pub volume: f32, pub replaygain: bool, pub shuffle: bool, pub repeat: RepeatMode,
    pub user_queue: Vec<Track>,   // indices match queue_remove / queue_move
    pub up_next: Vec<Track>,      // rest of the context in play order, at most UP_NEXT_LIMIT = 100
}
#[derive(Clone, Debug, Serialize)] #[serde(rename_all = "camelCase")]
pub struct Settings { pub replaygain: bool, pub watch: WatchStatus }   // `watch` is added in Task 6; until then Settings has only replaygain

impl Services {
    pub fn play(&self, context: PlayContext, start_index: u32) -> Result<()>;  // empty context or index ≥ len → InvalidInput
    pub fn pause(&self); pub fn resume(&self); pub fn next(&self); pub fn previous(&self);
    pub fn seek(&self, position_ms: u64);
    pub fn set_shuffle(&self, on: bool); pub fn set_repeat(&self, mode: RepeatMode);
    pub fn set_volume(&self, volume: f32);        // clamp 0–1, persists "player.volume"
    pub fn set_replaygain(&self, on: bool);       // persists "player.replaygain"
    pub fn queue_add(&self, tracks: Vec<TrackId>); pub fn queue_remove(&self, index: u32); pub fn queue_move(&self, from: u32, to: u32);
    pub fn player_view(&self) -> Result<PlayerView>;
    pub fn settings(&self) -> Result<Settings>;
}
```
- With feature `specta`: `PlayContext`, `PlayerView` and `Settings` derive `specta::Type`.

**Decisions:**
- **Context resolution:**
  - `Album` → `album_tracks`;
  - `Artist` → `artist_tracks`;
  - `Playlist` → items in order;
  - `Liked` → `liked_tracks`;
  - `Tracks` → as given.
- **Startup (`start`):**
  1. apply `player.volume` (default 1.0) and `player.replaygain` (default false);
  2. if `load_queue` returns a saved queue, call `player.restore(queue, position)`.
- **Coordinator (`coordinator.rs`):** a thread subscribed to the bus. On `PlayerStateChanged { track: Some(t), state: Playing }` with `t` ≠ the last track seen, it:
  1. calls `record_play(t, now)`;
  2. takes the `player.state()` snapshot;
  3. calls `save_queue(queue, position)`.

  It uses its own `Db` connection and ends when the bus is dropped or on shutdown.
- **`shutdown`:** saves `save_queue(snapshot.queue, snapshot.position_ms)`, stops the coordinator, the scan worker and the watcher, then drops the `Player`. `Services::drop` calls `shutdown`. A second call does nothing.

- [ ] **Step 1: Write the failing tests** `crates/services/tests/player_service.rs`
  - Use an album of 3 × 1 s WAVs and the capture sink.
  - "Wait for X" means `wait_for` with a 5 s timeout.

```rust
#[test] fn playing_an_album_starts_its_first_track()           // play(Album(id), 0) → PlayerStateChanged{Playing, Some(track 1)}
#[test] fn every_context_kind_resolves()                        // Artist, Playlist, Liked, Tracks(vec![t3]) each start the expected track
#[test] fn invalid_contexts_are_rejected()                      // empty Liked → InvalidInput; Album start_index 3 → InvalidInput
#[test] fn player_view_shows_track_queue_and_settings()         // play(Album, 0); queue_add([t3]) → view.track == t1, user_queue == [t3], up_next == [t2, t3]
#[test] fn queue_survives_a_restart_paused_at_its_position()
    // play, seek(1500), pause, shutdown; start again on the same dir →
    // view.state == Paused, track == the same, position_ms == 1500, up_next unchanged
#[test] fn track_change_saves_the_queue_without_shutdown()
    // play(Album, 0); next() → wait PlayerStateChanged(track 2); then mem::forget the Services
    // (no shutdown, simulating a kill); load_queue on a fresh Db → current == track 2
#[test] fn volume_and_replaygain_survive_a_restart()            // set_volume(0.4), set_replaygain(true), shutdown, start → view.volume == 0.4, replaygain
#[test] fn track_changes_are_recorded_in_history()              // play(Album, 0) → after Playing, recent_albums(5) == [the album]
#[test] fn restore_survives_a_track_that_left_the_library()
    // play(Album, 1), pause, shutdown; delete the file of track 2 + start (initial scan removes it) →
    // no panic; within 5 s view.state ∈ {Paused, Idle, Ended}; player_view() is Ok
```

- [ ] **Step 2: Run** `cargo test -p goymusic-services --test player_service`. Expected: FAIL (`play`, `PlayContext` and `player_view` do not exist).

- [ ] **Step 3: Implement** the Interfaces and Decisions.

- [ ] **Step 4: Run** `cargo test -p goymusic-services`. Expected: PASS. Then run the full gate.

- [ ] **Step 5: Commit**

```bash
git commit -m "Servicios: Player con contextos, vista, ajustes persistidos, cola restaurada en pausa e historial"
```

---

### Task 6: Servicios — vigilancia de carpetas con respaldo por sondeo

**Files:**
- Create: `crates/services/src/watch.rs`
- Modify:
  - `crates/services/src/lib.rs`: `start` watches every folder; `shutdown` stops the watcher.
  - `crates/services/src/library.rs`: `add_folder` and `remove_folder` update the watched paths.
  - `crates/services/src/player.rs`: `Settings.watch`.
  - `Cargo.toml`: workspace deps `notify` and `notify-debouncer-mini`.
- Test: `crates/services/tests/watch_service.rs`

**Interfaces:**
- Consumes: the scan worker's request `Sender<()>` (Task 4), `ServicesConfig::{debounce, poll_every}`.
- Produces:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize)] #[serde(tag = "mode", rename_all = "camelCase")]
pub enum WatchStatus { Live, Polling { reason: String } }   // + specta::Type under the feature
impl Services { pub fn watch_status(&self) -> WatchStatus; }
```

**Decisions:**
- `notify_debouncer_mini::new_debouncer(config.debounce, handler)`, with each folder watched with `RecursiveMode::Recursive`. Every debounced batch, of any size, sends **one** scan request.
- A full incremental scan, not just the affected folder. Reason: move detection pairs a missing file with a new one across **all** folders. Scanning only one folder would delete the "missing" file before seeing its new location, and playlists would lose it (criterion 2). An unchanged rescan costs < 5 s (criterion 1). This is recorded as a ruling against "reescaneo de las carpetas afectadas" (§6).
- **Fallback:** if `watch()` on any folder fails for any reason (inotify limit `MaxFilesWatch`, folder missing or unreadable), the status becomes `Polling { reason: error.to_string() }`. A thread then sends a scan request every `poll_every` until shutdown. The folders that could be watched keep their watch. The status returns to `Live` only after a restart in which every watch succeeds.
- Our own writes to the music folders (exporting an M3U there) trigger a harmless rescan.

- [ ] **Step 1: Write the failing tests** `crates/services/tests/watch_service.rs`
  - Config: `in_dir`, debounce 200 ms, `poll_every` 300 ms in the fallback test.
  - Timeouts: 10 s.

```rust
#[test] fn a_new_file_is_indexed_without_asking()         // add_folder, wait_idle; write a new WAV → wait LibraryUpdated; tracks() has it
#[test] fn a_burst_of_changes_triggers_one_scan()         // after the first scan, write 20 WAVs within ~100 ms → exactly 1 ScanProgress{done:0} in the next 2 s
#[test] fn moved_album_folder_stays_in_its_playlist()     // criterion 2: playlist with the album's 3 tracks; rename root/AlbumA → root/Moved/AlbumA;
                                                          // wait LibraryUpdated → playlist(id).items has the same 3 TrackIds, paths under Moved/
#[test] fn an_unwatchable_folder_falls_back_to_polling()  // add folder, shutdown, rename the folder away, start → watch_status() is Polling{..};
                                                          // ≥ 2 ScanProgress{done:0} within 2 s (polling every 300 ms)
#[test] fn removing_a_folder_stops_watching_it()          // remove_folder, then write a WAV into it → no LibraryUpdated within 1 s
#[test] fn settings_report_the_watch_status()             // settings().watch == WatchStatus::Live in a normal start
```

- [ ] **Step 2: Run** `cargo test -p goymusic-services --test watch_service`. Expected: FAIL (`watch_status` does not exist; no automatic scan).

- [ ] **Step 3: Implement** the Interfaces and Decisions.

- [ ] **Step 4: Run** `cargo test -p goymusic-services`. Expected: PASS. Then run the full gate. Run the watch tests 3 times in a row to rule out timing flakiness: `for i in 1 2 3; do cargo test -p goymusic-services --test watch_service || break; done`.

- [ ] **Step 5: Commit**

```bash
git commit -m "Servicios: vigilancia de carpetas con debounce y respaldo por sondeo; mover un álbum conserva sus playlists"
```

---

### Task 7: App Tauri — adaptador, tipos TS generados, protocolo art:// y logs

**PARADA antes de empezar:** comprobar `pkg-config --exists webkit2gtk-4.1 libsoup-3.0 gtk+-3.0 && echo ok`. Si no imprime `ok`, pedir a Javi el `apt install` de los Global Constraints y esperar.

**Files:**
- Create:
  - `apps/desktop/Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/default.json`
  - `icons/` (generados con `pnpm tauri icon` desde un PNG provisional de 1024 px de un solo color, sustituido en P3b)
  - `src/main.rs`, `src/lib.rs`, `src/commands.rs`, `src/forward.rs`, `src/art_protocol.rs`, `src/logging.rs`, `tests/bindings.rs`
  - `package.json` y `pnpm-workspace.yaml` (raíz)
  - el esqueleto mínimo de `ui/` para que exista un frontend que compilar: `package.json`, `index.html`, `vite.config.ts`, `tsconfig.json`, `src/main.tsx` con un `<h1>GoyMusic</h1>`
- Modify: `Cargo.toml` (+ `apps/desktop` en members; deps de tauri), `.gitignore` (+ `apps/desktop/gen/`)

**Interfaces:**
- Consumes: everything `Services` exposes (Tasks 4–6) with feature `specta`.
- Produces:
  - `goymusic_desktop::run()`;
  - `goymusic_desktop::specta_builder() -> tauri_specta::Builder<tauri::Wry>`;
  - `#[derive(Clone, Serialize, Type, tauri_specta::Event)] pub struct CoreEvent(pub Event)`;
  - `art_protocol::parse(path: &str) -> Option<(AlbumId, ArtSize)>`;
  - the file `ui/src/bindings.ts`.

  The UI consumes `commands.*` (tauri-specta turns names into camelCase: `commands.addFolder`, `commands.playerView`…) and `events.coreEvent`.

**Decisions:**
- **Commands:** one per `Services` method from Tasks 4–6, except `start`/`shutdown`/`events`/`wait_idle`/`album_art`, with the same names in snake_case.
  - Each one is `#[tauri::command] #[specta::specta] async fn name(state: State<'_, Arc<Services>>, …) -> Result<T, ServiceError>`.
  - Methods without `Result` return `Result<(), ServiceError>` so the TS API is uniform.
  - Paths arrive as `String`. IDs, `u32` and `u64` are typed.
- **Builder:** `specta_builder()` registers the commands and `CoreEvent`, plus `.dangerously_cast_bigints_to_number()` and `ErrorHandlingMode::Result` (the default).
  - In debug builds, `run()` exports to `../../ui/src/bindings.ts` with `Typescript::default().header("// Generated by tauri-specta. Do not edit.\n")`.
- **Event forwarding (`forward.rs`):** a thread does `events().subscribe()` and calls `CoreEvent(e).emit(&app_handle)` for each event, until the channel closes.
- **`art_protocol.rs`:** registered with `register_asynchronous_uri_scheme_protocol("art", …)`.
  - It accepts paths `/album/{id}/{200|600}`; anything else returns 404.
  - It runs `album_art` on a `std::thread` (never on the main thread) and responds `200` with `Content-Type: image/webp` and `Cache-Control: max-age=86400`. `None` returns 404; an error returns 500.
  - The UI builds `art://localhost/album/{id}/200`.
- **`tauri.conf.json`:**
  - `productName` "GoyMusic", `identifier` "dev.goymusic.desktop";
  - `build.devUrl` `http://localhost:1420`, `frontendDist` `../../ui/dist`, `beforeDevCommand` `{ "script": "pnpm dev", "cwd": "../../ui" }`, `beforeBuildCommand` `{ "script": "pnpm build", "cwd": "../../ui" }`;
  - one window, 1280×800, min 900×600, title "GoyMusic";
  - CSP `default-src 'self'; img-src 'self' art: http://art.localhost data:; style-src 'self' 'unsafe-inline'`.
- **`capabilities/default.json`:** `core:default`, `dialog:default`.
- **`run()`:**
  1. `logging::init()`, using `tracing_appender::rolling::Builder` with `Rotation::DAILY`, `max_log_files(7)`, prefix `goymusic` and the directory `dirs::state_dir()/goymusic/logs`. The guard is kept in `State`.
  2. `Services::start(ServicesConfig::default_paths(), Box::new(|| CpalSink::open_default().map(|s| Box::new(s) as _)))`;
  3. `manage(Arc<Services>)`, `mount_events`, start `forward`, then `tauri_plugin_dialog::init()`;
  4. on `RunEvent::Exit`, `services.shutdown()`.
- **Root:**
  - `package.json`: `private`, devDependency `@tauri-apps/cli`, scripts `"tauri": "tauri"`, `"dev": "tauri dev"`;
  - `pnpm-workspace.yaml`: `packages: ["ui"]`.

- [ ] **Step 1: Write the failing tests**

```rust
// apps/desktop/tests/bindings.rs: bindings committed in the repo = bindings generated now
#[test]
fn bindings_are_up_to_date() {
    let out = tempfile::NamedTempFile::new().unwrap();
    goymusic_desktop::export_bindings(out.path()).unwrap(); // same exporter and header as run()
    let fresh = std::fs::read_to_string(out.path()).unwrap();
    let committed_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/bindings.ts");
    if std::env::var_os("UPDATE_BINDINGS").is_some() { std::fs::write(committed_path, &fresh).unwrap(); return; }
    assert_eq!(std::fs::read_to_string(committed_path).unwrap(), fresh,
        "ui/src/bindings.ts is stale: run UPDATE_BINDINGS=1 cargo test -p goymusic-desktop --test bindings");
}
```
  - `art_protocol` unit tests (in `#[cfg(test)]`):
    - `parse("/album/12/200") == Some((AlbumId(12), ArtSize::Small))`;
    - `"/album/12/600"` → `Large`;
    - `"/album/x/200"`, `"/album/12/300"`, `"/track/1/200"` and `""` → `None`.
  - Produces gains `pub fn export_bindings(path: &Path) -> Result<(), specta_typescript::Error>`, which `run()` also uses.

- [ ] **Step 2: Run** `cargo test -p goymusic-desktop`. Expected: FAIL (crate does not exist).

- [ ] **Step 3: Implement** the crate, the UI skeleton and the Decisions. Then install the JS dependencies: `source ~/.nvm/nvm.sh && pnpm install` (root and `ui`, with the exact versions).

- [ ] **Step 4: Generate and verify**
  - Run `UPDATE_BINDINGS=1 cargo test -p goymusic-desktop --test bindings`, then `cargo test -p goymusic-desktop`.
    - Expected: PASS.
    - `ui/src/bindings.ts` contains `addFolder`, `playerView` and `coreEvent`.
    - IDs, `durationMs` and `positionMs` are `number` (`grep -c bigint ui/src/bindings.ts` → 0).
  - Run `source ~/.nvm/nvm.sh && pnpm --dir ui build`. Expected: OK.
  - Run the full gate.

- [ ] **Step 5: Manual smoke test** (Javi, with a graphical session)
  - Run `source ~/.nvm/nvm.sh && pnpm dev` from the root.
  - The "GoyMusic" window opens.
  - `~/.local/state/goymusic/logs/goymusic.<fecha>` exists.
  - Closing the window ends the process without errors.

- [ ] **Step 6: Commit**

```bash
git commit -m "App Tauri: comandos tipados sobre Services, tipos TS generados, eventos, protocolo art:// y logs"
```

---

### Task 8: UI — capa de datos (Query + Zustand + eventos), shell de desarrollo y CI

**Files:**
- Create:
  - `ui/src/core/result.ts`, `ui/src/core/queries.ts`, `ui/src/core/player-store.ts`, `ui/src/core/events.ts`
  - `ui/src/App.tsx`, `ui/src/test/setup.ts`
  - tests: `ui/src/core/player-store.test.ts`, `ui/src/core/events.test.ts`, `ui/src/App.test.tsx`
- Modify: `ui/src/main.tsx` (QueryClientProvider + `connectCoreEvents`), `ui/package.json` (scripts `test`: `vitest run`; `build`: `tsc --noEmit && vite build`), `ui/vite.config.ts` (`test: { environment: "jsdom", setupFiles }`), `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `commands`, `events` and the types in `ui/src/bindings.ts` (Task 7).
- Produces:

```ts
// result.ts
export class CommandError extends Error { constructor(public error: ServiceError) }
export function unwrap<T>(r: { status: "ok"; data: T } | { status: "error"; error: ServiceError }): T  // throws CommandError

// queries.ts: query keys (prefixes invalidated by events) and hooks
export const keys = { library: ["library"], albums: ["library","albums"], album: (id: number) => ["library","album",id],
  artist: (id: number) => ["library","artist",id], folders: ["library","folders"], recent: ["library","recent"],
  search: (q: string) => ["library","search",q], playlists: ["playlists"], playlist: (id: number) => ["playlists",id],
  likes: ["likes"], settings: ["settings"] } as const;
export function useAlbums(), useFolders(), useSettings()  // useQuery + unwrap(await commands.x())

// player-store.ts: Zustand
type Notice = { id: number; kind: "playbackError" | "deviceLost"; message: string };
interface PlayerStore { view: PlayerView | null; positionMs: number; scan: { done: number; total: number } | null; notices: Notice[];
  refresh(): Promise<void>; setPosition(trackId: number, ms: number): void; setScan(done: number, total: number): void;
  notify(kind: Notice["kind"], message: string): void; dismiss(id: number): void;
  setVolume(v: number): Promise<void>;   // optimistic: updates view.volume before the command
  togglePlay(): Promise<void> }          // Playing → pause; otherwise resume
export const usePlayer: UseBoundStore<StoreApi<PlayerStore>>

// events.ts
export function handleCoreEvent(e: Event, qc: QueryClient, store: PlayerStore): void
export function connectCoreEvents(qc: QueryClient): Promise<() => void>  // events.coreEvent.listen(e => handleCoreEvent(e.payload, qc, usePlayer.getState()))
```

**Event map (spec §3 and §7). The UI never polls:**

| Event | Action |
|---|---|
| `libraryUpdated` | invalidate `keys.library` |
| `scanProgress` | `setScan(done, total)`; on `done === total`, after 1 s, `scan = null` |
| `playlistsChanged` | invalidate `keys.playlists` |
| `likesChanged` | invalidate `keys.likes` |
| `queueChanged` | `refresh()` |
| `playerStateChanged` | `refresh()` |
| `positionChanged` | `setPosition` (ignored if `track` ≠ `view.track?.id`) |
| `playbackError` | `notify("playbackError", reason)` + `refresh()` |
| `audioDeviceLost` | `notify("deviceLost", reason)` + `refresh()` |

**Development shell (`App.tsx`, provisional and unstyled, replaced by P3b):**
- Section "Carpetas": the `useFolders` list, a "Añadir carpeta" button (`@tauri-apps/plugin-dialog` `open({ directory: true })` → `commands.addFolder`) and "Escaneando… {done}/{total}" while `scan` is set.
- Section "Álbumes": `useAlbums`, each one with an `<img src={artUrl(id, 200)}>` and a button that calls `commands.play({ kind: "album", id }, 0)`.
- Bar: title and artist of `view.track`, `formatMs(positionMs)` / `formatMs(track.durationMs)`, and the buttons "Anterior", "Reproducir"/"Pausa", "Siguiente". Shows the `notices`.
- Helpers in `queries.ts`: `artUrl(id: number, px: 200 | 600) => \`art://localhost/album/${id}/${px}\`` and `formatMs(ms) => "m:ss"`.

- [ ] **Step 1: Write the failing tests** (Vitest with `vi.mock("../bindings")`)

```ts
// player-store.test.ts
test("setVolume updates the view before the command resolves")   // view.volume 1 → setVolume(0.3): getState().view.volume === 0.3 synchronously
test("togglePlay pauses while playing and resumes otherwise")
test("positionChanged for another track is ignored")
// events.test.ts
test("libraryUpdated invalidates every library query")            // qc.invalidateQueries called with { queryKey: ["library"] }
test("playerStateChanged and queueChanged refresh the player view")
test("playbackError adds a notice and refreshes")
test("scanProgress tracks progress and clears when done")         // vi.useFakeTimers; done===total → after 1000 ms scan === null
// App.test.tsx
test("shows albums and plays one on click")                       // mocked albums → click → commands.play called with ({kind:"album", id}, 0)
test("formatMs formats minutes and seconds")                      // 0 → "0:00", 61_000 → "1:01", 3_600_000 → "60:00"
```

- [ ] **Step 2: Run** `source ~/.nvm/nvm.sh && pnpm --dir ui test`. Expected: FAIL (modules do not exist).

- [ ] **Step 3: Implement** the Interfaces, the event map and the shell. Then update `ci.yml`:
  - in the `rust` job:
    - add the apt packages of the Global Constraints to the existing apt line;
    - add the steps `pnpm/action-setup@v4` (pnpm 11), `actions/setup-node@v4` (node 24, cache pnpm) and `pnpm install --frozen-lockfile`. These go **before** clippy, because `goymusic-desktop` needs `ui/dist`, so `pnpm --dir ui build` runs before `cargo clippy`.
  - final steps: `pnpm --dir ui test` and `cargo build -p goymusic-desktop`. Bundling (AppImage/.deb) is P4.

- [ ] **Step 4: Run** `source ~/.nvm/nvm.sh && pnpm --dir ui test && pnpm --dir ui build`, then the full Rust gate. Expected: everything PASS.

- [ ] **Step 5: End-to-end smoke test** (Javi): `pnpm dev`.
  1. Add `~/Música`: the scan progresses and albums with covers appear.
  2. Click an album: it plays, the position advances, and "Siguiente" and "Pausa" work.
  3. Close and reopen: the same track, paused, at the same position (criterion 4, restoration part).

- [ ] **Step 6: Commit**

```bash
git commit -m "UI: capa de datos con TanStack Query y Zustand alimentada por eventos, shell de desarrollo y CI"
```

---

## Verificación final

1. Run `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and `pnpm --dir ui test && pnpm --dir ui build`. Everything green.
2. Run `git grep -n -i spotify -- crates apps ui`. It should find nothing, except comments that cite the Developer Policy, if any.
3. Javi's end-to-end smoke test (Task 8, Step 5).

## Cobertura de la spec en P3a

| Spec | Tarea |
|---|---|
| §3 core sin Tauri; solo `apps/desktop` conoce Tauri; adaptador sustituible | 1, 4, 7 |
| §3 flujo comando → servicio → evento → UI, sin polling; los 8 eventos | 4, 5, 7, 8 |
| §4 Player (todas las órdenes + `state`) y contextos | 5 |
| §4 Library (`tracks` sin filtro: ver "Fuera de P3a"), Playlists, Search | 2, 4 |
| §5 cola guardada y restaurada en pausa | 5 |
| §6 vigilancia (debounce 2 s, respaldo 10 min + aviso en ajustes) | 6 |
| §6 carátulas, miniaturas 200/600 WebP, `art://` | 3, 7 |
| §6 `play_history` | 2, 5 |
| §7 TanStack Query invalidado por eventos; Zustand con eventos del reproductor; tipos generados con tauri-specta | 7, 8 |
| §9 errores tipados con código exportados a TS; logs con rotación diaria y 7 días; Vitest; CI con pnpm test y build | 4, 7, 8 |
| §10 criterio 2 (mover álbum) y criterio 4 (parte de restauración) | 6, 5 |
| §7 disposición, rutas, interacción, temas, diseño Hallmark | P3b |
| §8 escritorio, §9 empaquetado | P4 |
