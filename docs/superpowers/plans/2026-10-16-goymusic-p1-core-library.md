# GoyMusic P1 — Núcleo y biblioteca: plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Crates Rust `goymusic-core` (tipos, cola de reproducción, eventos) y `goymusic-library` (SQLite, escaneo incremental con detección de movidos, consultas, playlists, "Me gusta", M3U, búsqueda, estado persistido), probados con `cargo test`.

**Architecture:** Workspace de Cargo. `core` es lógica pura sin E/S ni dependencias de UI. `library` usa SQLite (rusqlite con SQLite embebido y FTS5) y expone funciones libres agrupadas por módulo (`folders`, `scanner`, `queries`, `playlists`, `m3u`, `search`, `state`). La capa de servicios con eventos y el adaptador Tauri llegan en P3.

**Tech Stack:** Rust stable (edition 2021), rusqlite 0.32 (`bundled`), rusqlite_migration 1.3, lofty 0.21, walkdir 2, rayon 1, strsim 0.11, deunicode 1, rand 0.8, serde 1, serde_json 1, thiserror 2; tests con tempfile 3 y hound 3.

**Spec:** `docs/superpowers/specs/2026-10-16-goymusic-design.md` (§3, §4, §5 lógica de cola, §6, §9).

**Hoja de ruta (planes posteriores, fuera de este plan):**
- P2: motor de audio (symphonia → rubato → cpal, gapless, `AudioSink`, ReplayGain).
- P3: servicios + bus de eventos cableados, vigilancia de carpetas (notify, debounce 2 s, respaldo 10 min), app Tauri + UI React.
- P4: MPRIS, notificaciones, bandeja, instancia única, mini-reproductor, AppImage/.deb, CI completo.

## Global Constraints

- Rust stable, `edition = "2021"`, workspace `version = "0.1.0"`, `license = "GPL-3.0-or-later"` en todos los crates.
- Nombres de crate: `goymusic-core` (`crates/core`) y `goymusic-library` (`crates/library`).
- `goymusic-core` no depende de ningún otro crate del workspace ni de Tauri.
- Versiones fijadas exactamente como en "Tech Stack". Si la API de un crate difiere del fragmento del plan en la versión fijada, consultar docs.rs de **esa** versión y adaptar la llamada; no cambiar la versión.
- Ninguna pista de audio con copyright en el repositorio: los tests generan WAV con `hound`.
- Nada de código, esquemas ni nombres de Spotify (spec §1).
- Ruta de la base de datos en producción: `~/.local/share/goymusic/library.db`. Los tests usan `Db::open_in_memory()` o un `tempfile`.
- Al terminar cada tarea deben pasar: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` y `cargo test --workspace`.
- Mensajes de commit en español.
- Umbrales de la spec: "anterior" reinicia la pista si la posición > 3000 ms; detección de movidos con duración ±1000 ms; búsqueda difusa con Jaro-Winkler ≥ 0,85 aplicada solo si hay menos de 5 resultados; objetivo de búsqueda < 50 ms con 50 000 pistas.

## Review Focus

1. **Consultas de búsqueda con sintaxis de FTS5** (`"`, `*`, `-`, `(`, `AND`, `NEAR`): se tratan como texto y nunca devuelven error. Test en la Task 13.
2. **Archivo corrupto o truncado con extensión de audio**: se anota en `ScanReport.errors` y el escaneo continúa con el resto. Test en la Task 8.
3. **Nombres de archivo que no son UTF-8** (Linux): se anotan como error, sin pánico. Test en la Task 8.
4. **Playlists: posiciones fuera de rango y pistas duplicadas**: `InvalidPosition` en lugar de pánico; los duplicados se permiten. Test en la Task 11.
5. **Carpetas solapadas** (la misma dos veces, una subcarpeta o una carpeta padre de otra ya añadida): `FolderOverlap`. Test en la Task 7.

---

## Mapa de ficheros

```
Cargo.toml                         workspace
LICENSE                            GPL-3.0
.github/workflows/ci.yml           fmt + clippy + test
crates/core/
  Cargo.toml
  src/lib.rs                       módulos y re-exports
  src/ids.rs                       newtypes de IDs
  src/model.rs                     Track, Album, Artist, Folder, Playlist, PlaylistFolder, PlaylistItem
  src/queue.rs                     cola: contexto, cola de usuario, shuffle, repeat, historial
  src/events.rs                    Event + EventBus
  tests/model.rs  tests/queue_basic.rs  tests/queue_advanced.rs  tests/events.rs
crates/library/
  Cargo.toml
  migrations/0001_init.sql         esquema
  src/lib.rs
  src/error.rs                     LibraryError, Result
  src/db.rs                        Db (abrir, migrar, pragmas)
  src/tags.rs                      lectura de etiquetas (lofty)
  src/maintenance.rs               borrar pistas + huérfanos (compartido)
  src/folders.rs                   carpetas de la biblioteca
  src/scanner.rs                   escaneo incremental
  src/moves.rs                     detección de archivos movidos
  src/queries.rs                   lecturas: álbumes, artistas, pistas
  src/playlists.rs                 playlists, carpetas, "Me gusta"
  src/m3u.rs                       importar/exportar M3U8
  src/search.rs                    FTS5 + difusa
  src/state.rs                     cola persistida + ajustes
  tests/common/mod.rs              generador de WAV etiquetados
  tests/db.rs tests/tags.rs tests/folders.rs tests/scan.rs tests/moves.rs
  tests/queries.rs tests/playlists.rs tests/m3u.rs tests/search.rs tests/state.rs
```

---

### Task 1: Workspace, licencia, CI y modelo del núcleo

**Files:**
- Create: `Cargo.toml`, `LICENSE`, `.github/workflows/ci.yml`
- Create: `crates/core/Cargo.toml`, `crates/core/src/lib.rs`, `crates/core/src/ids.rs`, `crates/core/src/model.rs`
- Test: `crates/core/tests/model.rs`

**Interfaces:**
- Produces: `goymusic_core::{TrackId, AlbumId, ArtistId, PlaylistId, PlaylistFolderId, FolderId}` (newtypes `pub struct X(pub i64)`, `Copy`, `Eq`, `Hash`, `Ord`, serde transparente) y `goymusic_core::model::{Track, Album, Artist, Folder, Playlist, PlaylistFolder, PlaylistItem}`.

- [ ] **Step 1: Crear el workspace**

`Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["crates/core"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "GPL-3.0-or-later"

[workspace.dependencies]
deunicode = "1"
hound = "3"
lofty = "0.21"
rand = "0.8"
rayon = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
rusqlite_migration = "1.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
strsim = "0.11"
tempfile = "3"
thiserror = "2"
walkdir = "2"
```

`crates/core/Cargo.toml`:
```toml
[package]
name = "goymusic-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true

[dev-dependencies]
serde_json.workspace = true
```

Licencia:
```bash
curl -fsSL https://www.gnu.org/licenses/gpl-3.0.txt -o LICENSE
head -1 LICENSE   # Expected: "                    GNU GENERAL PUBLIC LICENSE"
```

`.github/workflows/ci.yml`:
```yaml
name: ci
on: [push, pull_request]
jobs:
  rust:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
```

- [ ] **Step 2: Escribir el test que falla**

`crates/core/tests/model.rs`:
```rust
use goymusic_core::model::Track;
use goymusic_core::{AlbumId, TrackId};

#[test]
fn ids_serialize_as_plain_numbers() {
    assert_eq!(serde_json::to_string(&TrackId(5)).unwrap(), "5");
    assert_eq!(serde_json::from_str::<AlbumId>("9").unwrap(), AlbumId(9));
}

#[test]
fn track_roundtrips_through_json() {
    let track = Track {
        id: TrackId(1),
        path: "/music/a.flac".into(),
        title: "Song".into(),
        artists: vec!["Artist".into()],
        album: Some("Album".into()),
        album_id: Some(AlbumId(2)),
        album_artist: None,
        track_no: Some(3),
        disc_no: None,
        year: Some(1997),
        genre: None,
        duration_ms: 1234,
    };
    let json = serde_json::to_string(&track).unwrap();
    assert_eq!(serde_json::from_str::<Track>(&json).unwrap(), track);
}
```

- [ ] **Step 3: Ejecutar el test y verificar que falla**

Run: `cargo test -p goymusic-core`
Expected: FAIL de compilación (`could not find model in goymusic_core`).

- [ ] **Step 4: Implementar**

`crates/core/src/lib.rs`:
```rust
//! GoyMusic core: domain types, playback queue and event bus. Pure logic, no I/O.

pub mod ids;
pub mod model;

pub use ids::*;
```

`crates/core/src/ids.rs`:
```rust
use serde::{Deserialize, Serialize};

macro_rules! id_types {
    ($($name:ident),* $(,)?) => {
        $(
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
            #[serde(transparent)]
            pub struct $name(pub i64);
        )*
    };
}

id_types!(TrackId, AlbumId, ArtistId, PlaylistId, PlaylistFolderId, FolderId);
```

`crates/core/src/model.rs`:
```rust
use serde::{Deserialize, Serialize};

use crate::{AlbumId, ArtistId, FolderId, PlaylistFolderId, PlaylistId, TrackId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub path: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_id: Option<AlbumId>,
    pub album_artist: Option<String>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub duration_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Album {
    pub id: AlbumId,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<i32>,
    pub track_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artist {
    pub id: ArtistId,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub id: FolderId,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: String,
    pub folder_id: Option<PlaylistFolderId>,
    pub track_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistFolder {
    pub id: PlaylistFolderId,
    pub name: String,
    pub parent_id: Option<PlaylistFolderId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub position: u32,
    pub track: Track,
}
```

- [ ] **Step 5: Ejecutar los tests y verificar que pasan**

Run: `cargo test -p goymusic-core && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (2 tests).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml LICENSE .github crates/core
git commit -m "Workspace, licencia GPL-3.0, CI y modelo del núcleo"
```

---

### Task 2: Cola — contexto, siguiente, anterior y repeat

**Files:**
- Create: `crates/core/src/queue.rs`
- Modify: `crates/core/src/lib.rs` (añadir `pub mod queue;`), `crates/core/Cargo.toml` (añadir `rand.workspace = true` a `[dependencies]`)
- Test: `crates/core/tests/queue_basic.rs`

**Interfaces:**
- Consumes: `TrackId` (Task 1).
- Produces (`goymusic_core::queue`): `Queue` (`Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize`), `RepeatMode { Off, All, One }`, `Previous { Restart, Track(TrackId) }`, `RESTART_THRESHOLD_MS: u64 = 3_000`.
  Métodos: `new()`, `current() -> Option<TrackId>`, `context() -> &[TrackId]`, `repeat() -> RepeatMode`, `shuffle() -> bool`, `set_repeat(RepeatMode)`, `play_context(Vec<TrackId>, start_index: usize, rng: &mut impl rand::Rng) -> Option<TrackId>`, `next() -> Option<TrackId>` (manual), `on_track_ended() -> Option<TrackId>` (automático), `previous(position_ms: u64) -> Previous`.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/core/tests/queue_basic.rs`:
```rust
use goymusic_core::queue::{Previous, Queue, RepeatMode};
use goymusic_core::TrackId;
use rand::rngs::StdRng;
use rand::SeedableRng;

fn ids(n: i64) -> Vec<TrackId> {
    (1..=n).map(TrackId).collect()
}

fn rng() -> StdRng {
    StdRng::seed_from_u64(7)
}

#[test]
fn play_context_starts_at_index() {
    let mut q = Queue::new();
    assert_eq!(q.play_context(ids(3), 1, &mut rng()), Some(TrackId(2)));
    assert_eq!(q.current(), Some(TrackId(2)));
}

#[test]
fn next_walks_context_then_stops() {
    let mut q = Queue::new();
    q.play_context(ids(3), 0, &mut rng());
    assert_eq!(q.next(), Some(TrackId(2)));
    assert_eq!(q.next(), Some(TrackId(3)));
    assert_eq!(q.next(), None);
    assert_eq!(q.next(), None);
}

#[test]
fn repeat_all_wraps_to_start() {
    let mut q = Queue::new();
    q.set_repeat(RepeatMode::All);
    q.play_context(ids(3), 2, &mut rng());
    assert_eq!(q.next(), Some(TrackId(1)));
}

#[test]
fn repeat_one_replays_when_track_ends_but_manual_next_advances() {
    let mut q = Queue::new();
    q.set_repeat(RepeatMode::One);
    q.play_context(ids(3), 0, &mut rng());
    assert_eq!(q.on_track_ended(), Some(TrackId(1)));
    assert_eq!(q.next(), Some(TrackId(2)));
}

#[test]
fn previous_after_threshold_restarts_track() {
    let mut q = Queue::new();
    q.play_context(ids(3), 0, &mut rng());
    q.next();
    assert_eq!(q.previous(3_001), Previous::Restart);
    assert_eq!(q.current(), Some(TrackId(2)));
}

#[test]
fn previous_within_threshold_goes_back_and_next_returns() {
    let mut q = Queue::new();
    q.play_context(ids(3), 0, &mut rng());
    q.next();
    q.next();
    assert_eq!(q.previous(1_000), Previous::Track(TrackId(2)));
    assert_eq!(q.current(), Some(TrackId(2)));
    assert_eq!(q.next(), Some(TrackId(3)));
}

#[test]
fn previous_with_no_history_restarts() {
    let mut q = Queue::new();
    q.play_context(ids(3), 0, &mut rng());
    assert_eq!(q.previous(0), Previous::Restart);
}

#[test]
fn empty_or_out_of_range_context_plays_nothing() {
    let mut q = Queue::new();
    assert_eq!(q.play_context(vec![], 0, &mut rng()), None);
    assert_eq!(q.play_context(ids(2), 5, &mut rng()), None);
    assert_eq!(q.current(), None);
    assert_eq!(q.next(), None);
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-core --test queue_basic`
Expected: FAIL de compilación (`could not find queue`).

- [ ] **Step 3: Implementar**

En `crates/core/src/lib.rs` añadir `pub mod queue;` debajo de `pub mod model;`. En `crates/core/Cargo.toml`, `[dependencies]` queda:
```toml
[dependencies]
rand.workspace = true
serde.workspace = true
```

`crates/core/src/queue.rs`:
```rust
//! Playback queue: a context (album, playlist, search results...) played in order, plus a
//! user queue that takes priority. Pure logic, no I/O.

use std::collections::VecDeque;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::TrackId;

/// "Previous" within this many milliseconds goes back a track; later it restarts the track.
pub const RESTART_THRESHOLD_MS: u64 = 3_000;
const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Previous {
    Restart,
    Track(TrackId),
}

/// A track that was current, with the context index the queue was at. Storing the context
/// index (not the position in the play order) keeps history valid across shuffle toggles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct HistoryEntry {
    track: TrackId,
    context_index: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Queue {
    context: Vec<TrackId>,
    /// Play order: indices into `context`.
    order: Vec<usize>,
    /// Position in `order` of the last context track that was current.
    cursor: Option<usize>,
    user_queue: VecDeque<TrackId>,
    history: Vec<HistoryEntry>,
    current: Option<TrackId>,
    shuffle: bool,
    repeat: RepeatMode,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> Option<TrackId> {
        self.current
    }

    pub fn context(&self) -> &[TrackId] {
        &self.context
    }

    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    /// Replaces the context and makes `tracks[start_index]` current. History is cleared:
    /// it refers to the previous context.
    pub fn play_context(&mut self, tracks: Vec<TrackId>, start_index: usize, rng: &mut impl Rng) -> Option<TrackId> {
        self.history.clear();
        self.context = tracks;
        if start_index >= self.context.len() {
            self.order.clear();
            self.cursor = None;
            self.current = None;
            return None;
        }
        self.order = self.build_order(start_index, rng);
        self.cursor = self.order.iter().position(|&i| i == start_index);
        self.current = Some(self.context[start_index]);
        self.current
    }

    /// Play order for the context, starting from `anchor` when shuffling.
    fn build_order(&self, _anchor: usize, _rng: &mut impl Rng) -> Vec<usize> {
        (0..self.context.len()).collect()
    }

    /// Manual "next": always moves on, even with repeat-one.
    pub fn next(&mut self) -> Option<TrackId> {
        self.advance(false)
    }

    /// The current track finished playing on its own.
    pub fn on_track_ended(&mut self) -> Option<TrackId> {
        self.advance(true)
    }

    fn advance(&mut self, track_ended: bool) -> Option<TrackId> {
        if track_ended && self.repeat == RepeatMode::One && self.current.is_some() {
            return self.current;
        }
        self.push_history();
        if let Some(track) = self.user_queue.pop_front() {
            self.current = Some(track);
            return self.current;
        }
        let next = self.cursor.map_or(0, |c| c + 1);
        let position = if next < self.order.len() {
            Some(next)
        } else if self.repeat != RepeatMode::Off && !self.order.is_empty() {
            Some(0)
        } else {
            None
        };
        match position {
            Some(p) => {
                self.cursor = Some(p);
                self.current = Some(self.context[self.order[p]]);
            }
            None => self.current = None,
        }
        self.current
    }

    fn push_history(&mut self) {
        if let Some(track) = self.current {
            let context_index = self.cursor.map(|c| self.order[c]);
            self.history.push(HistoryEntry { track, context_index });
            if self.history.len() > HISTORY_LIMIT {
                self.history.remove(0);
            }
        }
    }

    /// "Previous" button: restart the track after [`RESTART_THRESHOLD_MS`], otherwise go
    /// back to the last track that was current.
    pub fn previous(&mut self, position_ms: u64) -> Previous {
        if position_ms > RESTART_THRESHOLD_MS {
            return Previous::Restart;
        }
        match self.history.pop() {
            Some(entry) => {
                self.current = Some(entry.track);
                self.cursor = entry.context_index.and_then(|ci| self.order.iter().position(|&i| i == ci));
                Previous::Track(entry.track)
            }
            None => Previous::Restart,
        }
    }
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-core && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (8 tests nuevos). Clippy puede avisar de los parámetros `_anchor`/`_rng` sin usar: llevan `_` delante, así que no hay aviso; los usa la Task 3.

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "Cola de reproducción: contexto, siguiente, anterior y repeat"
```

---

### Task 3: Cola — cola de usuario, shuffle, próximas y serialización

**Files:**
- Modify: `crates/core/src/queue.rs`
- Test: `crates/core/tests/queue_advanced.rs`

**Interfaces:**
- Consumes: `Queue` (Task 2).
- Produces: `queue_add(&mut self, &[TrackId])`, `queue_remove(&mut self, index: usize) -> Option<TrackId>`, `queue_move(&mut self, from: usize, to: usize) -> bool`, `user_queue(&self) -> Vec<TrackId>`, `upcoming(&self, limit: usize) -> Vec<TrackId>`, `set_shuffle(&mut self, on: bool, rng: &mut impl Rng)`. `Queue` serializado con serde (lo usa la Task 14 para persistir la cola).

- [ ] **Step 1: Escribir los tests que fallan**

`crates/core/tests/queue_advanced.rs`:
```rust
use goymusic_core::queue::{Previous, Queue};
use goymusic_core::TrackId;
use rand::rngs::StdRng;
use rand::SeedableRng;

fn ids(n: i64) -> Vec<TrackId> {
    (1..=n).map(TrackId).collect()
}

fn rng() -> StdRng {
    StdRng::seed_from_u64(7)
}

#[test]
fn user_queue_has_priority_then_context_resumes() {
    let mut q = Queue::new();
    q.play_context(ids(3), 0, &mut rng());
    q.queue_add(&[TrackId(100)]);
    assert_eq!(q.next(), Some(TrackId(100)));
    assert_eq!(q.next(), Some(TrackId(2)));
}

#[test]
fn queue_remove_and_move() {
    let mut q = Queue::new();
    q.play_context(ids(1), 0, &mut rng());
    q.queue_add(&[TrackId(10), TrackId(11), TrackId(12)]);
    assert_eq!(q.queue_remove(1), Some(TrackId(11)));
    assert!(q.queue_move(1, 0));
    assert_eq!(q.user_queue(), vec![TrackId(12), TrackId(10)]);
    assert!(!q.queue_move(5, 0));
    assert_eq!(q.queue_remove(9), None);
}

#[test]
fn upcoming_lists_user_queue_then_context() {
    let mut q = Queue::new();
    q.play_context(ids(4), 1, &mut rng());
    q.queue_add(&[TrackId(50)]);
    assert_eq!(q.upcoming(10), vec![TrackId(50), TrackId(3), TrackId(4)]);
    assert_eq!(q.upcoming(2), vec![TrackId(50), TrackId(3)]);
}

#[test]
fn shuffle_keeps_current_and_is_a_permutation() {
    let mut r = rng();
    let mut q = Queue::new();
    q.play_context(ids(10), 4, &mut r);
    q.set_shuffle(true, &mut r);
    assert_eq!(q.current(), Some(TrackId(5)));
    let mut upcoming = q.upcoming(20);
    upcoming.sort();
    let expected: Vec<TrackId> = ids(10).into_iter().filter(|&t| t != TrackId(5)).collect();
    assert_eq!(upcoming, expected);
}

#[test]
fn play_context_with_shuffle_on_starts_with_chosen_track() {
    let mut r = rng();
    let mut q = Queue::new();
    q.set_shuffle(true, &mut r);
    assert_eq!(q.play_context(ids(10), 7, &mut r), Some(TrackId(8)));
    let upcoming = q.upcoming(20);
    assert_eq!(upcoming.len(), 9);
    assert!(!upcoming.contains(&TrackId(8)));
}

#[test]
fn shuffle_off_continues_in_natural_order_after_current() {
    let mut r = rng();
    let mut q = Queue::new();
    q.play_context(ids(10), 0, &mut r);
    q.set_shuffle(true, &mut r);
    let x = q.next().unwrap();
    q.set_shuffle(false, &mut r);
    let expected = if x.0 < 10 { Some(TrackId(x.0 + 1)) } else { None };
    assert_eq!(q.next(), expected);
}

#[test]
fn previous_survives_shuffle_toggle() {
    let mut r = rng();
    let mut q = Queue::new();
    q.play_context(ids(5), 0, &mut r);
    q.next();
    q.set_shuffle(true, &mut r);
    assert_eq!(q.previous(0), Previous::Track(TrackId(1)));
}

#[test]
fn queue_state_roundtrips_through_json() {
    let mut r = rng();
    let mut q = Queue::new();
    q.play_context(ids(6), 2, &mut r);
    q.set_shuffle(true, &mut r);
    q.queue_add(&[TrackId(99)]);
    q.next();
    let json = serde_json::to_string(&q).unwrap();
    assert_eq!(serde_json::from_str::<Queue>(&json).unwrap(), q);
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-core --test queue_advanced`
Expected: FAIL de compilación (`no method named queue_add`).

- [ ] **Step 3: Implementar**

En `crates/core/src/queue.rs`:

1. Cambiar los imports a:
```rust
use std::collections::VecDeque;

use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::TrackId;
```

2. Sustituir `build_order` por:
```rust
    /// Play order for the context. Natural order; with shuffle, `anchor` first and the
    /// rest in random order.
    fn build_order(&self, anchor: usize, rng: &mut impl Rng) -> Vec<usize> {
        if !self.shuffle {
            return (0..self.context.len()).collect();
        }
        let mut rest: Vec<usize> = (0..self.context.len()).filter(|&i| i != anchor).collect();
        rest.shuffle(rng);
        std::iter::once(anchor).chain(rest).collect()
    }
```

3. Añadir dentro de `impl Queue`:
```rust
    /// Turns shuffle on/off keeping the current context track in place.
    pub fn set_shuffle(&mut self, on: bool, rng: &mut impl Rng) {
        if on == self.shuffle {
            return;
        }
        self.shuffle = on;
        let Some(anchor) = self.cursor.map(|c| self.order[c]) else {
            return;
        };
        self.order = self.build_order(anchor, rng);
        self.cursor = self.order.iter().position(|&i| i == anchor);
    }

    pub fn queue_add(&mut self, tracks: &[TrackId]) {
        self.user_queue.extend(tracks.iter().copied());
    }

    pub fn queue_remove(&mut self, index: usize) -> Option<TrackId> {
        self.user_queue.remove(index)
    }

    /// Moves a user-queue entry; `false` if either index is out of range.
    pub fn queue_move(&mut self, from: usize, to: usize) -> bool {
        if from >= self.user_queue.len() || to >= self.user_queue.len() {
            return false;
        }
        let track = self.user_queue.remove(from).expect("index checked");
        self.user_queue.insert(to, track);
        true
    }

    pub fn user_queue(&self) -> Vec<TrackId> {
        self.user_queue.iter().copied().collect()
    }

    /// What plays next: the user queue, then the rest of the context in play order.
    pub fn upcoming(&self, limit: usize) -> Vec<TrackId> {
        let from = self.cursor.map_or(0, |c| c + 1);
        self.user_queue
            .iter()
            .copied()
            .chain(self.order.iter().skip(from).map(|&i| self.context[i]))
            .take(limit)
            .collect()
    }
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-core && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (todos los tests de `queue_basic` y `queue_advanced`).

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "Cola: cola de usuario, shuffle, próximas y serialización"
```

---

### Task 4: Eventos del núcleo

**Files:**
- Create: `crates/core/src/events.rs`
- Modify: `crates/core/src/lib.rs` (añadir `pub mod events;`)
- Test: `crates/core/tests/events.rs`

**Interfaces:**
- Produces (`goymusic_core::events`): `Event` (`Clone, Debug, PartialEq, Eq, Serialize, Deserialize`; `#[serde(tag = "type", rename_all = "camelCase")]`) con las variantes `LibraryUpdated`, `ScanProgress { done: u64, total: u64 }`, `PlaylistsChanged`, `LikesChanged`, `QueueChanged`. `EventBus` (`Default`) con `new()`, `subscribe() -> std::sync::mpsc::Receiver<Event>`, `publish(Event)` y `subscriber_count() -> usize`. Las variantes del reproductor se añaden en P2.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/core/tests/events.rs`:
```rust
use goymusic_core::events::{Event, EventBus};

#[test]
fn every_subscriber_receives_published_events() {
    let bus = EventBus::new();
    let a = bus.subscribe();
    let b = bus.subscribe();
    bus.publish(Event::ScanProgress { done: 1, total: 2 });
    assert_eq!(a.try_recv().unwrap(), Event::ScanProgress { done: 1, total: 2 });
    assert_eq!(b.try_recv().unwrap(), Event::ScanProgress { done: 1, total: 2 });
}

#[test]
fn dropped_subscribers_are_removed_on_publish() {
    let bus = EventBus::new();
    let kept = bus.subscribe();
    drop(bus.subscribe());
    bus.publish(Event::LibraryUpdated);
    assert_eq!(bus.subscriber_count(), 1);
    assert_eq!(kept.try_recv().unwrap(), Event::LibraryUpdated);
}

#[test]
fn events_serialize_with_a_type_tag() {
    let json = serde_json::to_string(&Event::ScanProgress { done: 3, total: 9 }).unwrap();
    assert_eq!(json, r#"{"type":"scanProgress","done":3,"total":9}"#);
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-core --test events`
Expected: FAIL de compilación (`could not find events`).

- [ ] **Step 3: Implementar**

En `crates/core/src/lib.rs`, la lista de módulos queda:
```rust
pub mod events;
pub mod ids;
pub mod model;
pub mod queue;
```

`crates/core/src/events.rs`:
```rust
//! Core events. Services publish them; adapters (Tauri now, WebSocket later) forward them
//! to the UI, so the UI never polls.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    LibraryUpdated,
    ScanProgress { done: u64, total: u64 },
    PlaylistsChanged,
    LikesChanged,
    QueueChanged,
}

#[derive(Default)]
pub struct EventBus {
    subscribers: Mutex<Vec<Sender<Event>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self) -> Receiver<Event> {
        let (tx, rx) = channel();
        self.subscribers.lock().expect("event bus poisoned").push(tx);
        rx
    }

    /// Sends `event` to every live subscriber and forgets the ones whose receiver was dropped.
    pub fn publish(&self, event: Event) {
        self.subscribers
            .lock()
            .expect("event bus poisoned")
            .retain(|s| s.send(event.clone()).is_ok());
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().expect("event bus poisoned").len()
    }
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-core && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "Bus de eventos del núcleo"
```

---

### Task 5: Crate de biblioteca, errores y base de datos

**Files:**
- Create: `crates/library/Cargo.toml`, `crates/library/src/lib.rs`, `crates/library/src/error.rs`, `crates/library/src/db.rs`, `crates/library/migrations/0001_init.sql`
- Modify: `Cargo.toml` (añadir `"crates/library"` a `members`)
- Test: `crates/library/tests/db.rs`

**Interfaces:**
- Produces: `goymusic_library::error::{LibraryError, Result}`; `goymusic_library::db::Db` con `open(&Path) -> Result<Db>`, `open_in_memory() -> Result<Db>`, `conn(&self) -> &rusqlite::Connection`, `conn_mut(&mut self) -> &mut rusqlite::Connection`.
- `LibraryError`: `Db(rusqlite::Error)`, `Migration(rusqlite_migration::Error)`, `Io(std::io::Error)`, `Json(serde_json::Error)`, `Tags { path: String, message: String }`, `NotFound { kind: &'static str, id: i64 }`, `InvalidPosition { position: usize, len: usize }`, `FolderOverlap(String)`, `NonUtf8Path(String)`.

- [ ] **Step 1: Crear el crate**

En `Cargo.toml` raíz: `members = ["crates/core", "crates/library"]`.

`crates/library/Cargo.toml`:
```toml
[package]
name = "goymusic-library"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
deunicode.workspace = true
goymusic-core = { path = "../core" }
lofty.workspace = true
rand.workspace = true
rayon.workspace = true
rusqlite.workspace = true
rusqlite_migration.workspace = true
serde_json.workspace = true
strsim.workspace = true
thiserror.workspace = true
walkdir.workspace = true

[dev-dependencies]
hound.workspace = true
tempfile.workspace = true
```

`crates/library/migrations/0001_init.sql`:
```sql
CREATE TABLE folders (
    id   INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE
);

CREATE TABLE artists (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE albums (
    id            INTEGER PRIMARY KEY,
    title         TEXT NOT NULL,
    artist_id     INTEGER REFERENCES artists(id) ON DELETE SET NULL,
    mb_release_id TEXT,
    year          INTEGER,
    group_key     TEXT NOT NULL UNIQUE
);

CREATE TABLE tracks (
    id              INTEGER PRIMARY KEY,
    path            TEXT NOT NULL UNIQUE,
    folder_id       INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    size            INTEGER NOT NULL,
    mtime           INTEGER NOT NULL,
    title           TEXT NOT NULL,
    album_id        INTEGER REFERENCES albums(id) ON DELETE SET NULL,
    album_artist    TEXT,
    track_no        INTEGER,
    disc_no         INTEGER,
    year            INTEGER,
    genre           TEXT,
    duration_ms     INTEGER NOT NULL,
    mb_recording_id TEXT
);
CREATE INDEX tracks_album ON tracks(album_id);
CREATE INDEX tracks_folder ON tracks(folder_id);

CREATE TABLE track_artists (
    track_id  INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    artist_id INTEGER NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    PRIMARY KEY (track_id, position)
);
CREATE INDEX track_artists_artist ON track_artists(artist_id);

CREATE TABLE playlist_folders (
    id        INTEGER PRIMARY KEY,
    name      TEXT NOT NULL,
    parent_id INTEGER REFERENCES playlist_folders(id) ON DELETE CASCADE
);

CREATE TABLE playlists (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    folder_id  INTEGER REFERENCES playlist_folders(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE playlist_items (
    id          INTEGER PRIMARY KEY,
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    pos         INTEGER NOT NULL,
    track_id    INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE
);
CREATE INDEX playlist_items_order ON playlist_items(playlist_id, pos);

CREATE TABLE likes (
    id       INTEGER PRIMARY KEY,
    track_id INTEGER NOT NULL UNIQUE REFERENCES tracks(id) ON DELETE CASCADE,
    liked_at INTEGER NOT NULL
);

CREATE TABLE play_history (
    id        INTEGER PRIMARY KEY,
    track_id  INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    played_at INTEGER NOT NULL
);

CREATE TABLE queue_state (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    snapshot    TEXT NOT NULL,
    position_ms INTEGER NOT NULL
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE VIRTUAL TABLE tracks_fts USING fts5(
    title, artists, album,
    tokenize = 'unicode61 remove_diacritics 2'
);
```

- [ ] **Step 2: Escribir el test que falla**

`crates/library/tests/db.rs`:
```rust
use goymusic_library::db::Db;

fn table_exists(db: &Db, name: &str) -> bool {
    db.conn()
        .query_row("SELECT count(*) FROM sqlite_master WHERE name = ?1", [name], |r| r.get::<_, i64>(0))
        .unwrap()
        == 1
}

#[test]
fn in_memory_db_is_migrated_with_foreign_keys_on() {
    let db = Db::open_in_memory().unwrap();
    for table in ["folders", "tracks", "albums", "artists", "playlists", "playlist_items", "likes", "queue_state", "settings", "tracks_fts"] {
        assert!(table_exists(&db, table), "missing table {table}");
    }
    let fk: i64 = db.conn().query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
    assert_eq!(fk, 1);
    let version: i64 = db.conn().query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(version, 1);
}

#[test]
fn file_db_is_created_with_parent_dirs_and_reopens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/library.db");
    drop(Db::open(&path).unwrap());
    let db = Db::open(&path).unwrap();
    let mode: String = db.conn().query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
    assert_eq!(mode, "wal");
}
```

- [ ] **Step 3: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-library --test db`
Expected: FAIL de compilación (`could not find db`).

- [ ] **Step 4: Implementar**

`crates/library/src/lib.rs`:
```rust
//! GoyMusic library: SQLite storage, folder scanning, playlists, search.

pub mod db;
pub mod error;
```

`crates/library/src/error.rs`:
```rust
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("migration: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("tags in {path}: {message}")]
    Tags { path: String, message: String },
    #[error("{kind} {id} not found")]
    NotFound { kind: &'static str, id: i64 },
    #[error("position {position} out of range (length {len})")]
    InvalidPosition { position: usize, len: usize },
    #[error("folder {0} overlaps a library folder")]
    FolderOverlap(String),
    #[error("path is not valid UTF-8: {0}")]
    NonUtf8Path(String),
}

pub type Result<T> = std::result::Result<T, LibraryError>;
```

`crates/library/src/db.rs`:
```rust
use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::error::Result;

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("../migrations/0001_init.sql"))])
}

pub struct Db {
    conn: Connection,
}

impl Db {
    /// Opens (creating parent directories) and migrates the library database.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        // journal_mode returns a row, so it needs the `_and_check` variant
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrations().to_latest(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}
```

- [ ] **Step 5: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-library && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (2 tests).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/library
git commit -m "Crate de biblioteca: errores, esquema SQLite y migraciones"
```

---

### Task 6: Lectura de etiquetas y generador de pistas de prueba

**Files:**
- Create: `crates/library/src/tags.rs`, `crates/library/tests/common/mod.rs`
- Modify: `crates/library/src/lib.rs` (añadir `pub mod tags;`)
- Test: `crates/library/tests/tags.rs`

**Interfaces:**
- Produces: `goymusic_library::tags::{TrackTags, read_tags}`. `TrackTags { title: Option<String>, artists: Vec<String>, album_artist: Option<String>, album: Option<String>, track_no: Option<u32>, disc_no: Option<u32>, year: Option<i32>, genre: Option<String>, duration_ms: u64, mb_recording_id: Option<String>, mb_release_id: Option<String> }` (`Clone, Debug, Default, PartialEq`). `read_tags(&Path) -> Result<TrackTags>`; un archivo que no se puede leer da `LibraryError::Tags`.
- Test support (`tests/common/mod.rs`, lo usan las Tasks 8–14): `write_wav(&Path, seconds: f32)`, `struct Meta<'a> { title, artist, album: Option<&'a str>, album_artist: Option<&'a str>, track: Option<u32>, mb_recording_id: Option<&'a str> }` con `Meta::new(title, artist)` y `write_tagged_wav(&Path, seconds: f32, &Meta)`.

- [ ] **Step 1: Escribir el generador de pruebas y el test que falla**

`crates/library/tests/common/mod.rs`:
```rust
#![allow(dead_code)] // each test binary uses a different subset

use std::f32::consts::PI;
use std::path::Path;

use lofty::config::WriteOptions;
use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag, TagType};

const RATE: u32 = 8_000;

/// Writes a mono 8 kHz sine wave. Generated audio keeps copyrighted files out of the repo.
pub fn write_wav(path: &Path, seconds: f32) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    let spec = hound::WavSpec { channels: 1, sample_rate: RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..(RATE as f32 * seconds) as u32 {
        let sample = (i as f32 * 440.0 * 2.0 * PI / RATE as f32).sin() * f32::from(i16::MAX) * 0.2;
        writer.write_sample(sample as i16).unwrap();
    }
    writer.finalize().unwrap();
}

pub struct Meta<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: Option<&'a str>,
    pub album_artist: Option<&'a str>,
    pub track: Option<u32>,
    pub mb_recording_id: Option<&'a str>,
}

impl<'a> Meta<'a> {
    pub fn new(title: &'a str, artist: &'a str) -> Self {
        Self { title, artist, album: None, album_artist: None, track: None, mb_recording_id: None }
    }
}

pub fn write_tagged_wav(path: &Path, seconds: f32, meta: &Meta) {
    write_wav(path, seconds);
    let mut tag = Tag::new(TagType::Id3v2);
    tag.insert_text(ItemKey::TrackTitle, meta.title.into());
    tag.insert_text(ItemKey::TrackArtist, meta.artist.into());
    if let Some(album) = meta.album {
        tag.insert_text(ItemKey::AlbumTitle, album.into());
    }
    if let Some(album_artist) = meta.album_artist {
        tag.insert_text(ItemKey::AlbumArtist, album_artist.into());
    }
    if let Some(track) = meta.track {
        tag.insert_text(ItemKey::TrackNumber, track.to_string());
    }
    if let Some(id) = meta.mb_recording_id {
        tag.insert_text(ItemKey::MusicBrainzRecordingId, id.into());
    }
    tag.save_to_path(path, WriteOptions::default()).unwrap();
}
```

`crates/library/tests/tags.rs`:
```rust
mod common;

use common::{write_tagged_wav, write_wav, Meta};
use goymusic_library::error::LibraryError;
use goymusic_library::tags::read_tags;

#[test]
fn reads_tags_and_duration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    let meta = Meta { album: Some("OK Computer"), album_artist: Some("Radiohead"), track: Some(3), ..Meta::new("Subterranean", "Radiohead") };
    write_tagged_wav(&path, 1.0, &meta);

    let tags = read_tags(&path).unwrap();
    assert_eq!(tags.title.as_deref(), Some("Subterranean"));
    assert_eq!(tags.artists, vec!["Radiohead".to_owned()]);
    assert_eq!(tags.album.as_deref(), Some("OK Computer"));
    assert_eq!(tags.album_artist.as_deref(), Some("Radiohead"));
    assert_eq!(tags.track_no, Some(3));
    assert!((950..=1050).contains(&tags.duration_ms), "duration {}", tags.duration_ms);
}

#[test]
fn untagged_file_has_duration_but_no_title() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.wav");
    write_wav(&path, 0.5);
    let tags = read_tags(&path).unwrap();
    assert_eq!(tags.title, None);
    assert!(tags.artists.is_empty());
    assert!(tags.duration_ms > 400);
}

#[test]
fn garbage_file_is_a_tags_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.mp3");
    std::fs::write(&path, b"definitely not audio").unwrap();
    assert!(matches!(read_tags(&path), Err(LibraryError::Tags { .. })));
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-library --test tags`
Expected: FAIL de compilación (`could not find tags`).

- [ ] **Step 3: Implementar**

En `crates/library/src/lib.rs` añadir `pub mod tags;`.

`crates/library/src/tags.rs`:
```rust
use std::path::Path;

use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag};

use crate::error::{LibraryError, Result};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackTags {
    pub title: Option<String>,
    pub artists: Vec<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub duration_ms: u64,
    pub mb_recording_id: Option<String>,
    pub mb_release_id: Option<String>,
}

pub fn read_tags(path: &Path) -> Result<TrackTags> {
    let tagged = lofty::read_from_path(path)
        .map_err(|e| LibraryError::Tags { path: path.display().to_string(), message: e.to_string() })?;
    let duration_ms = tagged.properties().duration().as_millis() as u64;
    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Ok(TrackTags { duration_ms, ..TrackTags::default() });
    };
    let mut artists: Vec<String> = Vec::new();
    for artist in tag.get_strings(&ItemKey::TrackArtist).map(str::trim).filter(|a| !a.is_empty()) {
        if !artists.iter().any(|known| known.eq_ignore_ascii_case(artist)) {
            artists.push(artist.to_owned());
        }
    }
    Ok(TrackTags {
        title: text(tag, &ItemKey::TrackTitle),
        artists,
        album_artist: text(tag, &ItemKey::AlbumArtist),
        album: text(tag, &ItemKey::AlbumTitle),
        track_no: number(tag, &ItemKey::TrackNumber),
        disc_no: number(tag, &ItemKey::DiscNumber),
        year: text(tag, &ItemKey::Year).or_else(|| text(tag, &ItemKey::RecordingDate)).and_then(|d| leading_year(&d)),
        genre: text(tag, &ItemKey::Genre),
        duration_ms,
        mb_recording_id: text(tag, &ItemKey::MusicBrainzRecordingId),
        mb_release_id: text(tag, &ItemKey::MusicBrainzReleaseId),
    })
}

fn text(tag: &Tag, key: &ItemKey) -> Option<String> {
    tag.get_string(key).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
}

/// "3" or "3/12" → 3.
fn number(tag: &Tag, key: &ItemKey) -> Option<u32> {
    text(tag, key)?.split('/').next()?.trim().parse().ok()
}

/// "1997", "1997-05-21" → 1997.
fn leading_year(date: &str) -> Option<i32> {
    date.get(..4)?.parse().ok()
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-library && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (3 tests nuevos).

- [ ] **Step 5: Commit**

```bash
git add crates/library
git commit -m "Lectura de etiquetas y generador de WAV de prueba"
```

---

### Task 7: Carpetas de la biblioteca

**Files:**
- Create: `crates/library/src/folders.rs`, `crates/library/src/maintenance.rs`
- Modify: `crates/library/src/lib.rs` (añadir `pub mod folders;` y `mod maintenance;`)
- Test: `crates/library/tests/folders.rs`

**Interfaces:**
- Consumes: `Db`, `LibraryError` (Task 5); `goymusic_core::model::Folder`, `FolderId`.
- Produces: `folders::add(&Db, &Path) -> Result<Folder>` (ruta canónica; error `FolderOverlap` si es la misma, una subcarpeta o una carpeta padre de una ya añadida); `folders::list(&Db) -> Result<Vec<Folder>>` (ordenada por ruta); `folders::remove(&mut Db, FolderId) -> Result<()>` (borra sus pistas; `NotFound` si no existe).
- Produces (interno del crate, lo usan las Tasks 8, 9 y 11): `maintenance::delete_tracks(&rusqlite::Connection, &[i64]) -> Result<()>` (también borra sus filas FTS) y `maintenance::cleanup_orphans(&rusqlite::Connection) -> Result<()>` (álbumes y artistas sin pistas).

- [ ] **Step 1: Escribir los tests que fallan**

`crates/library/tests/folders.rs`:
```rust
use goymusic_core::FolderId;
use goymusic_library::db::Db;
use goymusic_library::error::LibraryError;
use goymusic_library::folders;

#[test]
fn add_stores_canonical_path_and_lists_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let with_dots = dir.path().join(".");
    let folder = folders::add(&db, &with_dots).unwrap();
    let canonical = std::fs::canonicalize(dir.path()).unwrap();
    assert_eq!(folder.path, canonical.to_str().unwrap());
    assert_eq!(folders::list(&db).unwrap(), vec![folder]);
}

#[test]
fn overlapping_folders_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let child = dir.path().join("child");
    std::fs::create_dir(&child).unwrap();
    let db = Db::open_in_memory().unwrap();
    folders::add(&db, &child).unwrap();

    assert!(matches!(folders::add(&db, &child), Err(LibraryError::FolderOverlap(_))));
    assert!(matches!(folders::add(&db, dir.path()), Err(LibraryError::FolderOverlap(_))));
    let grandchild = child.join("deeper");
    std::fs::create_dir(&grandchild).unwrap();
    assert!(matches!(folders::add(&db, &grandchild), Err(LibraryError::FolderOverlap(_))));
}

#[test]
fn missing_path_is_an_io_error() {
    let db = Db::open_in_memory().unwrap();
    assert!(matches!(folders::add(&db, std::path::Path::new("/definitely/not/here")), Err(LibraryError::Io(_))));
}

#[test]
fn remove_unknown_folder_is_not_found() {
    let mut db = Db::open_in_memory().unwrap();
    assert!(matches!(folders::remove(&mut db, FolderId(42)), Err(LibraryError::NotFound { kind: "folder", id: 42 })));
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-library --test folders`
Expected: FAIL de compilación (`could not find folders`).

- [ ] **Step 3: Implementar**

En `crates/library/src/lib.rs`:
```rust
pub mod db;
pub mod error;
pub mod folders;
pub mod tags;

mod maintenance;
```

`crates/library/src/maintenance.rs`:
```rust
use rusqlite::Connection;

use crate::error::Result;

/// Deletes tracks (and their FTS rows; FTS5 tables have no foreign keys).
pub(crate) fn delete_tracks(conn: &Connection, ids: &[i64]) -> Result<()> {
    let mut fts = conn.prepare("DELETE FROM tracks_fts WHERE rowid = ?1")?;
    let mut track = conn.prepare("DELETE FROM tracks WHERE id = ?1")?;
    for id in ids {
        fts.execute([id])?;
        track.execute([id])?;
    }
    Ok(())
}

/// Removes albums without tracks and artists referenced by neither tracks nor albums.
pub(crate) fn cleanup_orphans(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DELETE FROM albums WHERE id NOT IN (SELECT album_id FROM tracks WHERE album_id IS NOT NULL);
         DELETE FROM artists WHERE id NOT IN (SELECT artist_id FROM track_artists)
                               AND id NOT IN (SELECT artist_id FROM albums WHERE artist_id IS NOT NULL);",
    )?;
    Ok(())
}
```

`crates/library/src/folders.rs`:
```rust
use std::path::Path;

use goymusic_core::model::Folder;
use goymusic_core::FolderId;

use crate::db::Db;
use crate::error::{LibraryError, Result};
use crate::maintenance;

/// Adds a library folder. Rejects folders equal to, inside, or containing an existing one,
/// so no file is ever indexed twice.
pub fn add(db: &Db, path: &Path) -> Result<Folder> {
    let canonical = std::fs::canonicalize(path)?;
    let text = canonical.to_str().ok_or_else(|| LibraryError::NonUtf8Path(canonical.display().to_string()))?.to_owned();
    for existing in list(db)? {
        let existing = Path::new(&existing.path);
        if canonical.starts_with(existing) || existing.starts_with(&canonical) {
            return Err(LibraryError::FolderOverlap(text));
        }
    }
    db.conn().execute("INSERT INTO folders(path) VALUES (?1)", [&text])?;
    Ok(Folder { id: FolderId(db.conn().last_insert_rowid()), path: text })
}

pub fn list(db: &Db) -> Result<Vec<Folder>> {
    let mut stmt = db.conn().prepare("SELECT id, path FROM folders ORDER BY path")?;
    let folders = stmt
        .query_map([], |r| Ok(Folder { id: FolderId(r.get(0)?), path: r.get(1)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(folders)
}

/// Removes a folder and every track indexed from it.
pub fn remove(db: &mut Db, id: FolderId) -> Result<()> {
    let tx = db.conn_mut().transaction()?;
    let track_ids: Vec<i64> = {
        let mut stmt = tx.prepare("SELECT id FROM tracks WHERE folder_id = ?1")?;
        let ids = stmt.query_map([id.0], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        ids
    };
    maintenance::delete_tracks(&tx, &track_ids)?;
    if tx.execute("DELETE FROM folders WHERE id = ?1", [id.0])? == 0 {
        return Err(LibraryError::NotFound { kind: "folder", id: id.0 });
    }
    maintenance::cleanup_orphans(&tx)?;
    tx.commit()?;
    Ok(())
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-library && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (4 tests nuevos). Clippy puede marcar `maintenance` como código sin usar fuera de `folders`: lo usa ya `folders::remove`, así que no hay aviso.

- [ ] **Step 5: Commit**

```bash
git add crates/library
git commit -m "Carpetas de la biblioteca con rechazo de solapamientos"
```

---

### Task 8: Escaneo incremental

**Files:**
- Create: `crates/library/src/scanner.rs`
- Modify: `crates/library/src/lib.rs` (añadir `pub mod scanner;`)
- Test: `crates/library/tests/scan.rs`

**Interfaces:**
- Consumes: `Db` (Task 5), `read_tags`/`TrackTags` (Task 6), `folders` y `maintenance` (Task 7), `goymusic_core::events::{Event, EventBus}` (Task 4).
- Produces: `scanner::scan(&mut Db, &EventBus) -> Result<ScanReport>`; `ScanReport { added, updated, moved, removed, unchanged: usize, errors: Vec<ScanError> }` (`Clone, Debug, Default, PartialEq, Eq`); `ScanError { path: String, message: String }`; `scanner::AUDIO_EXTENSIONS`.
- Produces (interno del crate, lo usa la Task 9): `pub(crate) struct DiskFile { folder_id: i64, path: String, size: i64, mtime: i64 }`, `pub(crate) fn write_track(&Connection, &DiskFile, &TrackTags, existing: Option<i64>) -> Result<i64>` y `pub(crate) fn file_stem(&str) -> String`.
- Eventos: `ScanProgress { done: 0, total }` al empezar, cada 100 archivos leídos y `{ done: total, total }` al terminar; `LibraryUpdated` solo si algo cambió.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/library/tests/scan.rs`:
```rust
mod common;

use common::{write_tagged_wav, write_wav, Meta};
use goymusic_core::events::{Event, EventBus};
use goymusic_library::db::Db;
use goymusic_library::{folders, scanner};

fn count(db: &Db, sql: &str) -> i64 {
    db.conn().query_row(sql, [], |r| r.get(0)).unwrap()
}

fn library(dir: &std::path::Path) -> (Db, EventBus) {
    let db = Db::open_in_memory().unwrap();
    folders::add(&db, dir).unwrap();
    (db, EventBus::new())
}

fn album_track(title: &str) -> Meta<'_> {
    Meta { album: Some("OK Computer"), album_artist: Some("Radiohead"), ..Meta::new(title, "Radiohead") }
}

#[test]
fn scan_adds_tracks_albums_and_artists() {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("cd/01.wav"), 0.3, &album_track("Airbag"));
    write_tagged_wav(&dir.path().join("cd/02.wav"), 0.3, &album_track("Paranoid Android"));
    let (mut db, events) = library(dir.path());

    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!((report.added, report.errors.len()), (2, 0));
    assert_eq!(count(&db, "SELECT count(*) FROM tracks"), 2);
    assert_eq!(count(&db, "SELECT count(*) FROM albums"), 1);
    assert_eq!(count(&db, "SELECT count(*) FROM artists"), 1);
    assert_eq!(count(&db, "SELECT count(*) FROM tracks_fts WHERE tracks_fts MATCH 'airbag'"), 1);
}

#[test]
fn rescan_without_changes_reads_nothing() {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("a.wav"), 0.3, &Meta::new("A", "X"));
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();

    let report = scanner::scan(&mut db, &events).unwrap();
    assert_eq!((report.unchanged, report.added, report.updated, report.removed), (1, 0, 0, 0));
}

#[test]
fn changed_file_is_updated_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    write_tagged_wav(&path, 0.3, &Meta::new("Old", "X"));
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();
    let id = count(&db, "SELECT id FROM tracks");

    write_tagged_wav(&path, 0.3, &Meta::new("A much longer new title", "X"));
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.updated, 1);
    let title: String = db.conn().query_row("SELECT title FROM tracks WHERE id = ?1", [id], |r| r.get(0)).unwrap();
    assert_eq!(title, "A much longer new title");
}

#[test]
fn deleted_file_is_removed_with_orphans() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    write_tagged_wav(&path, 0.3, &album_track("Airbag"));
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();

    std::fs::remove_file(&path).unwrap();
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.removed, 1);
    assert_eq!(count(&db, "SELECT count(*) FROM tracks"), 0);
    assert_eq!(count(&db, "SELECT count(*) FROM albums"), 0);
    assert_eq!(count(&db, "SELECT count(*) FROM artists"), 0);
    assert_eq!(count(&db, "SELECT count(*) FROM tracks_fts"), 0);
}

#[test]
fn untagged_file_uses_file_stem_as_title() {
    let dir = tempfile::tempdir().unwrap();
    write_wav(&dir.path().join("Mystery Song.wav"), 0.3);
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();
    let title: String = db.conn().query_row("SELECT title FROM tracks", [], |r| r.get(0)).unwrap();
    assert_eq!(title, "Mystery Song");
}

#[test]
fn corrupt_file_is_reported_and_scan_continues() {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("good.wav"), 0.3, &Meta::new("Good", "X"));
    std::fs::write(dir.path().join("bad.mp3"), b"not audio at all").unwrap();
    let (mut db, events) = library(dir.path());

    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.added, 1);
    assert_eq!(report.errors.len(), 1);
    assert!(report.errors[0].path.ends_with("bad.mp3"));
}

#[cfg(unix)]
#[test]
fn non_utf8_file_name_is_reported_not_fatal() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    write_wav(&dir.path().join(std::ffi::OsStr::from_bytes(b"bad\xff.wav")), 0.3);
    write_tagged_wav(&dir.path().join("ok.wav"), 0.3, &Meta::new("Ok", "X"));
    let (mut db, events) = library(dir.path());

    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.added, 1);
    assert_eq!(report.errors.len(), 1);
}

#[test]
fn ignores_non_audio_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("cover.jpg"), b"jpeg").unwrap();
    std::fs::write(dir.path().join("notes.txt"), b"text").unwrap();
    let (mut db, events) = library(dir.path());
    let report = scanner::scan(&mut db, &events).unwrap();
    assert_eq!(report, scanner::ScanReport::default());
}

#[test]
fn publishes_progress_and_library_updated_only_on_change() {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("a.wav"), 0.3, &Meta::new("A", "X"));
    write_tagged_wav(&dir.path().join("b.wav"), 0.3, &Meta::new("B", "X"));
    let (mut db, events) = library(dir.path());
    let rx = events.subscribe();

    scanner::scan(&mut db, &events).unwrap();
    let first: Vec<Event> = rx.try_iter().collect();
    assert!(first.contains(&Event::ScanProgress { done: 2, total: 2 }));
    assert!(first.contains(&Event::LibraryUpdated));

    scanner::scan(&mut db, &events).unwrap();
    let second: Vec<Event> = rx.try_iter().collect();
    assert!(!second.contains(&Event::LibraryUpdated));
}

#[test]
fn removing_a_folder_removes_its_tracks() {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("a.wav"), 0.3, &Meta::new("A", "X"));
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();

    let folder = folders::list(&db).unwrap().remove(0);
    folders::remove(&mut db, folder.id).unwrap();

    assert_eq!(count(&db, "SELECT count(*) FROM tracks"), 0);
    assert_eq!(count(&db, "SELECT count(*) FROM tracks_fts"), 0);
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-library --test scan`
Expected: FAIL de compilación (`could not find scanner`).

- [ ] **Step 3: Implementar**

En `crates/library/src/lib.rs` añadir `pub mod scanner;` a la lista de módulos públicos.

`crates/library/src/scanner.rs`:
```rust
//! Incremental scan: walk the library folders, read tags only for new or changed files and
//! keep the database in sync.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::UNIX_EPOCH;

use goymusic_core::events::{Event, EventBus};
use rayon::prelude::*;
use rusqlite::{params, Connection};
use walkdir::WalkDir;

use crate::db::Db;
use crate::error::Result;
use crate::maintenance;
use crate::tags::{read_tags, TrackTags};

pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "oga", "wav", "m4a", "mp4", "aac"];
const PROGRESS_EVERY: usize = 100;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanReport {
    pub added: usize,
    pub updated: usize,
    pub moved: usize,
    pub removed: usize,
    pub unchanged: usize,
    pub errors: Vec<ScanError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanError {
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub(crate) struct DiskFile {
    pub folder_id: i64,
    pub path: String,
    pub size: i64,
    pub mtime: i64,
}

struct Known {
    id: i64,
    size: i64,
    mtime: i64,
}

pub fn scan(db: &mut Db, events: &EventBus) -> Result<ScanReport> {
    let mut report = ScanReport::default();
    let disk = walk_folders(db.conn(), &mut report)?;
    let known = known_tracks(db.conn())?;
    let on_disk: HashSet<&str> = disk.iter().map(|f| f.path.as_str()).collect();

    let mut to_read: Vec<(&DiskFile, Option<i64>)> = Vec::new();
    for file in &disk {
        match known.get(&file.path) {
            Some(k) if k.size == file.size && k.mtime == file.mtime => report.unchanged += 1,
            Some(k) => to_read.push((file, Some(k.id))),
            None => to_read.push((file, None)),
        }
    }
    let missing: Vec<i64> = known.iter().filter(|(path, _)| !on_disk.contains(path.as_str())).map(|(_, k)| k.id).collect();

    let total = to_read.len() as u64;
    events.publish(Event::ScanProgress { done: 0, total });
    let read: Vec<_> = to_read.par_iter().map(|&(file, id)| (file, id, read_tags(Path::new(&file.path)))).collect();

    let tx = db.conn_mut().transaction()?;
    for (done, (file, existing, tags)) in read.into_iter().enumerate() {
        match tags {
            // an unreadable changed file keeps its previous row
            Err(e) => report.errors.push(ScanError { path: file.path.clone(), message: e.to_string() }),
            Ok(tags) => {
                write_track(&tx, file, &tags, existing)?;
                if existing.is_some() {
                    report.updated += 1;
                } else {
                    report.added += 1;
                }
            }
        }
        if (done + 1) % PROGRESS_EVERY == 0 {
            events.publish(Event::ScanProgress { done: done as u64 + 1, total });
        }
    }
    maintenance::delete_tracks(&tx, &missing)?;
    report.removed = missing.len();
    maintenance::cleanup_orphans(&tx)?;
    tx.commit()?;

    events.publish(Event::ScanProgress { done: total, total });
    if report.added + report.updated + report.moved + report.removed > 0 {
        events.publish(Event::LibraryUpdated);
    }
    Ok(report)
}

fn walk_folders(conn: &Connection, report: &mut ScanReport) -> Result<Vec<DiskFile>> {
    let folders: Vec<(i64, String)> = {
        let mut stmt = conn.prepare("SELECT id, path FROM folders")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        rows
    };
    let mut files = Vec::new();
    for (folder_id, root) in folders {
        for entry in WalkDir::new(&root).follow_links(true) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    let path = e.path().map_or_else(|| root.clone(), |p| p.display().to_string());
                    report.errors.push(ScanError { path, message: e.to_string() });
                    continue;
                }
            };
            if !entry.file_type().is_file() || !is_audio(entry.path()) {
                continue;
            }
            let Some(path) = entry.path().to_str() else {
                report.errors.push(ScanError { path: entry.path().display().to_string(), message: "path is not valid UTF-8".into() });
                continue;
            };
            let meta = match entry.metadata() {
                Ok(meta) => meta,
                Err(e) => {
                    report.errors.push(ScanError { path: path.to_owned(), message: e.to_string() });
                    continue;
                }
            };
            let mtime = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
            files.push(DiskFile { folder_id, path: path.to_owned(), size: meta.len() as i64, mtime });
        }
    }
    Ok(files)
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| AUDIO_EXTENSIONS.iter().any(|a| a.eq_ignore_ascii_case(ext)))
}

fn known_tracks(conn: &Connection) -> Result<HashMap<String, Known>> {
    let mut stmt = conn.prepare("SELECT path, id, size, mtime FROM tracks")?;
    let known = stmt
        .query_map([], |r| Ok((r.get(0)?, Known { id: r.get(1)?, size: r.get(2)?, mtime: r.get(3)? })))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(known)
}

/// Inserts a track, or updates the row `existing` (same file changed, or a moved file),
/// and refreshes its artists and FTS row. Returns the track id.
pub(crate) fn write_track(conn: &Connection, file: &DiskFile, tags: &TrackTags, existing: Option<i64>) -> Result<i64> {
    let title = tags.title.clone().unwrap_or_else(|| file_stem(&file.path));
    let album_id = match &tags.album {
        Some(album) => Some(upsert_album(conn, album, tags)?),
        None => None,
    };
    let duration_ms = tags.duration_ms as i64;
    let id = match existing {
        Some(id) => {
            conn.execute(
                "UPDATE tracks SET path = ?1, folder_id = ?2, size = ?3, mtime = ?4, title = ?5, album_id = ?6,
                     album_artist = ?7, track_no = ?8, disc_no = ?9, year = ?10, genre = ?11, duration_ms = ?12,
                     mb_recording_id = ?13
                 WHERE id = ?14",
                params![
                    file.path, file.folder_id, file.size, file.mtime, title, album_id, tags.album_artist, tags.track_no,
                    tags.disc_no, tags.year, tags.genre, duration_ms, tags.mb_recording_id, id
                ],
            )?;
            id
        }
        None => {
            conn.execute(
                "INSERT INTO tracks (path, folder_id, size, mtime, title, album_id, album_artist, track_no, disc_no,
                                     year, genre, duration_ms, mb_recording_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    file.path, file.folder_id, file.size, file.mtime, title, album_id, tags.album_artist, tags.track_no,
                    tags.disc_no, tags.year, tags.genre, duration_ms, tags.mb_recording_id
                ],
            )?;
            conn.last_insert_rowid()
        }
    };
    conn.execute("DELETE FROM track_artists WHERE track_id = ?1", [id])?;
    for (position, name) in tags.artists.iter().enumerate() {
        let artist_id = upsert_artist(conn, name)?;
        conn.execute(
            "INSERT OR IGNORE INTO track_artists (track_id, artist_id, position) VALUES (?1, ?2, ?3)",
            params![id, artist_id, position as i64],
        )?;
    }
    conn.execute("DELETE FROM tracks_fts WHERE rowid = ?1", [id])?;
    conn.execute(
        "INSERT INTO tracks_fts (rowid, title, artists, album) VALUES (?1, ?2, ?3, ?4)",
        params![id, title, tags.artists.join(" "), tags.album.as_deref().unwrap_or("")],
    )?;
    Ok(id)
}

fn upsert_artist(conn: &Connection, name: &str) -> Result<i64> {
    conn.execute("INSERT INTO artists (name) VALUES (?1) ON CONFLICT(name) DO NOTHING", [name])?;
    Ok(conn.query_row("SELECT id FROM artists WHERE name = ?1", [name], |r| r.get(0))?)
}

fn upsert_album(conn: &Connection, title: &str, tags: &TrackTags) -> Result<i64> {
    let artist = tags.album_artist.as_deref().or_else(|| tags.artists.first().map(String::as_str));
    let artist_id = artist.map(|name| upsert_artist(conn, name)).transpose()?;
    let key = album_key(title, artist, tags.mb_release_id.as_deref());
    conn.execute(
        "INSERT INTO albums (title, artist_id, mb_release_id, year, group_key) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(group_key) DO UPDATE SET year = COALESCE(albums.year, excluded.year)",
        params![title, artist_id, tags.mb_release_id, tags.year, key],
    )?;
    Ok(conn.query_row("SELECT id FROM albums WHERE group_key = ?1", [&key], |r| r.get(0))?)
}

/// Album identity: the MusicBrainz release when tagged, else album artist + title
/// (case-insensitive), so compilations stay one album.
fn album_key(title: &str, artist: Option<&str>, mb_release_id: Option<&str>) -> String {
    match mb_release_id {
        Some(id) => format!("mb:{id}"),
        None => format!("t:{}\u{1f}{}", artist.unwrap_or("").trim().to_lowercase(), title.trim().to_lowercase()),
    }
}

pub(crate) fn file_stem(path: &str) -> String {
    Path::new(path).file_stem().map_or_else(|| path.to_owned(), |s| s.to_string_lossy().into_owned())
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-library && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (10 tests nuevos). Si clippy marca `moved` o `file_stem` como sin usar, es aceptable solo hasta la Task 9; para que pase `-D warnings` ahora, `file_stem` ya se usa en `write_track` y `moved` es un campo público, así que no debería haber aviso.

- [ ] **Step 5: Commit**

```bash
git add crates/library
git commit -m "Escaneo incremental de la biblioteca"
```

---

### Task 9: Detección de archivos movidos

**Files:**
- Create: `crates/library/src/moves.rs`
- Modify: `crates/library/src/scanner.rs` (bloque de escritura y borrado dentro de `scan`), `crates/library/src/lib.rs` (añadir `mod moves;`)
- Test: `crates/library/tests/moves.rs`

**Interfaces:**
- Consumes: `DiskFile`, `write_track`, `file_stem` (Task 8); `TrackTags` (Task 6).
- Produces (interno): `moves::MissingTrack`, `moves::load_missing(&Connection, &[i64]) -> Result<Vec<MissingTrack>>`, `moves::take_match(&mut Vec<MissingTrack>, &DiskFile, &TrackTags) -> Option<i64>`. Después de esta tarea, `ScanReport.moved` cuenta los movidos y el ID de la pista se conserva.
- Regla (spec §6): mismo ID de grabación de MusicBrainz; o, si alguno de los dos no lo tiene, mismo tamaño + duración ±1000 ms + título + primer artista + álbum, sin distinguir mayúsculas.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/library/tests/moves.rs`:
```rust
mod common;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_library::db::Db;
use goymusic_library::{folders, scanner};

fn track_id_at(db: &Db, path: &std::path::Path) -> Option<i64> {
    let path = std::fs::canonicalize(path).ok()?;
    db.conn().query_row("SELECT id FROM tracks WHERE path = ?1", [path.to_str().unwrap()], |r| r.get(0)).ok()
}

#[test]
fn moved_file_keeps_its_id() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("a.wav");
    write_tagged_wav(&old, 0.3, &Meta { album: Some("Album"), ..Meta::new("Song", "Artist") });
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    let events = EventBus::new();
    scanner::scan(&mut db, &events).unwrap();
    let id = track_id_at(&db, &old).unwrap();

    let new = dir.path().join("sorted/Artist/a.wav");
    std::fs::create_dir_all(new.parent().unwrap()).unwrap();
    std::fs::rename(&old, &new).unwrap();
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!((report.moved, report.added, report.removed), (1, 0, 0));
    assert_eq!(track_id_at(&db, &new), Some(id));
}

#[test]
fn musicbrainz_id_matches_even_when_tags_changed() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("a.wav");
    write_tagged_wav(&old, 0.3, &Meta { mb_recording_id: Some("rec-1"), ..Meta::new("Song", "Artist") });
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    let events = EventBus::new();
    scanner::scan(&mut db, &events).unwrap();
    let id = track_id_at(&db, &old).unwrap();

    std::fs::remove_file(&old).unwrap();
    let new = dir.path().join("b.wav");
    write_tagged_wav(&new, 0.3, &Meta { mb_recording_id: Some("rec-1"), ..Meta::new("Song (Remastered)", "Artist") });
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.moved, 1);
    assert_eq!(track_id_at(&db, &new), Some(id));
}

#[test]
fn different_file_is_not_a_move() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("a.wav");
    write_tagged_wav(&old, 0.3, &Meta::new("Song", "Artist"));
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    let events = EventBus::new();
    scanner::scan(&mut db, &events).unwrap();

    std::fs::remove_file(&old).unwrap();
    write_tagged_wav(&dir.path().join("b.wav"), 0.3, &Meta::new("Other song", "Artist"));
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!((report.moved, report.added, report.removed), (0, 1, 1));
}

#[test]
fn move_between_library_folders_is_detected() {
    let root = tempfile::tempdir().unwrap();
    let (a, b) = (root.path().join("a"), root.path().join("b"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    write_tagged_wav(&a.join("x.wav"), 0.3, &Meta::new("Song", "Artist"));
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, &a).unwrap();
    folders::add(&db, &b).unwrap();
    let events = EventBus::new();
    scanner::scan(&mut db, &events).unwrap();
    let id = track_id_at(&db, &a.join("x.wav")).unwrap();

    std::fs::rename(a.join("x.wav"), b.join("x.wav")).unwrap();
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.moved, 1);
    assert_eq!(track_id_at(&db, &b.join("x.wav")), Some(id));
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-library --test moves`
Expected: FAIL (`moved_file_keeps_its_id`: `report.moved` es 0 y `added`/`removed` son 1).

- [ ] **Step 3: Implementar**

En `crates/library/src/lib.rs` añadir `mod moves;` junto a `mod maintenance;`.

`crates/library/src/moves.rs`:
```rust
//! Moved-file detection: a "new" file that matches a "missing" one keeps the old track id,
//! so playlists, likes and history survive reorganising the music folders.

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::Result;
use crate::scanner::{file_stem, DiskFile};
use crate::tags::TrackTags;

const DURATION_TOLERANCE_MS: i64 = 1_000;

#[derive(Clone, Debug)]
pub(crate) struct MissingTrack {
    pub id: i64,
    size: i64,
    duration_ms: i64,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    mb_recording_id: Option<String>,
}

pub(crate) fn load_missing(conn: &Connection, ids: &[i64]) -> Result<Vec<MissingTrack>> {
    let mut stmt = conn.prepare(
        "SELECT t.size, t.duration_ms, t.title,
                (SELECT a.name FROM track_artists ta JOIN artists a ON a.id = ta.artist_id
                  WHERE ta.track_id = t.id ORDER BY ta.position LIMIT 1),
                al.title, t.mb_recording_id
           FROM tracks t LEFT JOIN albums al ON al.id = t.album_id
          WHERE t.id = ?1",
    )?;
    let mut missing = Vec::with_capacity(ids.len());
    for &id in ids {
        let row = stmt
            .query_row(params![id], |r| {
                Ok(MissingTrack {
                    id,
                    size: r.get(0)?,
                    duration_ms: r.get(1)?,
                    title: r.get(2)?,
                    artist: r.get(3)?,
                    album: r.get(4)?,
                    mb_recording_id: r.get(5)?,
                })
            })
            .optional()?;
        missing.extend(row);
    }
    Ok(missing)
}

/// Removes and returns the id of the missing track `file` is a move of, if any.
pub(crate) fn take_match(missing: &mut Vec<MissingTrack>, file: &DiskFile, tags: &TrackTags) -> Option<i64> {
    let title = tags.title.clone().unwrap_or_else(|| file_stem(&file.path));
    let position = missing.iter().position(|m| is_same(m, file, tags, &title))?;
    Some(missing.swap_remove(position).id)
}

fn is_same(m: &MissingTrack, file: &DiskFile, tags: &TrackTags, title: &str) -> bool {
    if let (Some(a), Some(b)) = (&m.mb_recording_id, &tags.mb_recording_id) {
        return a == b;
    }
    m.size == file.size
        && (m.duration_ms - tags.duration_ms as i64).abs() <= DURATION_TOLERANCE_MS
        && m.title.to_lowercase() == title.to_lowercase()
        && same_text(m.artist.as_deref(), tags.artists.first().map(String::as_str))
        && same_text(m.album.as_deref(), tags.album.as_deref())
}

fn same_text(a: Option<&str>, b: Option<&str>) -> bool {
    a.map(str::to_lowercase) == b.map(str::to_lowercase)
}
```

En `crates/library/src/scanner.rs`:

1. Añadir `use crate::moves;` a los imports.
2. Sustituir el bloque desde `let tx = db.conn_mut().transaction()?;` hasta `report.removed = missing.len();` (ambos incluidos) por:
```rust
    let tx = db.conn_mut().transaction()?;
    let mut missing = moves::load_missing(&tx, &missing)?;
    for (done, (file, existing, tags)) in read.into_iter().enumerate() {
        match tags {
            // an unreadable changed file keeps its previous row
            Err(e) => report.errors.push(ScanError { path: file.path.clone(), message: e.to_string() }),
            Ok(tags) => match existing {
                Some(id) => {
                    write_track(&tx, file, &tags, Some(id))?;
                    report.updated += 1;
                }
                None => match moves::take_match(&mut missing, file, &tags) {
                    Some(id) => {
                        write_track(&tx, file, &tags, Some(id))?;
                        report.moved += 1;
                    }
                    None => {
                        write_track(&tx, file, &tags, None)?;
                        report.added += 1;
                    }
                },
            },
        }
        if (done + 1) % PROGRESS_EVERY == 0 {
            events.publish(Event::ScanProgress { done: done as u64 + 1, total });
        }
    }
    let removed: Vec<i64> = missing.iter().map(|m| m.id).collect();
    maintenance::delete_tracks(&tx, &removed)?;
    report.removed = removed.len();
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-library && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (4 tests nuevos + los 10 de `scan` siguen pasando).

- [ ] **Step 5: Commit**

```bash
git add crates/library
git commit -m "Detección de archivos movidos conservando el ID de pista"
```

---

### Task 10: Consultas de la biblioteca

**Files:**
- Create: `crates/library/src/queries.rs`
- Modify: `crates/library/src/lib.rs` (añadir `pub mod queries;`)
- Test: `crates/library/tests/queries.rs`

**Interfaces:**
- Consumes: `Db`; modelos de `goymusic_core::model`.
- Produces: `queries::TrackSort { Title, Artist, Album, RecentlyAdded }`; `albums(&Db) -> Result<Vec<Album>>` (por artista y título, sin distinguir mayúsculas); `album(&Db, AlbumId) -> Result<Album>`; `album_tracks(&Db, AlbumId) -> Result<Vec<Track>>` (por disco, número de pista y título); `artists(&Db) -> Result<Vec<Artist>>` (por nombre); `artist_albums(&Db, ArtistId) -> Result<Vec<Album>>`; `tracks(&Db, TrackSort, offset: u32, limit: u32) -> Result<Vec<Track>>`; `track(&Db, TrackId) -> Result<Track>`; `track_count(&Db) -> Result<u64>`. Los inexistentes dan `NotFound { kind: "album" | "track", id }`.
- Produces (interno, lo usan las Tasks 11–13): `pub(crate) const TRACK_SELECT: &str` y `pub(crate) fn track_from_row(&rusqlite::Row) -> rusqlite::Result<Track>`.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/library/tests/queries.rs`:
```rust
mod common;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_core::{AlbumId, TrackId};
use goymusic_library::db::Db;
use goymusic_library::error::LibraryError;
use goymusic_library::queries::{self, TrackSort};
use goymusic_library::{folders, scanner};

fn sample_library() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let ok = |t, n| Meta { album: Some("OK Computer"), album_artist: Some("Radiohead"), track: Some(n), ..Meta::new(t, "Radiohead") };
    write_tagged_wav(&dir.path().join("ok/02.wav"), 0.3, &ok("Paranoid Android", 2));
    write_tagged_wav(&dir.path().join("ok/01.wav"), 0.3, &ok("Airbag", 1));
    write_tagged_wav(&dir.path().join("b/01.wav"), 0.3, &Meta { album: Some("Blackstar"), ..Meta::new("Lazarus", "David Bowie") });
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    scanner::scan(&mut db, &EventBus::new()).unwrap();
    (dir, db)
}

#[test]
fn albums_are_listed_with_artist_and_track_count() {
    let (_dir, db) = sample_library();
    let albums = queries::albums(&db).unwrap();
    let summary: Vec<_> = albums.iter().map(|a| (a.title.as_str(), a.artist.as_deref(), a.track_count)).collect();
    assert_eq!(summary, vec![("Blackstar", Some("David Bowie"), 1), ("OK Computer", Some("Radiohead"), 2)]);
}

#[test]
fn album_tracks_follow_track_numbers() {
    let (_dir, db) = sample_library();
    let ok = queries::albums(&db).unwrap().into_iter().find(|a| a.title == "OK Computer").unwrap();
    let titles: Vec<_> = queries::album_tracks(&db, ok.id).unwrap().into_iter().map(|t| t.title).collect();
    assert_eq!(titles, vec!["Airbag", "Paranoid Android"]);
}

#[test]
fn artists_and_their_albums() {
    let (_dir, db) = sample_library();
    let artists = queries::artists(&db).unwrap();
    assert_eq!(artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), vec!["David Bowie", "Radiohead"]);
    let albums = queries::artist_albums(&db, artists[1].id).unwrap();
    assert_eq!(albums.iter().map(|a| a.title.as_str()).collect::<Vec<_>>(), vec!["OK Computer"]);
}

#[test]
fn tracks_are_sorted_and_paged() {
    let (_dir, db) = sample_library();
    let by_title: Vec<_> = queries::tracks(&db, TrackSort::Title, 0, 10).unwrap().into_iter().map(|t| t.title).collect();
    assert_eq!(by_title, vec!["Airbag", "Lazarus", "Paranoid Android"]);
    let page: Vec<_> = queries::tracks(&db, TrackSort::Title, 1, 1).unwrap().into_iter().map(|t| t.title).collect();
    assert_eq!(page, vec!["Lazarus"]);
    assert_eq!(queries::track_count(&db).unwrap(), 3);
}

#[test]
fn track_has_artists_and_album() {
    let (_dir, db) = sample_library();
    let first = queries::tracks(&db, TrackSort::Title, 0, 1).unwrap().remove(0);
    let track = queries::track(&db, first.id).unwrap();
    assert_eq!(track.artists, vec!["Radiohead".to_owned()]);
    assert_eq!(track.album.as_deref(), Some("OK Computer"));
    assert_eq!(track.track_no, Some(1));
}

#[test]
fn unknown_ids_are_not_found() {
    let (_dir, db) = sample_library();
    assert!(matches!(queries::track(&db, TrackId(999)), Err(LibraryError::NotFound { kind: "track", id: 999 })));
    assert!(matches!(queries::album(&db, AlbumId(999)), Err(LibraryError::NotFound { kind: "album", id: 999 })));
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-library --test queries`
Expected: FAIL de compilación (`could not find queries`).

- [ ] **Step 3: Implementar**

En `crates/library/src/lib.rs` añadir `pub mod queries;`.

`crates/library/src/queries.rs`:
```rust
use goymusic_core::model::{Album, Artist, Track};
use goymusic_core::{AlbumId, ArtistId, TrackId};
use rusqlite::{params, OptionalExtension, Row};

use crate::db::Db;
use crate::error::{LibraryError, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackSort {
    Title,
    Artist,
    Album,
    RecentlyAdded,
}

/// Columns read by [`track_from_row`]. Artists come ordered and joined with U+001F.
pub(crate) const TRACK_SELECT: &str = "
    SELECT t.id, t.path, t.title, al.title, t.album_id, t.album_artist, t.track_no, t.disc_no, t.year, t.genre,
           t.duration_ms,
           (SELECT group_concat(name, char(31)) FROM
               (SELECT a.name AS name FROM track_artists ta JOIN artists a ON a.id = ta.artist_id
                 WHERE ta.track_id = t.id ORDER BY ta.position)) AS artists_joined
      FROM tracks t LEFT JOIN albums al ON al.id = t.album_id";

pub(crate) fn track_from_row(r: &Row) -> rusqlite::Result<Track> {
    let artists: Option<String> = r.get(11)?;
    Ok(Track {
        id: TrackId(r.get(0)?),
        path: r.get(1)?,
        title: r.get(2)?,
        album: r.get(3)?,
        album_id: r.get::<_, Option<i64>>(4)?.map(AlbumId),
        album_artist: r.get(5)?,
        track_no: r.get(6)?,
        disc_no: r.get(7)?,
        year: r.get(8)?,
        genre: r.get(9)?,
        duration_ms: r.get::<_, i64>(10)? as u64,
        artists: artists.map(|s| s.split('\u{1f}').map(str::to_owned).collect()).unwrap_or_default(),
    })
}

const ALBUM_SELECT: &str = "
    SELECT al.id, al.title, ar.name, al.year, (SELECT count(*) FROM tracks t WHERE t.album_id = al.id)
      FROM albums al LEFT JOIN artists ar ON ar.id = al.artist_id";

fn album_from_row(r: &Row) -> rusqlite::Result<Album> {
    Ok(Album { id: AlbumId(r.get(0)?), title: r.get(1)?, artist: r.get(2)?, year: r.get(3)?, track_count: r.get(4)? })
}

pub fn albums(db: &Db) -> Result<Vec<Album>> {
    let sql = format!("{ALBUM_SELECT} ORDER BY ar.name COLLATE NOCASE, al.title COLLATE NOCASE");
    let mut stmt = db.conn().prepare(&sql)?;
    let albums = stmt.query_map([], album_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(albums)
}

pub fn album(db: &Db, id: AlbumId) -> Result<Album> {
    let sql = format!("{ALBUM_SELECT} WHERE al.id = ?1");
    db.conn().query_row(&sql, [id.0], album_from_row).optional()?.ok_or(LibraryError::NotFound { kind: "album", id: id.0 })
}

pub fn album_tracks(db: &Db, id: AlbumId) -> Result<Vec<Track>> {
    let sql = format!("{TRACK_SELECT} WHERE t.album_id = ?1 ORDER BY coalesce(t.disc_no, 1), t.track_no, t.title COLLATE NOCASE");
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks = stmt.query_map([id.0], track_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(tracks)
}

pub fn artists(db: &Db) -> Result<Vec<Artist>> {
    let mut stmt = db.conn().prepare("SELECT id, name FROM artists ORDER BY name COLLATE NOCASE")?;
    let artists = stmt.query_map([], |r| Ok(Artist { id: ArtistId(r.get(0)?), name: r.get(1)? }))?.collect::<rusqlite::Result<_>>()?;
    Ok(artists)
}

pub fn artist_albums(db: &Db, id: ArtistId) -> Result<Vec<Album>> {
    let sql = format!("{ALBUM_SELECT} WHERE al.artist_id = ?1 ORDER BY al.year, al.title COLLATE NOCASE");
    let mut stmt = db.conn().prepare(&sql)?;
    let albums = stmt.query_map([id.0], album_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(albums)
}

pub fn tracks(db: &Db, sort: TrackSort, offset: u32, limit: u32) -> Result<Vec<Track>> {
    let order = match sort {
        TrackSort::Title => "t.title COLLATE NOCASE, t.id",
        TrackSort::Artist => "artists_joined COLLATE NOCASE, al.title COLLATE NOCASE, t.track_no, t.id",
        TrackSort::Album => "al.title COLLATE NOCASE, coalesce(t.disc_no, 1), t.track_no, t.id",
        TrackSort::RecentlyAdded => "t.id DESC",
    };
    let sql = format!("{TRACK_SELECT} ORDER BY {order} LIMIT ?1 OFFSET ?2");
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks = stmt.query_map(params![limit, offset], track_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(tracks)
}

pub fn track(db: &Db, id: TrackId) -> Result<Track> {
    let sql = format!("{TRACK_SELECT} WHERE t.id = ?1");
    db.conn().query_row(&sql, [id.0], track_from_row).optional()?.ok_or(LibraryError::NotFound { kind: "track", id: id.0 })
}

pub fn track_count(db: &Db) -> Result<u64> {
    Ok(db.conn().query_row("SELECT count(*) FROM tracks", [], |r| r.get::<_, i64>(0))? as u64)
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-library && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (6 tests nuevos).

- [ ] **Step 5: Commit**

```bash
git add crates/library
git commit -m "Consultas de álbumes, artistas y pistas"
```

---

### Task 11: Playlists, carpetas de playlists y "Me gusta"

**Files:**
- Create: `crates/library/src/playlists.rs`
- Modify: `crates/library/src/lib.rs` (añadir `pub mod playlists;`)
- Test: `crates/library/tests/playlists.rs`

**Interfaces:**
- Consumes: `Db`, `TRACK_SELECT`, `track_from_row` (Task 10); `PlaylistId`, `PlaylistFolderId`, `TrackId`; `Playlist`, `PlaylistFolder`, `PlaylistItem`, `Track`.
- Produces: `create(&Db, &str) -> Result<Playlist>`, `rename(&Db, PlaylistId, &str) -> Result<()>`, `delete(&Db, PlaylistId) -> Result<()>`, `list(&Db) -> Result<Vec<Playlist>>` (por nombre), `get(&Db, PlaylistId) -> Result<Playlist>`, `items(&Db, PlaylistId) -> Result<Vec<PlaylistItem>>` (posiciones 0..n), `add_tracks(&mut Db, PlaylistId, &[TrackId]) -> Result<()>` (al final; se permiten duplicados), `remove_items(&mut Db, PlaylistId, positions: &[u32]) -> Result<()>`, `move_item(&mut Db, PlaylistId, from: u32, to: u32) -> Result<()>`, `create_folder(&Db, &str, parent: Option<PlaylistFolderId>) -> Result<PlaylistFolder>`, `folders(&Db) -> Result<Vec<PlaylistFolder>>`, `move_to_folder(&Db, PlaylistId, Option<PlaylistFolderId>) -> Result<()>`, `like(&Db, TrackId) -> Result<()>` (idempotente), `unlike(&Db, TrackId) -> Result<()>`, `is_liked(&Db, TrackId) -> Result<bool>`, `liked_tracks(&Db) -> Result<Vec<Track>>` (el último que diste "Me gusta" va primero).
- Errores: playlist inexistente → `NotFound { kind: "playlist", id }`; posición fuera de rango → `InvalidPosition { position, len }`; pista inexistente → `LibraryError::Db` (clave foránea).
- Los eventos `PlaylistsChanged` / `LikesChanged` los publica la capa de servicios en P3, no estas funciones.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/library/tests/playlists.rs`:
```rust
mod common;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_core::{PlaylistId, TrackId};
use goymusic_library::db::Db;
use goymusic_library::error::LibraryError;
use goymusic_library::queries::{self, TrackSort};
use goymusic_library::{folders, playlists, scanner};

/// Three scanned tracks; returns their ids sorted by title: A, B, C.
fn library() -> (tempfile::TempDir, Db, Vec<TrackId>) {
    let dir = tempfile::tempdir().unwrap();
    for title in ["A", "B", "C"] {
        write_tagged_wav(&dir.path().join(format!("{title}.wav")), 0.2, &Meta::new(title, "X"));
    }
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    scanner::scan(&mut db, &EventBus::new()).unwrap();
    let ids = queries::tracks(&db, TrackSort::Title, 0, 10).unwrap().into_iter().map(|t| t.id).collect();
    (dir, db, ids)
}

fn titles(db: &Db, id: PlaylistId) -> Vec<String> {
    playlists::items(db, id).unwrap().into_iter().map(|i| i.track.title).collect()
}

#[test]
fn create_add_and_list() {
    let (_dir, mut db, t) = library();
    let p = playlists::create(&db, "Road trip").unwrap();
    playlists::add_tracks(&mut db, p.id, &[t[2], t[0]]).unwrap();
    assert_eq!(titles(&db, p.id), vec!["C", "A"]);
    let listed = playlists::list(&db).unwrap();
    assert_eq!((listed[0].name.as_str(), listed[0].track_count), ("Road trip", 2));
}

#[test]
fn duplicates_are_allowed() {
    let (_dir, mut db, t) = library();
    let p = playlists::create(&db, "Loop").unwrap();
    playlists::add_tracks(&mut db, p.id, &[t[0], t[0]]).unwrap();
    assert_eq!(titles(&db, p.id), vec!["A", "A"]);
}

#[test]
fn move_and_remove_items_keep_positions_dense() {
    let (_dir, mut db, t) = library();
    let p = playlists::create(&db, "Mix").unwrap();
    playlists::add_tracks(&mut db, p.id, &t).unwrap();
    playlists::move_item(&mut db, p.id, 0, 2).unwrap();
    assert_eq!(titles(&db, p.id), vec!["B", "C", "A"]);
    playlists::remove_items(&mut db, p.id, &[0, 2]).unwrap();
    assert_eq!(titles(&db, p.id), vec!["C"]);
    let positions: Vec<u32> = playlists::items(&db, p.id).unwrap().into_iter().map(|i| i.position).collect();
    assert_eq!(positions, vec![0]);
}

#[test]
fn out_of_range_positions_are_rejected() {
    let (_dir, mut db, t) = library();
    let p = playlists::create(&db, "Mix").unwrap();
    playlists::add_tracks(&mut db, p.id, &t[..1]).unwrap();
    assert!(matches!(playlists::move_item(&mut db, p.id, 0, 5), Err(LibraryError::InvalidPosition { position: 5, len: 1 })));
    assert!(matches!(playlists::remove_items(&mut db, p.id, &[3]), Err(LibraryError::InvalidPosition { position: 3, len: 1 })));
    assert_eq!(titles(&db, p.id), vec!["A"]);
}

#[test]
fn unknown_playlist_or_track_fails_cleanly() {
    let (_dir, mut db, _t) = library();
    assert!(matches!(playlists::get(&db, PlaylistId(77)), Err(LibraryError::NotFound { kind: "playlist", id: 77 })));
    assert!(matches!(playlists::add_tracks(&mut db, PlaylistId(77), &[TrackId(1)]), Err(LibraryError::NotFound { .. })));
    let p = playlists::create(&db, "P").unwrap();
    assert!(matches!(playlists::add_tracks(&mut db, p.id, &[TrackId(9999)]), Err(LibraryError::Db(_))));
}

#[test]
fn rename_delete_and_folders() {
    let (_dir, db, _t) = library();
    let p = playlists::create(&db, "Old").unwrap();
    playlists::rename(&db, p.id, "New").unwrap();
    let folder = playlists::create_folder(&db, "Moods", None).unwrap();
    playlists::move_to_folder(&db, p.id, Some(folder.id)).unwrap();
    let got = playlists::get(&db, p.id).unwrap();
    assert_eq!((got.name.as_str(), got.folder_id), ("New", Some(folder.id)));
    assert_eq!(playlists::folders(&db).unwrap(), vec![folder]);
    playlists::delete(&db, p.id).unwrap();
    assert!(playlists::list(&db).unwrap().is_empty());
}

#[test]
fn likes_are_idempotent_and_newest_first() {
    let (_dir, db, t) = library();
    playlists::like(&db, t[0]).unwrap();
    playlists::like(&db, t[2]).unwrap();
    playlists::like(&db, t[0]).unwrap();
    let liked: Vec<_> = playlists::liked_tracks(&db).unwrap().into_iter().map(|t| t.title).collect();
    assert_eq!(liked, vec!["C", "A"]);
    playlists::unlike(&db, t[2]).unwrap();
    assert!(!playlists::is_liked(&db, t[2]).unwrap());
    assert!(playlists::is_liked(&db, t[0]).unwrap());
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-library --test playlists`
Expected: FAIL de compilación (`could not find playlists`).

- [ ] **Step 3: Implementar**

En `crates/library/src/lib.rs` añadir `pub mod playlists;`.

`crates/library/src/playlists.rs`:
```rust
use std::time::{SystemTime, UNIX_EPOCH};

use goymusic_core::model::{Playlist, PlaylistFolder, PlaylistItem, Track};
use goymusic_core::{PlaylistFolderId, PlaylistId, TrackId};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::db::Db;
use crate::error::{LibraryError, Result};
use crate::queries::{track_from_row, TRACK_SELECT};

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

const PLAYLIST_SELECT: &str = "
    SELECT p.id, p.name, p.folder_id, (SELECT count(*) FROM playlist_items i WHERE i.playlist_id = p.id)
      FROM playlists p";

fn playlist_from_row(r: &Row) -> rusqlite::Result<Playlist> {
    Ok(Playlist {
        id: PlaylistId(r.get(0)?),
        name: r.get(1)?,
        folder_id: r.get::<_, Option<i64>>(2)?.map(PlaylistFolderId),
        track_count: r.get(3)?,
    })
}

fn not_found(id: PlaylistId) -> LibraryError {
    LibraryError::NotFound { kind: "playlist", id: id.0 }
}

fn ensure_exists(conn: &Connection, id: PlaylistId) -> Result<()> {
    conn.query_row("SELECT 1 FROM playlists WHERE id = ?1", [id.0], |_| Ok(())).optional()?.ok_or_else(|| not_found(id))
}

pub fn create(db: &Db, name: &str) -> Result<Playlist> {
    db.conn().execute("INSERT INTO playlists (name, created_at) VALUES (?1, ?2)", params![name, now_secs()])?;
    get(db, PlaylistId(db.conn().last_insert_rowid()))
}

pub fn rename(db: &Db, id: PlaylistId, name: &str) -> Result<()> {
    match db.conn().execute("UPDATE playlists SET name = ?1 WHERE id = ?2", params![name, id.0])? {
        0 => Err(not_found(id)),
        _ => Ok(()),
    }
}

pub fn delete(db: &Db, id: PlaylistId) -> Result<()> {
    match db.conn().execute("DELETE FROM playlists WHERE id = ?1", [id.0])? {
        0 => Err(not_found(id)),
        _ => Ok(()),
    }
}

pub fn list(db: &Db) -> Result<Vec<Playlist>> {
    let sql = format!("{PLAYLIST_SELECT} ORDER BY p.name COLLATE NOCASE, p.id");
    let mut stmt = db.conn().prepare(&sql)?;
    let playlists = stmt.query_map([], playlist_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(playlists)
}

pub fn get(db: &Db, id: PlaylistId) -> Result<Playlist> {
    let sql = format!("{PLAYLIST_SELECT} WHERE p.id = ?1");
    db.conn().query_row(&sql, [id.0], playlist_from_row).optional()?.ok_or_else(|| not_found(id))
}

pub fn items(db: &Db, id: PlaylistId) -> Result<Vec<PlaylistItem>> {
    ensure_exists(db.conn(), id)?;
    let sql = format!("{TRACK_SELECT} JOIN playlist_items i ON i.track_id = t.id WHERE i.playlist_id = ?1 ORDER BY i.pos");
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks: Vec<Track> = stmt.query_map([id.0], track_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(tracks.into_iter().enumerate().map(|(position, track)| PlaylistItem { position: position as u32, track }).collect())
}

fn item_track_ids(conn: &Connection, id: PlaylistId) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT track_id FROM playlist_items WHERE playlist_id = ?1 ORDER BY pos")?;
    let ids = stmt.query_map([id.0], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    Ok(ids)
}

/// Rewrites the playlist with positions 0..n (simpler and safer than shifting positions).
fn rewrite_items(conn: &Connection, id: PlaylistId, track_ids: &[i64]) -> Result<()> {
    conn.execute("DELETE FROM playlist_items WHERE playlist_id = ?1", [id.0])?;
    let mut insert = conn.prepare("INSERT INTO playlist_items (playlist_id, pos, track_id) VALUES (?1, ?2, ?3)")?;
    for (pos, track_id) in track_ids.iter().enumerate() {
        insert.execute(params![id.0, pos as i64, track_id])?;
    }
    Ok(())
}

pub fn add_tracks(db: &mut Db, id: PlaylistId, tracks: &[TrackId]) -> Result<()> {
    let tx = db.conn_mut().transaction()?;
    ensure_exists(&tx, id)?;
    let mut ids = item_track_ids(&tx, id)?;
    ids.extend(tracks.iter().map(|t| t.0));
    rewrite_items(&tx, id, &ids)?;
    tx.commit()?;
    Ok(())
}

pub fn remove_items(db: &mut Db, id: PlaylistId, positions: &[u32]) -> Result<()> {
    let tx = db.conn_mut().transaction()?;
    ensure_exists(&tx, id)?;
    let ids = item_track_ids(&tx, id)?;
    if let Some(&bad) = positions.iter().find(|&&p| p as usize >= ids.len()) {
        return Err(LibraryError::InvalidPosition { position: bad as usize, len: ids.len() });
    }
    let kept: Vec<i64> = ids.into_iter().enumerate().filter(|(i, _)| !positions.contains(&(*i as u32))).map(|(_, t)| t).collect();
    rewrite_items(&tx, id, &kept)?;
    tx.commit()?;
    Ok(())
}

pub fn move_item(db: &mut Db, id: PlaylistId, from: u32, to: u32) -> Result<()> {
    let tx = db.conn_mut().transaction()?;
    ensure_exists(&tx, id)?;
    let mut ids = item_track_ids(&tx, id)?;
    for position in [from, to] {
        if position as usize >= ids.len() {
            return Err(LibraryError::InvalidPosition { position: position as usize, len: ids.len() });
        }
    }
    let track = ids.remove(from as usize);
    ids.insert(to as usize, track);
    rewrite_items(&tx, id, &ids)?;
    tx.commit()?;
    Ok(())
}

pub fn create_folder(db: &Db, name: &str, parent: Option<PlaylistFolderId>) -> Result<PlaylistFolder> {
    db.conn().execute("INSERT INTO playlist_folders (name, parent_id) VALUES (?1, ?2)", params![name, parent.map(|p| p.0)])?;
    Ok(PlaylistFolder { id: PlaylistFolderId(db.conn().last_insert_rowid()), name: name.to_owned(), parent_id: parent })
}

pub fn folders(db: &Db) -> Result<Vec<PlaylistFolder>> {
    let mut stmt = db.conn().prepare("SELECT id, name, parent_id FROM playlist_folders ORDER BY name COLLATE NOCASE, id")?;
    let folders = stmt
        .query_map([], |r| {
            Ok(PlaylistFolder {
                id: PlaylistFolderId(r.get(0)?),
                name: r.get(1)?,
                parent_id: r.get::<_, Option<i64>>(2)?.map(PlaylistFolderId),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(folders)
}

pub fn move_to_folder(db: &Db, id: PlaylistId, folder: Option<PlaylistFolderId>) -> Result<()> {
    match db.conn().execute("UPDATE playlists SET folder_id = ?1 WHERE id = ?2", params![folder.map(|f| f.0), id.0])? {
        0 => Err(not_found(id)),
        _ => Ok(()),
    }
}

pub fn like(db: &Db, track: TrackId) -> Result<()> {
    db.conn().execute("INSERT OR IGNORE INTO likes (track_id, liked_at) VALUES (?1, ?2)", params![track.0, now_secs()])?;
    Ok(())
}

pub fn unlike(db: &Db, track: TrackId) -> Result<()> {
    db.conn().execute("DELETE FROM likes WHERE track_id = ?1", [track.0])?;
    Ok(())
}

pub fn is_liked(db: &Db, track: TrackId) -> Result<bool> {
    Ok(db.conn().query_row("SELECT 1 FROM likes WHERE track_id = ?1", [track.0], |_| Ok(())).optional()?.is_some())
}

/// Liked tracks, most recently liked first (`likes.id` grows with every like).
pub fn liked_tracks(db: &Db) -> Result<Vec<Track>> {
    let sql = format!("{TRACK_SELECT} JOIN likes l ON l.track_id = t.id ORDER BY l.id DESC");
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks = stmt.query_map([], track_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(tracks)
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-library && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (7 tests nuevos).

- [ ] **Step 5: Commit**

```bash
git add crates/library
git commit -m "Playlists, carpetas de playlists y Me gusta"
```

---

### Task 12: Importar y exportar M3U8

**Files:**
- Create: `crates/library/src/m3u.rs`
- Modify: `crates/library/src/lib.rs` (añadir `pub mod m3u;`)
- Test: `crates/library/tests/m3u.rs`

**Interfaces:**
- Consumes: `playlists::{create, add_tracks, get, items}` (Task 11).
- Produces: `m3u::ImportReport { playlist: Playlist, matched: usize, missing: Vec<String> }` (`Debug`); `m3u::import(&mut Db, name: &str, m3u: &Path) -> Result<ImportReport>`; `m3u::export(&Db, PlaylistId, &Path) -> Result<()>`.
- Reglas: ignora líneas vacías y las que empiezan por `#` (también el BOM UTF-8); resuelve rutas relativas desde la carpeta del `.m3u`; una entrada que no está en la biblioteca se anota en `missing` sin abortar. La exportación escribe `#EXTM3U`, `#EXTINF:<segundos>,<artistas> - <título>` y la ruta absoluta.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/library/tests/m3u.rs`:
```rust
mod common;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_library::db::Db;
use goymusic_library::{folders, m3u, playlists, scanner};

fn library() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("music/a.wav"), 1.0, &Meta::new("Alpha", "Artist"));
    write_tagged_wav(&dir.path().join("music/b.wav"), 1.0, &Meta::new("Beta", "Artist"));
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, &dir.path().join("music")).unwrap();
    scanner::scan(&mut db, &EventBus::new()).unwrap();
    (dir, db)
}

#[test]
fn import_resolves_relative_paths_and_reports_missing() {
    let (dir, mut db) = library();
    let list = dir.path().join("lists/mine.m3u8");
    std::fs::create_dir_all(list.parent().unwrap()).unwrap();
    std::fs::write(&list, "\u{feff}#EXTM3U\n#EXTINF:1,Artist - Beta\n../music/b.wav\r\n\n../music/a.wav\nnowhere/c.mp3\n").unwrap();

    let report = m3u::import(&mut db, "Imported", &list).unwrap();

    assert_eq!(report.matched, 2);
    assert_eq!(report.missing, vec!["nowhere/c.mp3".to_owned()]);
    let titles: Vec<_> = playlists::items(&db, report.playlist.id).unwrap().into_iter().map(|i| i.track.title).collect();
    assert_eq!(titles, vec!["Beta", "Alpha"]);
    assert_eq!(report.playlist.track_count, 2);
}

#[test]
fn export_writes_extended_m3u_that_imports_back() {
    let (dir, mut db) = library();
    let list = dir.path().join("first.m3u8");
    std::fs::write(&list, "music/a.wav\nmusic/b.wav\n").unwrap();
    let first = m3u::import(&mut db, "First", &list).unwrap();

    let out = dir.path().join("out/exported.m3u8");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    m3u::export(&db, first.playlist.id, &out).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("#EXTM3U\n"));
    assert!(text.contains("#EXTINF:1,Artist - Alpha\n"));

    let again = m3u::import(&mut db, "Again", &out).unwrap();
    assert_eq!((again.matched, again.missing.len()), (2, 0));
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-library --test m3u`
Expected: FAIL de compilación (`could not find m3u`).

- [ ] **Step 3: Implementar**

En `crates/library/src/lib.rs` añadir `pub mod m3u;`.

`crates/library/src/m3u.rs`:
```rust
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use goymusic_core::model::Playlist;
use goymusic_core::{PlaylistId, TrackId};
use rusqlite::OptionalExtension;

use crate::db::Db;
use crate::error::Result;
use crate::playlists;

#[derive(Debug)]
pub struct ImportReport {
    pub playlist: Playlist,
    pub matched: usize,
    /// Entries (as written in the file) not found in the library.
    pub missing: Vec<String>,
}

/// Creates playlist `name` from an M3U/M3U8 file. Relative entries resolve from the file's
/// directory; entries not in the library are reported, not fatal.
pub fn import(db: &mut Db, name: &str, m3u: &Path) -> Result<ImportReport> {
    let text = std::fs::read_to_string(m3u)?;
    let base = m3u.parent().unwrap_or_else(|| Path::new("."));
    let mut tracks = Vec::new();
    let mut missing = Vec::new();
    for entry in text.lines().map(|l| l.trim_start_matches('\u{feff}').trim()) {
        if entry.is_empty() || entry.starts_with('#') {
            continue;
        }
        let candidate = if Path::new(entry).is_absolute() { PathBuf::from(entry) } else { base.join(entry) };
        let resolved = std::fs::canonicalize(&candidate).ok().and_then(|p| p.to_str().map(str::to_owned));
        let id = match resolved {
            Some(path) => db.conn().query_row("SELECT id FROM tracks WHERE path = ?1", [&path], |r| r.get::<_, i64>(0)).optional()?,
            None => None,
        };
        match id {
            Some(id) => tracks.push(TrackId(id)),
            None => missing.push(entry.to_owned()),
        }
    }
    let created = playlists::create(db, name)?;
    playlists::add_tracks(db, created.id, &tracks)?;
    Ok(ImportReport { playlist: playlists::get(db, created.id)?, matched: tracks.len(), missing })
}

/// Writes the playlist as extended M3U8 with absolute paths.
pub fn export(db: &Db, id: PlaylistId, path: &Path) -> Result<()> {
    let mut out = String::from("#EXTM3U\n");
    for item in playlists::items(db, id)? {
        let track = &item.track;
        let seconds = (track.duration_ms + 500) / 1000; // rounded: 999 ms is "1", not "0"
        let _ = writeln!(out, "#EXTINF:{seconds},{} - {}", track.artists.join(", "), track.title);
        let _ = writeln!(out, "{}", track.path);
    }
    std::fs::write(path, out)?;
    Ok(())
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-library && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (2 tests nuevos).

- [ ] **Step 5: Commit**

```bash
git add crates/library
git commit -m "Importar y exportar playlists M3U8"
```

---

### Task 13: Búsqueda instantánea con tolerancia a erratas

**Files:**
- Create: `crates/library/src/search.rs`
- Modify: `crates/library/src/lib.rs` (añadir `pub mod search;`)
- Test: `crates/library/tests/search.rs`

**Interfaces:**
- Consumes: `TRACK_SELECT`, `track_from_row`, `queries::{album, track}` (Task 10); la tabla `tracks_fts` que mantiene el escáner (Task 8).
- Produces: `search::search(&Db, query: &str, limit: usize) -> Result<SearchResults>`; `SearchResults { top: Option<TopResult>, tracks: Vec<Track>, albums: Vec<Album>, artists: Vec<Artist> }` (`Clone, Debug, Default, PartialEq`); `TopResult { Artist(Artist), Album(Album), Track(Track) }`; `FUZZY_THRESHOLD: f64 = 0.85`; `FUZZY_BELOW: usize = 5`.
- Reglas: la consulta se divide en palabras alfanuméricas, así que la sintaxis de FTS5 nunca llega al motor. Cada palabra se busca por prefijo y todas deben aparecer. Si el total de resultados es menor que `FUZZY_BELOW`, se añaden coincidencias por Jaro-Winkler ≥ `FUZZY_THRESHOLD` sobre nombres de artistas, álbumes y títulos, sin acentos y en minúsculas. Mejor resultado: un artista cuyo nombre empieza por la consulta; si no, el primer álbum; si no, la primera canción; si no, el primer artista.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/library/tests/search.rs`:
```rust
mod common;

use std::time::Instant;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_library::db::Db;
use goymusic_library::search::{search, TopResult};
use goymusic_library::{folders, scanner};
use rusqlite::params;

fn library() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let ok = |t| Meta { album: Some("OK Computer"), album_artist: Some("Radiohead"), ..Meta::new(t, "Radiohead") };
    write_tagged_wav(&dir.path().join("1.wav"), 0.2, &ok("Airbag"));
    write_tagged_wav(&dir.path().join("2.wav"), 0.2, &ok("Paranoid Android"));
    write_tagged_wav(&dir.path().join("3.wav"), 0.2, &Meta { album: Some("Lemonade"), ..Meta::new("Formation", "Beyoncé") });
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    scanner::scan(&mut db, &EventBus::new()).unwrap();
    (dir, db)
}

#[test]
fn prefix_matches_while_typing() {
    let (_dir, db) = library();
    let r = search(&db, "radi", 10).unwrap();
    assert_eq!(r.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), vec!["Radiohead"]);
    assert_eq!(r.tracks.len(), 2);
    assert!(matches!(r.top, Some(TopResult::Artist(ref a)) if a.name == "Radiohead"));
}

#[test]
fn several_words_must_all_match() {
    let (_dir, db) = library();
    let r = search(&db, "paranoid andr", 10).unwrap();
    assert_eq!(r.tracks.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(), vec!["Paranoid Android"]);
}

#[test]
fn album_search() {
    let (_dir, db) = library();
    let r = search(&db, "ok comp", 10).unwrap();
    assert_eq!(r.albums.iter().map(|a| a.title.as_str()).collect::<Vec<_>>(), vec!["OK Computer"]);
}

#[test]
fn typo_finds_artist_through_fuzzy_fallback() {
    let (_dir, db) = library();
    let r = search(&db, "radioheaf", 10).unwrap();
    assert!(r.artists.iter().any(|a| a.name == "Radiohead"), "{r:?}");
}

#[test]
fn accents_are_ignored() {
    let (_dir, db) = library();
    let r = search(&db, "beyonce", 10).unwrap();
    assert!(r.artists.iter().any(|a| a.name == "Beyoncé"), "{r:?}");
    assert_eq!(r.tracks.len(), 1);
}

#[test]
fn fts_syntax_is_treated_as_text() {
    let (_dir, db) = library();
    for query in ["\"", "*", "AND", "air -bag", "(paranoid", "NEAR(a b)", "\"unclosed", "''"] {
        assert!(search(&db, query, 10).is_ok(), "query {query:?} failed");
    }
}

#[test]
fn empty_query_returns_nothing() {
    let (_dir, db) = library();
    assert_eq!(search(&db, "   ", 10).unwrap(), Default::default());
}

/// Spec target: < 50 ms with 50 000 tracks. Run with `cargo test --release -- --ignored`.
#[test]
#[ignore]
fn search_is_fast_on_a_large_library() {
    let mut db = Db::open_in_memory().unwrap();
    {
        let tx = db.conn_mut().transaction().unwrap();
        tx.execute("INSERT INTO folders (id, path) VALUES (1, '/synthetic')", []).unwrap();
        for i in 0..50_000i64 {
            let artist = format!("Artist {}", i % 2_000);
            let title = format!("Song number {i} love");
            tx.execute("INSERT OR IGNORE INTO artists (name) VALUES (?1)", [&artist]).unwrap();
            let artist_id: i64 = tx.query_row("SELECT id FROM artists WHERE name = ?1", [&artist], |r| r.get(0)).unwrap();
            tx.execute(
                "INSERT INTO tracks (id, path, folder_id, size, mtime, title, duration_ms) VALUES (?1, ?2, 1, 1, 1, ?3, 1000)",
                params![i + 1, format!("/synthetic/{i}.mp3"), title],
            )
            .unwrap();
            tx.execute("INSERT INTO track_artists (track_id, artist_id, position) VALUES (?1, ?2, 0)", params![i + 1, artist_id]).unwrap();
            tx.execute("INSERT INTO tracks_fts (rowid, title, artists, album) VALUES (?1, ?2, ?3, '')", params![i + 1, title, artist]).unwrap();
        }
        tx.commit().unwrap();
    }
    for query in ["love", "artist 19", "artsit 1999"] {
        let start = Instant::now();
        search(&db, query, 20).unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 50, "{query:?} took {elapsed:?}");
    }
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-library --test search`
Expected: FAIL de compilación (`could not find search`).

- [ ] **Step 3: Implementar**

En `crates/library/src/lib.rs` añadir `pub mod search;`.

`crates/library/src/search.rs`:
```rust
//! Instant search: FTS5 prefix matching, plus a Jaro-Winkler fallback for typos when FTS
//! finds little.

use std::collections::HashSet;

use deunicode::deunicode;
use goymusic_core::model::{Album, Artist, Track};
use goymusic_core::{AlbumId, ArtistId, TrackId};
use rusqlite::params;
use strsim::jaro_winkler;

use crate::db::Db;
use crate::error::Result;
use crate::queries::{self, track_from_row, TRACK_SELECT};

pub const FUZZY_THRESHOLD: f64 = 0.85;
pub const FUZZY_BELOW: usize = 5;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchResults {
    pub top: Option<TopResult>,
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TopResult {
    Artist(Artist),
    Album(Album),
    Track(Track),
}

pub fn search(db: &Db, query: &str, limit: usize) -> Result<SearchResults> {
    let tokens = tokenize(query);
    if tokens.is_empty() {
        return Ok(SearchResults::default());
    }
    let any_column = fts_query(&tokens);
    let mut results = SearchResults {
        tracks: fts_tracks(db, &any_column, limit)?,
        albums: fts_albums(db, &format!("album : ({any_column})"), limit)?,
        artists: fts_artists(db, &format!("artists : ({any_column})"), &tokens, limit)?,
        top: None,
    };
    if results.tracks.len() + results.albums.len() + results.artists.len() < FUZZY_BELOW {
        fuzzy_fill(db, &normalize(query), limit, &mut results)?;
    }
    results.top = pick_top(&tokens.join(" "), &results);
    Ok(results)
}

/// Lowercase, accent-free words. Only alphanumerics survive, so FTS5 syntax
/// (quotes, `*`, `-`, parentheses, operators) can never reach the query engine.
fn tokenize(query: &str) -> Vec<String> {
    normalize(query).split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).map(str::to_owned).collect()
}

fn normalize(text: &str) -> String {
    deunicode(text).to_lowercase()
}

/// Every token as a quoted prefix phrase: `"radio"* "head"*` (implicit AND).
fn fts_query(tokens: &[String]) -> String {
    tokens.iter().map(|t| format!("\"{t}\"*")).collect::<Vec<_>>().join(" ")
}

fn fts_tracks(db: &Db, fts: &str, limit: usize) -> Result<Vec<Track>> {
    let sql = format!("{TRACK_SELECT} JOIN tracks_fts ON tracks_fts.rowid = t.id WHERE tracks_fts MATCH ?1 ORDER BY tracks_fts.rank LIMIT ?2");
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks = stmt.query_map(params![fts, limit as i64], track_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(tracks)
}

fn fts_albums(db: &Db, fts: &str, limit: usize) -> Result<Vec<Album>> {
    let mut stmt = db.conn().prepare(
        "SELECT DISTINCT t.album_id FROM tracks_fts JOIN tracks t ON t.id = tracks_fts.rowid
          WHERE tracks_fts MATCH ?1 AND t.album_id IS NOT NULL LIMIT ?2",
    )?;
    let ids: Vec<i64> = stmt.query_map(params![fts, limit as i64], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    ids.into_iter().map(|id| queries::album(db, AlbumId(id))).collect()
}

/// Artists of matching tracks whose own name matches (a track also lists its featured artists).
fn fts_artists(db: &Db, fts: &str, tokens: &[String], limit: usize) -> Result<Vec<Artist>> {
    let mut stmt = db.conn().prepare(
        "SELECT DISTINCT a.id, a.name FROM tracks_fts
           JOIN track_artists ta ON ta.track_id = tracks_fts.rowid
           JOIN artists a ON a.id = ta.artist_id
          WHERE tracks_fts MATCH ?1",
    )?;
    let all: Vec<Artist> = stmt.query_map([fts], |r| Ok(Artist { id: ArtistId(r.get(0)?), name: r.get(1)? }))?.collect::<rusqlite::Result<_>>()?;
    Ok(all
        .into_iter()
        .filter(|a| {
            let words = tokenize(&a.name);
            tokens.iter().all(|t| words.iter().any(|w| w.starts_with(t.as_str())))
        })
        .take(limit)
        .collect())
}

fn fuzzy_fill(db: &Db, query: &str, limit: usize, results: &mut SearchResults) -> Result<()> {
    let artists = fuzzy_names(db, "SELECT id, name FROM artists", query)?;
    let known: HashSet<ArtistId> = results.artists.iter().map(|a| a.id).collect();
    for (id, name) in artists {
        if results.artists.len() >= limit {
            break;
        }
        if !known.contains(&ArtistId(id)) {
            results.artists.push(Artist { id: ArtistId(id), name });
        }
    }

    let albums = fuzzy_names(db, "SELECT id, title FROM albums", query)?;
    let known: HashSet<AlbumId> = results.albums.iter().map(|a| a.id).collect();
    for (id, _) in albums {
        if results.albums.len() >= limit {
            break;
        }
        if !known.contains(&AlbumId(id)) {
            results.albums.push(queries::album(db, AlbumId(id))?);
        }
    }

    let tracks = fuzzy_names(db, "SELECT id, title FROM tracks", query)?;
    let known: HashSet<TrackId> = results.tracks.iter().map(|t| t.id).collect();
    for (id, _) in tracks {
        if results.tracks.len() >= limit {
            break;
        }
        if !known.contains(&TrackId(id)) {
            results.tracks.push(queries::track(db, TrackId(id))?);
        }
    }
    Ok(())
}

/// `(id, name)` rows whose normalized name scores ≥ FUZZY_THRESHOLD, best first.
fn fuzzy_names(db: &Db, sql: &str, query: &str) -> Result<Vec<(i64, String)>> {
    let mut stmt = db.conn().prepare(sql)?;
    let rows: Vec<(i64, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
    let mut scored: Vec<(f64, i64, String)> = rows
        .into_iter()
        .filter_map(|(id, name)| {
            let score = jaro_winkler(query, &normalize(&name));
            (score >= FUZZY_THRESHOLD).then_some((score, id, name))
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    Ok(scored.into_iter().map(|(_, id, name)| (id, name)).collect())
}

fn pick_top(query: &str, results: &SearchResults) -> Option<TopResult> {
    if let Some(artist) = results.artists.iter().find(|a| normalize(&a.name).starts_with(query)) {
        return Some(TopResult::Artist(artist.clone()));
    }
    results
        .albums
        .first()
        .map(|a| TopResult::Album(a.clone()))
        .or_else(|| results.tracks.first().map(|t| TopResult::Track(t.clone())))
        .or_else(|| results.artists.first().map(|a| TopResult::Artist(a.clone())))
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo test -p goymusic-library && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (7 tests; 1 ignorado).

Run: `cargo test -p goymusic-library --release --test search -- --ignored`
Expected: PASS (`search_is_fast_on_a_large_library`). Si falla por tiempo solo en la consulta con errata, la causa es la pasada difusa sobre 50 000 títulos: anotarlo en el commit y abrir una tarea en P3 para precalcular los nombres normalizados. No relajar el umbral sin consultarlo.

- [ ] **Step 5: Commit**

```bash
git add crates/library
git commit -m "Búsqueda instantánea con FTS5 y tolerancia a erratas"
```

---

### Task 14: Estado persistido de la cola y ajustes

**Files:**
- Create: `crates/library/src/state.rs`
- Modify: `crates/library/src/lib.rs` (añadir `pub mod state;`)
- Test: `crates/library/tests/state.rs`

**Interfaces:**
- Consumes: `goymusic_core::queue::Queue` (serializable, Task 3); `Db`.
- Produces: `state::save_queue(&Db, &Queue, position_ms: u64) -> Result<()>`; `state::load_queue(&Db) -> Result<Option<(Queue, u64)>>`; `state::set_setting(&Db, key: &str, value: &str) -> Result<()>`; `state::setting(&Db, key: &str) -> Result<Option<String>>`. P3 restaura la cola al abrir, en pausa (spec §5).

- [ ] **Step 1: Escribir los tests que fallan**

`crates/library/tests/state.rs`:
```rust
use goymusic_core::queue::Queue;
use goymusic_core::TrackId;
use goymusic_library::db::Db;
use goymusic_library::state;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn nothing_saved_yet() {
    let db = Db::open_in_memory().unwrap();
    assert!(state::load_queue(&db).unwrap().is_none());
    assert_eq!(state::setting(&db, "theme").unwrap(), None);
}

#[test]
fn queue_and_position_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.db");
    let mut queue = Queue::new();
    queue.play_context((1..=5).map(TrackId).collect(), 2, &mut StdRng::seed_from_u64(1));
    queue.queue_add(&[TrackId(42)]);
    {
        let db = Db::open(&path).unwrap();
        state::save_queue(&db, &queue, 61_500).unwrap();
        state::save_queue(&db, &queue, 62_000).unwrap(); // overwrites, single row
    }
    let db = Db::open(&path).unwrap();
    assert_eq!(state::load_queue(&db).unwrap(), Some((queue, 62_000)));
}

#[test]
fn settings_overwrite() {
    let db = Db::open_in_memory().unwrap();
    state::set_setting(&db, "theme", "dark").unwrap();
    state::set_setting(&db, "theme", "light").unwrap();
    assert_eq!(state::setting(&db, "theme").unwrap().as_deref(), Some("light"));
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-library --test state`
Expected: FAIL de compilación (`could not find state`).

- [ ] **Step 3: Implementar**

En `crates/library/src/lib.rs`, la lista final de módulos queda:
```rust
pub mod db;
pub mod error;
pub mod folders;
pub mod m3u;
pub mod playlists;
pub mod queries;
pub mod scanner;
pub mod search;
pub mod state;
pub mod tags;

mod maintenance;
mod moves;
```

`crates/library/src/state.rs`:
```rust
use goymusic_core::queue::Queue;
use rusqlite::{params, OptionalExtension};

use crate::db::Db;
use crate::error::Result;

/// Stores the queue and playback position (one row, overwritten on every save).
pub fn save_queue(db: &Db, queue: &Queue, position_ms: u64) -> Result<()> {
    let snapshot = serde_json::to_string(queue)?;
    db.conn().execute(
        "INSERT INTO queue_state (id, snapshot, position_ms) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET snapshot = excluded.snapshot, position_ms = excluded.position_ms",
        params![snapshot, position_ms as i64],
    )?;
    Ok(())
}

pub fn load_queue(db: &Db) -> Result<Option<(Queue, u64)>> {
    let row: Option<(String, i64)> = db
        .conn()
        .query_row("SELECT snapshot, position_ms FROM queue_state WHERE id = 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    row.map(|(snapshot, position)| Ok((serde_json::from_str(&snapshot)?, position as u64))).transpose()
}

pub fn set_setting(db: &Db, key: &str, value: &str) -> Result<()> {
    db.conn().execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn setting(db: &Db, key: &str) -> Result<Option<String>> {
    Ok(db.conn().query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional()?)
}
```

`load_queue` devuelve un `Queue` reconstruido de JSON. Para comparar en el test hace falta `Queue: PartialEq` (ya derivado en la Task 2).

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS de todo el workspace.

- [ ] **Step 5: Commit**

```bash
git add crates/library
git commit -m "Estado persistido de la cola y ajustes"
```

---

## Cobertura de la spec en P1

| Spec | Tarea |
|---|---|
| §3 arquitectura (core sin UI, crates) | 1, 5 |
| §3 eventos `LibraryUpdated`, `ScanProgress`, `PlaylistsChanged`, `LikesChanged`, `QueueChanged` | 4, 8 (eventos de reproductor → P2) |
| §4 Library | 7, 8, 10 |
| §4 Playlists (CRUD, carpetas, Me gusta, M3U) | 11, 12 |
| §4 Search | 13 |
| §5 lógica de cola (contexto, cola de usuario, shuffle, repeat, anterior > 3 s, persistencia) | 2, 3, 14 |
| §6 escaneo incremental, etiquetas, títulos desde nombre de archivo | 6, 8 |
| §6 identidad estable / movidos | 9 |
| §6 agrupación de álbumes, SQLite WAL + migraciones | 5, 8 |
| §6 búsqueda FTS5 + difusa, < 50 ms | 13 |
| §9 errores tipados, tests con audio generado | 5–14 |
| §6 vigilancia de cambios, carátulas `art://` | P3 |
| §5 motor de audio, gapless, ReplayGain | P2 |
| §7 UI, §8 escritorio, §9 logs/empaquetado/CI completo | P3, P4 |
