# GoyMusic P2 — Motor de audio: plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Crate `goymusic-audio`: reproduce la cola del núcleo desde archivos locales (MP3, FLAC, Ogg Vorbis, WAV, AAC/M4A, ALAC) con transición gapless, resampleo, volumen, ReplayGain opcional, recuperación ante errores de archivo y de dispositivo, en un hilo propio con la API del servicio Player (spec §4).

**Architecture:** Cadena archivo → `Decoder` (symphonia) → `Converter` (mapa de canales + rubato) → ReplayGain → `AudioSink`. El `Engine` es una máquina de estados **síncrona y determinista** que posee la `Queue` del núcleo y se prueba llamando a `pump()` con un `CaptureSink` en memoria; `Player` lo ejecuta en un hilo y recibe órdenes por canal. Gapless: con ≤ 10 s por decodificar se abre la siguiente pista (`Queue::peek_next`), y al acabar la actual sus muestras se escriben justo detrás en el mismo búfer. La cola solo avanza (`on_track_ended`) cuando la siguiente empieza a **oírse**, así que `queue.current()` siempre es lo que suena. `CpalSink` es la salida real: búfer circular sin bloqueos (ringbuf) leído por el callback de cpal.

**Tech Stack:** Rust stable (edition 2021); symphonia 0.5.5 (features `mp3 flac ogg vorbis wav pcm aac isomp4 alac`), rubato 0.16.2, cpal 0.15.3 (ALSA), ringbuf 0.4.8, lofty 0.21, rand 0.8, thiserror 2; tests con hound 3, flacenc 0.5.1 (FLAC generado en el test) y tempfile 3.

**Spec:** `docs/superpowers/specs/2026-10-16-goymusic-design.md` (§3 eventos, §4 Player, §5 motor de audio, §9 errores y pruebas, §10 criterio 3).

**Origen del código:** todo el código de este plan se escribió y verificó antes en un prototipo (`.superpowers/p2-proto/`, ignorado por git): 55 tests en verde, más 2 `#[ignore]` comprobados en esta máquina (dispositivo real y MP3 vía ffmpeg); clippy y fmt limpios.

**Hoja de ruta:** P3 conecta `Player` con la biblioteca (resolver `TrackId → ruta` y guardado/restauración con `state`), vigilancia de carpetas, app Tauri y UI. P4: MPRIS, notificaciones, bandeja, empaquetado.

## Global Constraints

- Crate `goymusic-audio` en `crates/audio`; `version`, `edition` y `license` heredados del workspace (`GPL-3.0-or-later`). Del workspace solo depende de `goymusic-core`.
- Versiones fijadas con `=` como en "Tech Stack". Si una API no cuadra, consultar docs.rs de **esa** versión; no cambiar la versión.
- Dependencia de sistema de cpal en Linux: `libasound2-dev` y `pkg-config` (en CI se instalan con apt).
- Ningún audio con copyright en el repo: los tests generan WAV (hound) y FLAC (flacenc); el test de MP3 lo codifica con ffmpeg y está marcado `#[ignore]`.
- Umbrales: precarga con ≤ 10 000 ms por decodificar; búfer objetivo 500 ms; `PositionChanged` cada 250 ms (~4 Hz); reintento de dispositivo cada 2 s; "anterior" reinicia con posición > 3000 ms (`RESTART_THRESHOLD_MS` del núcleo).
- Gapless: activar `enable_gapless` en symphonia y **no** recortar a mano (los decodificadores MP3 y Vorbis ya recortan retardo y relleno; hacerlo otra vez quitaría audio).
- Al terminar cada tarea deben pasar `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
- Commits en español. Nada de código, esquemas ni nombres de Spotify.
- Flujo de commits: el hook git-safety crea commits `checkpoint:` tras cada edición. Al cerrar cada tarea: `git reset --soft <commit base de la tarea>` y un único commit con el mensaje del paso final.

## Review Focus

1. **Pista ilegible, inexistente o que ya no está en la biblioteca, en mitad de la cola**: se emite `PlaybackError { track, reason }` y suena la siguiente; nunca pánico ni silencio indefinido. Test en la Task 6 (`unreadable_and_unknown_tracks_are_reported_and_skipped`).
2. **Cola entera de archivos rotos con repeat all/one**: termina en `Ended` tras probar cada pista una vez, en vez de un bucle infinito que bloquea el hilo. Test en la Task 6 (`a_queue_of_broken_files_on_repeat_ends_instead_of_looping`).
3. **Editar la cola cuando la siguiente ya está precargada o ya está en el búfer**: suena la nueva siguiente, sin perder ni repetir audio de la actual. Tests en la Task 6 (`editing_the_queue_replaces_the_preloaded_track`, `queue_edit_while_the_successor_is_buffered_restarts_cleanly`).
4. **Auriculares/DAC desconectados en plena reproducción**: pausa, `AudioDeviceLost`, reintento con el dispositivo por defecto y reanudación en la misma muestra. Test en la Task 6 (`lost_device_pauses_and_resumes_on_the_default_device`).
5. **Archivos con frecuencia o canales distintos a los del dispositivo** (mono 22 kHz, 44,1 kHz en salida de 48 kHz): sin cambio de tono ni de duración, y sin hueco en la unión gapless. Tests en la Task 3 (`resampling_keeps_duration_and_pitch`) y la Task 6 (`gapless_through_the_resampler_has_no_silence`).

## Limitaciones conocidas (aceptadas en v1)

- **AAC/M4A**: symphonia 0.5 no aplica el recorte gapless de iTunes (`iTunSMPB`); entre pistas AAC puede quedar un hueco de unos 20–40 ms. El criterio 3 de la spec cubre FLAC y MP3, y ambos se verifican por test.
- **ReplayGain** se aplica al decodificar, así que activarlo o desactivarlo afecta a partir de la siguiente pista. El volumen se aplica en la salida y se oye al instante.
- En los últimos ~500 ms de una pista, la siguiente ya puede estar en el búfer. Si en ese momento se edita la cola y la siguiente cambia, el motor reinicia la pista actual en su posición: un salto mínimo, en un caso raro.

---

## Mapa de ficheros

```
Cargo.toml                         + crates/audio en members, dependencias de audio
.github/workflows/ci.yml           + apt (ALSA, ffmpeg) y test MP3 gapless
crates/core/src/events.rs          + PlaybackState y eventos del reproductor
crates/core/src/queue.rs           + Queue::peek_next
crates/audio/
  Cargo.toml
  src/lib.rs                       módulos
  src/error.rs                     AudioError
  src/decoder.rs                   symphonia: abrir, decodificar a f32 intercalado, seek exacto
  src/convert.rs                   mapa de canales + resampleo (rubato) con vaciado al final
  src/gain.rs                      ReplayGain (lectura de etiquetas, factor) y curva de volumen
  src/sink.rs                      trait AudioSink, SinkFormat, SinkError, CaptureSink (pruebas)
  src/engine.rs                    máquina de estados: cola, precarga, gapless, errores, dispositivo
  src/player.rs                    hilo de audio + API del servicio Player
  src/cpal_sink.rs                 salida real: cpal + ringbuf
  examples/play.rs                 prueba manual de escucha
  tests/common/mod.rs              generadores WAV/FLAC/senoidales + Rig de pruebas del motor
  tests/decoder.rs  tests/convert.rs  tests/gain.rs  tests/capture.rs
  tests/engine.rs  tests/player.rs
  tests/cpal_device.rs  tests/mp3_gapless.rs   (#[ignore]: dispositivo real / ffmpeg)
```

---

### Task 1: Núcleo — eventos del reproductor y siguiente pista sin avanzar

**Files:**
- Modify: `crates/core/src/events.rs` (sustituir el contenido completo)
- Modify: `crates/core/src/queue.rs` (añadir `peek_next` justo antes de `on_track_ended`)
- Test: `crates/core/tests/events.rs`, `crates/core/tests/queue_advanced.rs` (añadir al final)

**Interfaces:**
- Consumes: `TrackId`, `Queue::on_track_ended` (P1).
- Produces:
  - `goymusic_core::events::PlaybackState { Idle, Loading, Playing, Paused, Ended }` (`Copy, Default = Idle`, serde camelCase).
  - Nuevas variantes de `Event` (campos en camelCase en JSON):
    - `PlayerStateChanged { state: PlaybackState, track: Option<TrackId> }`
    - `PositionChanged { track: TrackId, position_ms: u64 }`
    - `PlaybackError { track: TrackId, reason: String }`
    - `AudioDeviceLost { reason: String }`
  - `Queue::peek_next(&self) -> Option<TrackId>`: lo que devolvería `on_track_ended`, sin mutar la cola.

- [ ] **Step 1: Escribir los tests que fallan**

Añadir al final de `crates/core/tests/events.rs`:
```rust
#[test]
fn player_events_use_camel_case_fields() {
    use goymusic_core::events::PlaybackState;
    use goymusic_core::TrackId;
    let json = serde_json::to_string(&Event::PositionChanged {
        track: TrackId(4),
        position_ms: 1500,
    })
    .unwrap();
    assert_eq!(json, r#"{"type":"positionChanged","track":4,"positionMs":1500}"#);
    let json = serde_json::to_string(&Event::PlayerStateChanged {
        state: PlaybackState::Playing,
        track: None,
    })
    .unwrap();
    assert_eq!(json, r#"{"type":"playerStateChanged","state":"playing","track":null}"#);
}
```

Añadir al final de `crates/core/tests/queue_advanced.rs`:
```rust
#[test]
fn peek_next_matches_on_track_ended_without_mutating() {
    use goymusic_core::queue::RepeatMode;
    let mut q = Queue::new();
    q.play_context(ids(2), 1, &mut rng());
    assert_eq!(q.peek_next(), None);
    q.set_repeat(RepeatMode::All);
    assert_eq!(q.peek_next(), Some(TrackId(1)));
    q.set_repeat(RepeatMode::One);
    assert_eq!(q.peek_next(), Some(TrackId(2)));
    q.queue_add(&[TrackId(9)]);
    q.set_repeat(RepeatMode::Off);
    let before = q.clone();
    assert_eq!(q.peek_next(), Some(TrackId(9)));
    assert_eq!(q, before);
    assert_eq!(q.on_track_ended(), Some(TrackId(9)));
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-core`
Expected: FAIL de compilación (`no variant named PositionChanged`, `no method named peek_next`).

- [ ] **Step 3: Implementar**

`crates/core/src/events.rs` completo:
```rust
//! Core events. Services publish them; adapters (Tauri now, WebSocket later) forward them
//! to the UI, so the UI never polls.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::TrackId;

/// Audio engine state: `Idle → Loading → Playing ⇄ Paused → Ended`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackState {
    #[default]
    Idle,
    Loading,
    Playing,
    Paused,
    Ended,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Event {
    LibraryUpdated,
    ScanProgress {
        done: u64,
        total: u64,
    },
    PlaylistsChanged,
    LikesChanged,
    QueueChanged,
    /// The state changed or a different track became audible.
    PlayerStateChanged {
        state: PlaybackState,
        track: Option<TrackId>,
    },
    /// Published ~4 times per second while playing.
    PositionChanged {
        track: TrackId,
        position_ms: u64,
    },
    /// `track` could not be played; the engine moved on to the next one.
    PlaybackError {
        track: TrackId,
        reason: String,
    },
    /// The output device failed; playback is paused while the engine retries the default device.
    AudioDeviceLost {
        reason: String,
    },
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

En `crates/core/src/queue.rs`, justo antes de `/// The current track finished playing on its own.`:
```rust
    /// What [`Queue::on_track_ended`] would return, without changing the queue. The audio engine
    /// uses it to preload the next track for gapless playback.
    pub fn peek_next(&self) -> Option<TrackId> {
        self.clone().on_track_ended()
    }
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS (2 tests nuevos en `goymusic-core`; la biblioteca no cambia).

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "Núcleo: eventos del reproductor y siguiente pista sin avanzar la cola"
```

---

### Task 2: Crate de audio y decodificador

**Prerrequisito:** `pkg-config --modversion alsa` debe responder (cabeceras de ALSA para cpal). Si falla: `sudo apt install libasound2-dev pkg-config` en una terminal propia, porque sudo necesita TTY.

**Files:**
- Modify: `Cargo.toml` (raíz), `.github/workflows/ci.yml`
- Create: `crates/audio/Cargo.toml`, `crates/audio/src/lib.rs`, `crates/audio/src/error.rs`, `crates/audio/src/decoder.rs`
- Test: `crates/audio/tests/common/mod.rs` (generadores), `crates/audio/tests/decoder.rs`

**Interfaces:**
- Consumes: nada de tareas anteriores.
- Produces:
  - `goymusic_audio::AudioError { Open { path, reason }, Unsupported(String), Decode(String), Resample(String) }` y `error::Result<T>`.
  - `goymusic_audio::decoder::Decoder`:
    - `open(&Path) -> Result<Decoder>`
    - `sample_rate() -> u32`, `channels() -> usize`, `total_frames() -> Option<u64>`
    - `next_chunk() -> Result<Option<Vec<f32>>>` (muestras intercaladas; `None` al final)
    - `seek(frame: u64) -> Result<()>` (exacto al frame; se limita al último frame)
  - Generadores de tests (`tests/common`): `sine(rate, channels, freq, start, frames) -> Vec<i16>` (dos rangos consecutivos empalman sin costura), `ramp(frames)` (el frame *i* vale *i*), `write_wav`, `write_flac`, `to_i16`.

- [ ] **Step 1: Crear el crate y escribir los tests que fallan**

`Cargo.toml` (raíz) completo:
```toml
[workspace]
resolver = "2"
members = ["crates/core", "crates/library", "crates/audio"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "GPL-3.0-or-later"

[workspace.dependencies]
cpal = "=0.15.3"
deunicode = "1"
flacenc = "=0.5.1"
hound = "3"
lofty = "0.21"
rand = "0.8"
rayon = "1"
ringbuf = "=0.4.8"
rubato = "=0.16.2"
rusqlite = { version = "0.32", features = ["bundled"] }
rusqlite_migration = "1.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
strsim = "0.11"
symphonia = { version = "=0.5.5", default-features = false, features = ["mp3", "flac", "ogg", "vorbis", "wav", "pcm", "aac", "isomp4", "alac"] }
tempfile = "3"
thiserror = "2"
walkdir = "2"
```

`crates/audio/Cargo.toml`:
```toml
[package]
name = "goymusic-audio"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
cpal.workspace = true
goymusic-core = { path = "../core" }
lofty.workspace = true
rand.workspace = true
ringbuf.workspace = true
rubato.workspace = true
symphonia.workspace = true
thiserror.workspace = true

[dev-dependencies]
flacenc.workspace = true
hound.workspace = true
tempfile.workspace = true
```

`crates/audio/src/lib.rs` (los módulos se van añadiendo en cada tarea):
```rust
//! GoyMusic audio engine: file → symphonia → channel mapping + rubato → ReplayGain → sink.

pub mod error;

pub use error::AudioError;
```

`crates/audio/src/error.rs`:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("cannot open {path}: {reason}")]
    Open { path: String, reason: String },
    #[error("unsupported audio: {0}")]
    Unsupported(String),
    #[error("decode failed: {0}")]
    Decode(String),
    #[error("resampler: {0}")]
    Resample(String),
}

pub type Result<T> = std::result::Result<T, AudioError>;
```

En `.github/workflows/ci.yml`, antes de `- run: cargo fmt --all -- --check`:
```yaml
      - run: sudo apt-get update && sudo apt-get install -y libasound2-dev pkg-config
```

`crates/audio/tests/common/mod.rs` (primera parte; la Task 6 añade el resto):
```rust
#![allow(dead_code)] // each test binary uses a different subset

use std::f64::consts::PI;
use std::path::Path;

/// `frames` frames of a sine wave, interleaved over `channels`, starting at frame `start` of
/// the same wave. Two calls with consecutive ranges join seamlessly, which is what the
/// gapless tests rely on. Amplitude 0.25 of 16-bit full scale.
pub fn sine(rate: u32, channels: u16, freq: f64, start: u64, frames: u64) -> Vec<i16> {
    let mut out = Vec::with_capacity((frames * u64::from(channels)) as usize);
    for i in start..start + frames {
        let v = (i as f64 * freq * 2.0 * PI / f64::from(rate)).sin() * 8192.0;
        for _ in 0..channels {
            out.push(v.round() as i16);
        }
    }
    out
}

/// Frame `i` holds the value `i` (mono), so a decoded sample says which frame it came from.
pub fn ramp(frames: u64) -> Vec<i16> {
    (0..frames).map(|i| i as i16).collect()
}

pub fn write_wav(path: &Path, rate: u32, channels: u16, samples: &[i16]) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for &s in samples {
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();
}

pub fn write_flac(path: &Path, rate: u32, channels: u16, samples: &[i16]) {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;
    let samples: Vec<i32> = samples.iter().map(|&s| i32::from(s)).collect();
    let config = flacenc::config::Encoder::default().into_verified().unwrap();
    let source = flacenc::source::MemSource::from_samples(&samples, channels as usize, 16, rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size).unwrap();
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).unwrap();
    let mut bytes = sink.as_slice().to_vec();
    // flacenc 0.5 counts the short last block in STREAMINFO's minimum block size; the FLAC
    // spec excludes it. With min != max, symphonia expects variable-size blocks and rejects
    // every frame. Bytes 8..10 are the minimum, 10..12 the maximum: copy max over min.
    let (min, max) = (8..10, 10..12);
    let max_block = bytes[max].to_vec();
    bytes[min].copy_from_slice(&max_block);
    std::fs::write(path, bytes).unwrap();
}

/// Decoded `f32` back to the 16-bit integer it was written as.
pub fn to_i16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|&s| (s * 32768.0).round() as i16).collect()
}
```

`crates/audio/tests/decoder.rs`:
```rust
mod common;

use common::{ramp, sine, to_i16, write_flac, write_wav};
use goymusic_audio::decoder::Decoder;
use goymusic_audio::AudioError;

fn decode_all(d: &mut Decoder) -> Vec<f32> {
    let mut out = Vec::new();
    while let Some(chunk) = d.next_chunk().unwrap() {
        out.extend(chunk);
    }
    out
}

#[test]
fn wav_decodes_to_the_exact_samples() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    let samples = sine(44_100, 2, 440.0, 0, 10_000);
    write_wav(&path, 44_100, 2, &samples);

    let mut d = Decoder::open(&path).unwrap();
    assert_eq!(
        (d.sample_rate(), d.channels(), d.total_frames()),
        (44_100, 2, Some(10_000))
    );
    assert_eq!(to_i16(&decode_all(&mut d)), samples);
}

#[test]
fn flac_decodes_to_the_exact_samples() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.flac");
    let samples = sine(48_000, 1, 1000.0, 0, 12_345);
    write_flac(&path, 48_000, 1, &samples);

    let mut d = Decoder::open(&path).unwrap();
    assert_eq!((d.sample_rate(), d.channels()), (48_000, 1));
    assert_eq!(to_i16(&decode_all(&mut d)), samples);
}

#[test]
fn garbage_and_missing_files_are_open_errors() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.mp3");
    std::fs::write(&bad, b"definitely not audio").unwrap();
    assert!(matches!(Decoder::open(&bad), Err(AudioError::Open { .. })));
    assert!(matches!(
        Decoder::open(&dir.path().join("missing.flac")),
        Err(AudioError::Open { .. })
    ));
}

#[test]
fn truncated_flac_returns_what_it_can_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cut.flac");
    write_flac(&path, 44_100, 1, &sine(44_100, 1, 440.0, 0, 44_100));
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();

    let mut d = Decoder::open(&path).unwrap();
    let mut frames = 0;
    while let Ok(Some(c)) = d.next_chunk() {
        frames += c.len();
    }
    assert!(frames > 0 && frames < 44_100);
}

#[test]
fn seek_lands_on_the_exact_frame() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["ramp.wav", "ramp.flac"] {
        let path = dir.path().join(name);
        let samples = ramp(30_000);
        if name.ends_with(".wav") {
            write_wav(&path, 8_000, 1, &samples);
        } else {
            write_flac(&path, 8_000, 1, &samples);
        }
        let mut d = Decoder::open(&path).unwrap();
        for target in [12_000u64, 4_321, 0, 29_999] {
            d.seek(target).unwrap();
            let first = d.next_chunk().unwrap().unwrap()[0];
            assert_eq!(to_i16(&[first])[0] as u64, target, "{name} seek to {target}");
        }
    }
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-audio --test decoder`
Expected: FAIL de compilación (`could not find decoder in goymusic_audio`).

- [ ] **Step 3: Implementar**

En `crates/audio/src/lib.rs` añadir `pub mod decoder;` encima de `pub mod error;`.

`crates/audio/src/decoder.rs`:
```rust
//! One open audio file: probes the container, decodes packets to interleaved `f32` and seeks
//! to an exact frame. Gapless trimming of encoder delay/padding (MP3, Vorbis) is done by
//! symphonia itself once `enable_gapless` is set, so it must not be repeated here.

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder as CodecDecoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::{AudioError, Result};

/// Consecutive undecodable packets tolerated before the file is declared broken.
const MAX_BAD_PACKETS: u32 = 50;

pub struct Decoder {
    format: Box<dyn FormatReader>,
    codec: Box<dyn CodecDecoder>,
    track_id: u32,
    sample_rate: u32,
    channels: usize,
    total_frames: Option<u64>,
    /// Frames to drop from the next decoded packets (accurate seek lands before the target).
    skip_frames: u64,
    buffer: Option<SampleBuffer<f32>>,
}

impl Decoder {
    pub fn open(path: &Path) -> Result<Self> {
        let open_err = |reason: String| AudioError::Open {
            path: path.display().to_string(),
            reason,
        };
        let file = File::open(path).map_err(|e| open_err(e.to_string()))?;
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                MediaSourceStream::new(Box::new(file), Default::default()),
                &format_opts,
                &MetadataOptions::default(),
            )
            .map_err(|e| open_err(e.to_string()))?;
        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| AudioError::Unsupported("no audio track".into()))?;
        let params = &track.codec_params;
        let sample_rate = params
            .sample_rate
            .ok_or_else(|| AudioError::Unsupported("unknown sample rate".into()))?;
        let channels = params
            .channels
            .map(|c| c.count())
            .ok_or_else(|| AudioError::Unsupported("unknown channel layout".into()))?;
        let codec = symphonia::default::get_codecs()
            .make(params, &DecoderOptions::default())
            .map_err(|e| AudioError::Unsupported(e.to_string()))?;
        Ok(Self {
            track_id: track.id,
            total_frames: params.n_frames,
            format,
            codec,
            sample_rate,
            channels,
            skip_frames: 0,
            buffer: None,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Length in frames when the container declares it (after gapless trimming).
    pub fn total_frames(&self) -> Option<u64> {
        self.total_frames
    }

    /// Next block of interleaved samples; `None` at the end of the stream.
    pub fn next_chunk(&mut self) -> Result<Option<Vec<f32>>> {
        let mut bad_packets = 0;
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                Err(SymphoniaError::ResetRequired) => return Ok(None),
                Err(e) => return Err(AudioError::Decode(e.to_string())),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let decoded = match self.codec.decode(&packet) {
                Ok(d) => d,
                Err(SymphoniaError::DecodeError(e)) => {
                    bad_packets += 1;
                    if bad_packets > MAX_BAD_PACKETS {
                        return Err(AudioError::Decode(e.to_string()));
                    }
                    continue;
                }
                Err(e) => return Err(AudioError::Decode(e.to_string())),
            };
            if decoded.frames() == 0 {
                continue;
            }
            let needed = decoded.capacity() as u64;
            let buffer = match &mut self.buffer {
                Some(b) if b.capacity() as u64 >= needed => b,
                slot => slot.insert(SampleBuffer::new(needed, *decoded.spec())),
            };
            buffer.copy_interleaved_ref(decoded);
            let mut samples = buffer.samples().to_vec();
            if self.skip_frames > 0 {
                let frames = (samples.len() / self.channels) as u64;
                let drop = self.skip_frames.min(frames);
                self.skip_frames -= drop;
                samples.drain(..drop as usize * self.channels);
            }
            if !samples.is_empty() {
                return Ok(Some(samples));
            }
        }
    }

    /// Positions the stream so the next sample returned is frame `frame` (clamped to the end).
    pub fn seek(&mut self, frame: u64) -> Result<()> {
        let frame = self
            .total_frames
            .map_or(frame, |total| frame.min(total.saturating_sub(1)));
        // Timestamps are in the track's time base (usually 1/sample_rate). Integer maths: a
        // float round trip loses a frame (29_999 / 8_000 s seeks to frame 29_998).
        let (numer, denom) = self.time_base();
        let ts = (u128::from(frame) * u128::from(denom) / (u128::from(numer) * u128::from(self.sample_rate))) as u64;
        let seeked = self
            .format
            .seek(
                SeekMode::Accurate,
                SeekTo::TimeStamp {
                    ts,
                    track_id: self.track_id,
                },
            )
            .map_err(|e| AudioError::Decode(format!("seek: {e}")))?;
        self.codec.reset();
        let late = seeked.required_ts.saturating_sub(seeked.actual_ts);
        self.skip_frames =
            (u128::from(late) * u128::from(numer) * u128::from(self.sample_rate) / u128::from(denom)) as u64;
        Ok(())
    }

    /// `(numer, denom)` of the track's time base; 1/sample_rate when the container omits it.
    fn time_base(&self) -> (u32, u32) {
        self.format
            .tracks()
            .iter()
            .find(|t| t.id == self.track_id)
            .and_then(|t| t.codec_params.time_base)
            .map_or((1, self.sample_rate), |tb| (tb.numer, tb.denom))
    }
}
```

Notas:
- **Por qué hay que corregir el FLAC de flacenc.** El generador corrige 2 bytes de STREAMINFO porque flacenc 0.5 cuenta el último bloque, el corto, en el tamaño mínimo de bloque. Con mínimo ≠ máximo, symphonia interpreta bloques de tamaño variable y rechaza todos los frames con "end of stream".
- **Por qué el seek usa aritmética entera.** Usa `SeekTo::TimeStamp` en la base de tiempo de la pista. Con `f64`, buscar 29 999/8 000 s cae en el frame 29 998.

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS (5 tests en `decoder`).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock .github/workflows/ci.yml crates/audio
git commit -m "Audio: crate goymusic-audio y decodificador con seek exacto"
```

---

### Task 3: Conversión de canales y resampleo

**Files:**
- Create: `crates/audio/src/convert.rs`
- Modify: `crates/audio/src/lib.rs` (añadir `pub mod convert;` el primero)
- Test: `crates/audio/tests/convert.rs`

**Interfaces:**
- Consumes: `AudioError::Resample` (Task 2).
- Produces: `goymusic_audio::convert::Converter`:
  - `new(in_rate: u32, in_channels: usize, out_rate: u32, out_channels: usize) -> Result<Converter>`
  - `push(&mut self, &[f32]) -> Result<Vec<f32>>`: puede devolver menos frames de los que recibe, porque guarda un bloque parcial.
  - `finish(&mut self) -> Result<Vec<f32>>`: vacía el resampler al final de la pista; la salida total mide exactamente `frames_in * out_rate / in_rate`, sin el retardo inicial.
  - Mapa de canales:
    - mismo número de canales: sin cambios;
    - a mono: promedio;
    - en otro caso, el canal *c* toma el canal de entrada *c*, o el último si hay menos (mono → ambos lados).

- [ ] **Step 1: Escribir los tests que fallan**

`crates/audio/tests/convert.rs`:
```rust
use goymusic_audio::convert::Converter;

fn sine_f32(rate: u32, freq: f64, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|i| ((i as f64 * freq * std::f64::consts::TAU / f64::from(rate)).sin() * 0.5) as f32)
        .collect()
}

/// Upward zero crossings: a frequency estimate that ignores amplitude.
fn crossings(mono: &[f32]) -> usize {
    mono.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count()
}

fn convert_all(c: &mut Converter, input: &[f32], block: usize) -> Vec<f32> {
    let mut out = Vec::new();
    for chunk in input.chunks(block) {
        out.extend(c.push(chunk).unwrap());
    }
    out.extend(c.finish().unwrap());
    out
}

#[test]
fn same_format_passes_through_untouched() {
    let input = sine_f32(44_100, 440.0, 5_000);
    let mut c = Converter::new(44_100, 1, 44_100, 1).unwrap();
    assert_eq!(convert_all(&mut c, &input, 777), input);
}

#[test]
fn mono_is_duplicated_and_stereo_is_averaged() {
    let mut up = Converter::new(48_000, 1, 48_000, 2).unwrap();
    assert_eq!(up.push(&[0.1, 0.2]).unwrap(), vec![0.1, 0.1, 0.2, 0.2]);
    let mut down = Converter::new(48_000, 2, 48_000, 1).unwrap();
    assert_eq!(down.push(&[0.2, 0.4, -1.0, 1.0]).unwrap(), vec![0.3_f32, 0.0]);
}

#[test]
fn resampling_keeps_duration_and_pitch() {
    for (from, to) in [(44_100u32, 48_000u32), (48_000, 44_100), (22_050, 48_000)] {
        let seconds = 2;
        let input = sine_f32(from, 1000.0, from as usize * seconds);
        let mut c = Converter::new(from, 1, to, 1).unwrap();
        let out = convert_all(&mut c, &input, 1_000);
        assert_eq!(out.len(), to as usize * seconds, "{from}->{to}: exact length");
        let measured = crossings(&out) as i64;
        assert!(
            (measured - 2_000).abs() <= 2,
            "{from}->{to}: 1 kHz for 2 s, got {measured} cycles"
        );
        // no delay at the start: the first output samples follow the input's sine, not silence
        let first_peak = out[..to as usize / 1000].iter().fold(0f32, |m, s| m.max(s.abs()));
        assert!(first_peak > 0.4, "{from}->{to}: resampler delay not trimmed");
    }
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-audio --test convert`
Expected: FAIL de compilación (`could not find convert`).

- [ ] **Step 3: Implementar**

`crates/audio/src/convert.rs`:
```rust
//! Brings decoded audio to the sink's format: channel mapping, then resampling with rubato
//! when the file's sample rate differs from the device's.

use rubato::{FftFixedInOut, Resampler};

use crate::error::{AudioError, Result};

const CHUNK_FRAMES: usize = 1024;

pub struct Converter {
    in_channels: usize,
    out_channels: usize,
    resampler: Option<Resampling>,
}

struct Resampling {
    inner: FftFixedInOut<f32>,
    /// Input not yet resampled, one Vec per output channel.
    pending: Vec<Vec<f32>>,
    /// Leading output frames still to drop: the resampler's own delay.
    delay_left: usize,
    in_rate: u64,
    out_rate: u64,
    frames_in: u64,
    frames_out: u64,
}

impl Converter {
    pub fn new(in_rate: u32, in_channels: usize, out_rate: u32, out_channels: usize) -> Result<Self> {
        let resampler = if in_rate == out_rate {
            None
        } else {
            let inner = FftFixedInOut::new(in_rate as usize, out_rate as usize, CHUNK_FRAMES, out_channels)
                .map_err(|e| AudioError::Resample(e.to_string()))?;
            Some(Resampling {
                delay_left: inner.output_delay(),
                inner,
                pending: vec![Vec::new(); out_channels],
                in_rate: u64::from(in_rate),
                out_rate: u64::from(out_rate),
                frames_in: 0,
                frames_out: 0,
            })
        };
        Ok(Self {
            in_channels,
            out_channels,
            resampler,
        })
    }

    /// Converts a block of interleaved input; may return fewer frames than given (the
    /// resampler holds a partial chunk until more input or [`Converter::finish`]).
    pub fn push(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        let mapped = map_channels(input, self.in_channels, self.out_channels);
        match &mut self.resampler {
            None => Ok(mapped),
            Some(r) => r.push(&mapped, self.out_channels),
        }
    }

    /// End of the track: flushes what the resampler still holds, so consecutive tracks join
    /// without losing or adding frames.
    pub fn finish(&mut self) -> Result<Vec<f32>> {
        match &mut self.resampler {
            None => Ok(Vec::new()),
            Some(r) => r.finish(self.out_channels),
        }
    }
}

/// Same channel count: unchanged. To mono: average. Otherwise output channel `c` takes input
/// channel `c`, or the last input channel when there are fewer (mono → both sides).
fn map_channels(input: &[f32], from: usize, to: usize) -> Vec<f32> {
    if from == to {
        return input.to_vec();
    }
    let mut out = Vec::with_capacity(input.len() / from * to);
    for frame in input.chunks_exact(from) {
        if to == 1 {
            out.push(frame.iter().sum::<f32>() / from as f32);
        } else {
            out.extend((0..to).map(|c| frame[c.min(from - 1)]));
        }
    }
    out
}

impl Resampling {
    fn push(&mut self, interleaved: &[f32], channels: usize) -> Result<Vec<f32>> {
        for frame in interleaved.chunks_exact(channels) {
            for (c, &s) in frame.iter().enumerate() {
                self.pending[c].push(s);
            }
        }
        self.frames_in += (interleaved.len() / channels) as u64;
        let mut out = Vec::new();
        while self.pending[0].len() >= self.inner.input_frames_next() {
            let n = self.inner.input_frames_next();
            let chunk: Vec<Vec<f32>> = self.pending.iter_mut().map(|p| p.drain(..n).collect()).collect();
            let resampled = self
                .inner
                .process(&chunk, None)
                .map_err(|e| AudioError::Resample(e.to_string()))?;
            self.emit(&resampled, &mut out, u64::MAX);
        }
        Ok(out)
    }

    fn finish(&mut self, channels: usize) -> Result<Vec<f32>> {
        let expected = self.frames_in * self.out_rate / self.in_rate;
        let mut out = Vec::new();
        let mut tail = Some(std::mem::replace(&mut self.pending, vec![Vec::new(); channels]));
        while self.frames_out < expected {
            let resampled = match tail.take() {
                Some(t) => self.inner.process_partial(Some(&t), None),
                None => self.inner.process_partial::<Vec<f32>>(None, None),
            }
            .map_err(|e| AudioError::Resample(e.to_string()))?;
            self.emit(&resampled, &mut out, expected);
        }
        Ok(out)
    }

    /// Appends resampled frames to `out`, skipping the leading delay and stopping at `limit`.
    fn emit(&mut self, resampled: &[Vec<f32>], out: &mut Vec<f32>, limit: u64) {
        let frames = resampled[0].len();
        let skip = self.delay_left.min(frames);
        self.delay_left -= skip;
        for i in skip..frames {
            if self.frames_out >= limit {
                return;
            }
            out.extend(resampled.iter().map(|ch| ch[i]));
            self.frames_out += 1;
        }
    }
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS (3 tests en `convert`).

- [ ] **Step 5: Commit**

```bash
git add crates/audio
git commit -m "Audio: mapa de canales y resampleo sin pérdida de duración"
```

---

### Task 4: ReplayGain y curva de volumen

**Files:**
- Create: `crates/audio/src/gain.rs`
- Modify: `crates/audio/src/lib.rs` (añadir `pub mod gain;` tras `pub mod error;`)
- Test: `crates/audio/tests/gain.rs`

**Interfaces:**
- Consumes: generadores de `tests/common` (Task 2).
- Produces (`goymusic_audio::gain`):
  - `ReplayGain { track_gain_db: Option<f32>, track_peak: Option<f32> }`:
    - `read(&Path) -> ReplayGain`: sin etiquetas devuelve `default()`;
    - `factor() -> f32`: 1.0 sin ganancia; limitado a `1 / peak`.
  - `parse_number(&str) -> Option<f32>`
  - `volume_to_amplitude(f32) -> f32` (cúbica, limitada a 0–1)
  - `apply(&mut [f32], factor)`: multiplica y recorta a ±1.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/audio/tests/gain.rs`:
```rust
mod common;

use common::{sine, write_wav};
use goymusic_audio::gain::{apply, parse_number, volume_to_amplitude, ReplayGain};
use lofty::config::WriteOptions;
use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag, TagType};

#[test]
fn parses_replaygain_tag_values() {
    assert_eq!(parse_number("-6.48 dB"), Some(-6.48));
    assert_eq!(parse_number(" +1.5 db "), Some(1.5));
    assert_eq!(parse_number("0.988547"), Some(0.988547));
    assert_eq!(parse_number("loud"), None);
    assert_eq!(parse_number("NaN dB"), None);
}

#[test]
fn factor_follows_gain_and_respects_peak() {
    let quieter = ReplayGain {
        track_gain_db: Some(-6.0),
        track_peak: None,
    };
    assert!((quieter.factor() - 0.501).abs() < 0.001);
    let louder = ReplayGain {
        track_gain_db: Some(6.0),
        track_peak: Some(0.8),
    };
    assert_eq!(louder.factor(), 1.25, "+6 dB would clip a 0.8 peak, capped at 1/0.8");
    assert_eq!(ReplayGain::default().factor(), 1.0);
}

#[test]
fn reads_replaygain_from_tags() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rg.wav");
    write_wav(&path, 8_000, 1, &sine(8_000, 1, 440.0, 0, 800));
    let mut tag = Tag::new(TagType::Id3v2);
    tag.insert_text(ItemKey::ReplayGainTrackGain, "-7.25 dB".into());
    tag.insert_text(ItemKey::ReplayGainTrackPeak, "0.95".into());
    tag.save_to_path(&path, WriteOptions::default()).unwrap();

    let rg = ReplayGain::read(&path);
    assert_eq!(
        rg,
        ReplayGain {
            track_gain_db: Some(-7.25),
            track_peak: Some(0.95)
        }
    );
    assert_eq!(ReplayGain::read(&dir.path().join("missing.wav")), ReplayGain::default());
}

#[test]
fn volume_curve_and_clamping() {
    assert_eq!(volume_to_amplitude(1.0), 1.0);
    assert_eq!(volume_to_amplitude(0.5), 0.125);
    assert_eq!(volume_to_amplitude(-3.0), 0.0);
    assert_eq!(volume_to_amplitude(7.0), 1.0);
    let mut samples = [0.5, -0.9];
    apply(&mut samples, 2.0);
    assert_eq!(samples, [1.0, -1.0]);
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-audio --test gain`
Expected: FAIL de compilación (`could not find gain`).

- [ ] **Step 3: Implementar**

`crates/audio/src/gain.rs`:
```rust
//! Software gain: per-track ReplayGain (applied while decoding) and the volume curve
//! (applied by the sink, so volume changes are heard at once, not after the buffer drains).

use std::path::Path;

use lofty::prelude::*;
use lofty::tag::ItemKey;

/// ReplayGain values from a file's tags.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReplayGain {
    pub track_gain_db: Option<f32>,
    pub track_peak: Option<f32>,
}

impl ReplayGain {
    /// Reads the track gain and peak; missing or unreadable tags give `ReplayGain::default()`.
    pub fn read(path: &Path) -> Self {
        let Ok(file) = lofty::read_from_path(path) else {
            return Self::default();
        };
        let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) else {
            return Self::default();
        };
        Self {
            track_gain_db: tag.get_string(&ItemKey::ReplayGainTrackGain).and_then(parse_number),
            track_peak: tag.get_string(&ItemKey::ReplayGainTrackPeak).and_then(parse_number),
        }
    }

    /// Linear factor for the track; never pushes the declared peak above full scale.
    /// No gain tag means 1.0.
    pub fn factor(&self) -> f32 {
        let Some(db) = self.track_gain_db else {
            return 1.0;
        };
        let factor = 10f32.powf(db / 20.0);
        match self.track_peak {
            Some(peak) if peak > 0.0 => factor.min(1.0 / peak),
            _ => factor,
        }
    }
}

/// `"-6.48 dB"`, `"+1.2 dB"`, `"0.988"` → the number. Tag values vary in case and spacing.
pub fn parse_number(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    let number = trimmed
        .strip_suffix("dB")
        .or_else(|| trimmed.strip_suffix("db"))
        .or_else(|| trimmed.strip_suffix("DB"))
        .unwrap_or(trimmed)
        .trim();
    number.parse::<f32>().ok().filter(|v| v.is_finite())
}

/// Volume 0.0–1.0 to amplitude. Cubic, so the slider feels even to the ear rather than
/// doing all its work in the top fifth.
pub fn volume_to_amplitude(volume: f32) -> f32 {
    volume.clamp(0.0, 1.0).powi(3)
}

/// Multiplies in place and clamps to the valid sample range.
pub fn apply(samples: &mut [f32], factor: f32) {
    if factor == 1.0 {
        return;
    }
    for s in samples {
        *s = (*s * factor).clamp(-1.0, 1.0);
    }
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS (4 tests en `gain`).

- [ ] **Step 5: Commit**

```bash
git add crates/audio
git commit -m "Audio: ReplayGain por pista y curva de volumen"
```

---

### Task 5: Abstracción de salida y salida de pruebas

**Files:**
- Create: `crates/audio/src/sink.rs`
- Modify: `crates/audio/src/lib.rs` (añadir `pub mod sink;` al final de los `pub mod`)
- Test: `crates/audio/tests/capture.rs`

**Interfaces:**
- Consumes: nada.
- Produces (`goymusic_audio::sink`):
  - `SinkFormat { sample_rate: u32, channels: u16 }` (`Copy, Eq`).
  - `SinkError { NoDevice, Device(String) }`.
  - `trait AudioSink`, sin `Send` (el stream de cpal no puede cambiar de hilo):
    - `format()`
    - `write(&[f32]) -> usize`: no bloquea; acepta frames completos.
    - `queued_frames()`, `played_frames()` (monótono)
    - `set_paused(bool)`, `set_volume(amplitude)`
    - `clear()`: al volver, la cola está vacía.
    - `take_error() -> Option<SinkError>`
  - `type SinkFactory = Box<dyn FnMut() -> Result<Box<dyn AudioSink>, SinkError> + Send>`.
  - `CaptureSink::new(SinkFormat) -> (CaptureSink, CaptureHandle)`.
    - El sink "reproduce" al instante, salvo en pausa o en modo `hold`.
    - `CaptureHandle`: `played()`, `clears()`, `fail(SinkError)`, `hold(bool)`, `play(frames, channels)`.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/audio/tests/capture.rs`:
```rust
use goymusic_audio::sink::{AudioSink, CaptureSink, SinkError, SinkFormat};

const STEREO: SinkFormat = SinkFormat {
    sample_rate: 48_000,
    channels: 2,
};

#[test]
fn plays_instantly_and_applies_volume() {
    let (mut sink, handle) = CaptureSink::new(STEREO);
    assert_eq!(sink.write(&[0.5, 0.5]), 2);
    sink.set_volume(0.5);
    sink.write(&[0.5, 0.5]);
    assert_eq!(handle.played(), vec![0.5, 0.5, 0.25, 0.25]);
    assert_eq!(sink.played_frames(), 2);
}

#[test]
fn pause_holds_samples_and_clear_drops_them() {
    let (mut sink, handle) = CaptureSink::new(STEREO);
    sink.set_paused(true);
    sink.write(&[0.1, 0.1, 0.2, 0.2]);
    assert_eq!((sink.queued_frames(), sink.played_frames()), (2, 0));
    sink.clear();
    sink.write(&[0.3, 0.3]);
    sink.set_paused(false);
    assert_eq!(handle.played(), vec![0.3, 0.3]);
    assert_eq!(handle.clears(), 1);
}

#[test]
fn injected_failure_is_reported_once() {
    let (mut sink, handle) = CaptureSink::new(STEREO);
    handle.fail(SinkError::Device("unplugged".into()));
    assert_eq!(sink.take_error(), Some(SinkError::Device("unplugged".into())));
    assert_eq!(sink.take_error(), None);
}

#[test]
fn hold_mode_plays_only_on_demand() {
    let (mut sink, handle) = CaptureSink::new(STEREO);
    handle.hold(true);
    sink.write(&[0.1, 0.1, 0.2, 0.2, 0.3, 0.3]);
    assert_eq!((sink.queued_frames(), sink.played_frames()), (3, 0));
    handle.play(2, 2);
    assert_eq!(handle.played(), vec![0.1, 0.1, 0.2, 0.2]);
    assert_eq!(sink.queued_frames(), 1);
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-audio --test capture`
Expected: FAIL de compilación (`could not find sink`).

- [ ] **Step 3: Implementar**

`crates/audio/src/sink.rs`:
```rust
//! Audio output abstraction. The engine writes interleaved `f32` in the sink's format; the
//! sink plays it. `CpalSink` is the real device, `CaptureSink` records samples for tests.

use std::sync::{Arc, Mutex};

use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SinkFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SinkError {
    #[error("no audio output device")]
    NoDevice,
    #[error("audio device error: {0}")]
    Device(String),
}

pub trait AudioSink {
    fn format(&self) -> SinkFormat;
    /// Queues interleaved samples without blocking; returns how many were accepted, always a
    /// whole number of frames. Fewer than offered means the buffer is full.
    fn write(&mut self, samples: &[f32]) -> usize;
    /// Frames queued but not played yet.
    fn queued_frames(&self) -> u64;
    /// Frames played since the sink was created. Monotonic; `clear` does not reset it.
    fn played_frames(&self) -> u64;
    fn set_paused(&mut self, paused: bool);
    /// Amplitude applied at output time (see `gain::volume_to_amplitude`).
    fn set_volume(&mut self, amplitude: f32);
    /// Drops everything queued. On return `queued_frames() == 0`.
    fn clear(&mut self);
    /// A device failure since the last call, if any.
    fn take_error(&mut self) -> Option<SinkError>;
}

/// Opens a sink. The engine calls it at start and again after a device failure. It runs on
/// the engine thread because a cpal stream cannot move between threads.
pub type SinkFactory = Box<dyn FnMut() -> Result<Box<dyn AudioSink>, SinkError> + Send>;

/// Test sink: "plays" instantly into memory and records what was heard. Paused, or in hold
/// mode, samples wait in a queue instead; `CaptureHandle::play` then plays them on demand,
/// which lets tests reproduce a device that is behind the decoder.
pub struct CaptureSink {
    format: SinkFormat,
    state: Arc<Mutex<CaptureState>>,
}

#[derive(Default)]
struct CaptureState {
    played: Vec<f32>,
    queued: Vec<f32>,
    paused: bool,
    hold: bool,
    amplitude: f32,
    error: Option<SinkError>,
    clears: usize,
}

/// The test's view of a [`CaptureSink`] that the engine owns.
#[derive(Clone)]
pub struct CaptureHandle {
    state: Arc<Mutex<CaptureState>>,
}

impl CaptureSink {
    pub fn new(format: SinkFormat) -> (Self, CaptureHandle) {
        let state = Arc::new(Mutex::new(CaptureState {
            amplitude: 1.0,
            ..Default::default()
        }));
        (
            Self {
                format,
                state: state.clone(),
            },
            CaptureHandle { state },
        )
    }
}

impl AudioSink for CaptureSink {
    fn format(&self) -> SinkFormat {
        self.format
    }

    fn write(&mut self, samples: &[f32]) -> usize {
        let mut s = self.state.lock().unwrap();
        let amplitude = s.amplitude;
        if s.paused || s.hold || !s.queued.is_empty() {
            s.queued.extend_from_slice(samples);
        } else {
            s.played.extend(samples.iter().map(|x| x * amplitude));
        }
        samples.len()
    }

    fn queued_frames(&self) -> u64 {
        (self.state.lock().unwrap().queued.len() / self.format.channels as usize) as u64
    }

    fn played_frames(&self) -> u64 {
        (self.state.lock().unwrap().played.len() / self.format.channels as usize) as u64
    }

    fn set_paused(&mut self, paused: bool) {
        let mut s = self.state.lock().unwrap();
        s.paused = paused;
        if !paused && !s.hold {
            s.play_queued(usize::MAX);
        }
    }

    fn set_volume(&mut self, amplitude: f32) {
        self.state.lock().unwrap().amplitude = amplitude;
    }

    fn clear(&mut self) {
        let mut s = self.state.lock().unwrap();
        s.queued.clear();
        s.clears += 1;
    }

    fn take_error(&mut self) -> Option<SinkError> {
        self.state.lock().unwrap().error.take()
    }
}

impl CaptureState {
    fn play_queued(&mut self, samples: usize) {
        let n = samples.min(self.queued.len());
        let amplitude = self.amplitude;
        let moved: Vec<f32> = self.queued.drain(..n).map(|x| x * amplitude).collect();
        self.played.extend(moved);
    }
}

impl CaptureHandle {
    /// Hold mode: written samples stay queued until [`CaptureHandle::play`].
    pub fn hold(&self, on: bool) {
        self.state.lock().unwrap().hold = on;
    }

    /// Plays up to `frames` queued frames (ignored while paused).
    pub fn play(&self, frames: usize, channels: usize) {
        let mut s = self.state.lock().unwrap();
        if !s.paused {
            s.play_queued(frames * channels);
        }
    }

    /// Every sample played so far, interleaved.
    pub fn played(&self) -> Vec<f32> {
        self.state.lock().unwrap().played.clone()
    }

    pub fn clears(&self) -> usize {
        self.state.lock().unwrap().clears
    }

    /// Simulates a device failure, reported at the next `take_error`.
    pub fn fail(&self, error: SinkError) {
        self.state.lock().unwrap().error = Some(error);
    }
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS (4 tests en `capture`).

- [ ] **Step 5: Commit**

```bash
git add crates/audio
git commit -m "Audio: trait AudioSink y salida de captura para pruebas"
```

---

### Task 6: Motor de reproducción — cola, gapless, precarga, errores y dispositivo

**Files:**
- Create: `crates/audio/src/engine.rs`
- Modify: `crates/audio/src/lib.rs` (añadir `pub mod engine;` tras `pub mod decoder;`)
- Modify: `crates/audio/tests/common/mod.rs` (añadir al final el `Rig`)
- Test: `crates/audio/tests/engine.rs`

**Interfaces:**
- Consumes: `Decoder` (T2), `Converter` (T3), `gain::*` (T4), `AudioSink`/`SinkFactory`/`SinkFormat` (T5), `Queue::peek_next` y eventos (T1).
- Produces (`goymusic_audio::engine`):
  - `type Resolver = Box<dyn Fn(TrackId) -> Option<PathBuf> + Send>`.
  - `const PRELOAD_MS: u64 = 10_000`.
  - `PlayerSnapshot { state, track, position_ms, volume, replaygain, queue }`.
  - `Engine::new(SinkFactory, Resolver, Arc<EventBus>)`.
  - Consultas:
    - `state()`, `current()` (la pista que se oye), `queue()`, `position_ms()`
    - `preloaded()`, `buffered_tracks()`: diagnóstico y tests
    - `needs_sink()`, `snapshot()`
  - Órdenes:
    - `play_context(Vec<TrackId>, usize)`, `pause()`, `resume()`, `next()`, `previous()`, `seek(ms)`
    - `set_shuffle(bool)`, `set_repeat(RepeatMode)`
    - `queue_add(&[TrackId])`, `queue_remove(usize)`, `queue_move(usize, usize)`
    - `set_volume(f32)`, `set_replaygain(bool)`, `restore(Queue, ms)`
  - Ciclo:
    - `pump()`: trabajo acotado; llamarlo en bucle.
    - `publish_position()`
    - `try_reopen_sink() -> bool`
  - Rig de pruebas (`tests/common`): `Rig::new(SinkFormat, &[(i64, PathBuf)])` con `engine`, `events`, `sinks`, `factory_fails`, `format`. Métodos: `sink(i)`, `run(max)`, `advance(sink, frames)`, `drain_events()`. Funciones auxiliares: `capture_factory`, `resolver`, `heard(&[Event])`, `longest_quiet_run`.

Reglas que implementa (spec §5 y §9):
- **Gapless.** Con ≤ `PRELOAD_MS` por decodificar se abre `queue.peek_next()`. Al llegar al final del archivo, la siguiente se escribe detrás en el mismo búfer (`Segment`). Cuando el contador de frames reproducidos alcanza su inicio, la cola avanza con `on_track_ended()` y se publica `PlayerStateChanged`.
- **Invariante.** `queue.current()` es siempre lo que se oye.
- **Edición de la cola.**
  - Si la siguiente precargada deja de ser la siguiente, se descarta.
  - Si ya estaba en el búfer, se reinicia la pista actual en su posición (Review Focus 3).
- **Error al abrir.** `PlaybackError` y `queue.next()`, como máximo una vez por pista de la cola, más una (Review Focus 1 y 2).
- **Error a mitad de la decodificación.** `PlaybackError` y se pasa a la siguiente sin hueco.
- **Dispositivo perdido.**
  - Se guarda `(pista, posición)`, se pausa y se emite `AudioDeviceLost`.
  - `try_reopen_sink` reabre y deja la pista en pausa en esa posición (Review Focus 4).
- **Sin dispositivo al empezar.** Igual que un dispositivo perdido.
- **Fin de la cola.** `PlayerStateChanged { Ended, Some(última) }`.

- [ ] **Step 1: Escribir los tests que fallan**

Añadir al final de `crates/audio/tests/common/mod.rs`:
```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use goymusic_audio::engine::Engine;
use goymusic_audio::sink::{AudioSink, CaptureHandle, CaptureSink, SinkError, SinkFactory, SinkFormat};
use goymusic_core::events::{Event, EventBus, PlaybackState};
use goymusic_core::TrackId;

/// An engine over capture sinks. Every sink the factory opens is kept in `sinks`.
pub struct Rig {
    pub engine: Engine,
    pub events: Receiver<Event>,
    pub sinks: Arc<Mutex<Vec<CaptureHandle>>>,
    pub factory_fails: Arc<AtomicBool>,
    pub format: SinkFormat,
}

pub fn capture_factory(format: SinkFormat) -> (SinkFactory, Arc<Mutex<Vec<CaptureHandle>>>, Arc<AtomicBool>) {
    let sinks = Arc::new(Mutex::new(Vec::new()));
    let fails = Arc::new(AtomicBool::new(false));
    let (s, f) = (sinks.clone(), fails.clone());
    let factory: SinkFactory = Box::new(move || {
        if f.load(Ordering::SeqCst) {
            return Err(SinkError::NoDevice);
        }
        let (sink, handle) = CaptureSink::new(format);
        s.lock().unwrap().push(handle);
        Ok(Box::new(sink) as Box<dyn AudioSink>)
    });
    (factory, sinks, fails)
}

pub fn resolver(files: &[(i64, PathBuf)]) -> goymusic_audio::engine::Resolver {
    let map: HashMap<TrackId, PathBuf> = files.iter().map(|(id, p)| (TrackId(*id), p.clone())).collect();
    Box::new(move |id| map.get(&id).cloned())
}

impl Rig {
    pub fn new(format: SinkFormat, files: &[(i64, PathBuf)]) -> Self {
        let bus = Arc::new(EventBus::new());
        let events = bus.subscribe();
        let (factory, sinks, factory_fails) = capture_factory(format);
        let engine = Engine::new(factory, resolver(files), bus);
        Self {
            engine,
            events,
            sinks,
            factory_fails,
            format,
        }
    }

    pub fn sink(&self, i: usize) -> CaptureHandle {
        self.sinks.lock().unwrap()[i].clone()
    }

    /// Pumps until playback stops (ended, idle, paused) or `max` pumps.
    pub fn run(&mut self, max: usize) {
        for _ in 0..max {
            self.engine.pump();
            if self.engine.state() != PlaybackState::Playing {
                return;
            }
        }
    }

    /// Lets a held sink play `frames` frames, pumping in between like the real thread does.
    pub fn advance(&mut self, sink: usize, frames: usize) {
        let handle = self.sink(sink);
        let channels = self.format.channels as usize;
        let mut left = frames;
        while left > 0 {
            self.engine.pump();
            let step = left.min(400);
            handle.play(step, channels);
            left -= step;
        }
    }

    pub fn drain_events(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }
}

/// Tracks announced as audible, in order, without consecutive repeats.
pub fn heard(events: &[Event]) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    for e in events {
        if let Event::PlayerStateChanged {
            state: PlaybackState::Playing,
            track: Some(t),
        } = e
        {
            if out.last() != Some(&t.0) {
                out.push(t.0);
            }
        }
    }
    out
}

/// Longest run of samples (per channel 0) below `threshold` in absolute value.
pub fn longest_quiet_run(interleaved: &[f32], channels: usize, threshold: f32) -> usize {
    let (mut best, mut run) = (0, 0);
    for frame in interleaved.chunks_exact(channels) {
        if frame[0].abs() < threshold {
            run += 1;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best
}
```

`crates/audio/tests/engine.rs`:
```rust
mod common;

use std::path::PathBuf;

use common::{heard, longest_quiet_run, ramp, sine, to_i16, write_flac, write_wav, Rig};
use goymusic_audio::sink::{SinkError, SinkFormat};
use goymusic_core::events::{Event, PlaybackState};
use goymusic_core::queue::RepeatMode;
use goymusic_core::TrackId;

const CD: SinkFormat = SinkFormat {
    sample_rate: 44_100,
    channels: 2,
};
const LOW: SinkFormat = SinkFormat {
    sample_rate: 8_000,
    channels: 1,
};

/// Writes `frames` frames of one continuous 8 kHz mono ramp-free sine per track.
fn short_tracks(dir: &std::path::Path, seconds: &[u64]) -> Vec<(i64, PathBuf)> {
    let mut start = 0;
    seconds
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let path = dir.join(format!("{i}.wav"));
            let frames = s * 8_000;
            write_wav(&path, 8_000, 1, &sine(8_000, 1, 440.0, start, frames));
            start += frames;
            (i as i64 + 1, path)
        })
        .collect()
}

#[test]
fn gapless_flac_and_wav_join_sample_exact() {
    let dir = tempfile::tempdir().unwrap();
    let parts = [
        (0u64, 30_000u64, "a.flac"),
        (30_000, 20_000, "b.wav"),
        (50_000, 11_111, "c.flac"),
    ];
    let mut files = Vec::new();
    for (i, (start, frames, name)) in parts.iter().enumerate() {
        let path = dir.path().join(name);
        let samples = sine(44_100, 2, 440.0, *start, *frames);
        if name.ends_with(".flac") {
            write_flac(&path, 44_100, 2, &samples);
        } else {
            write_wav(&path, 44_100, 2, &samples);
        }
        files.push((i as i64 + 1, path));
    }
    let mut rig = Rig::new(CD, &files);

    rig.engine.play_context(vec![TrackId(1), TrackId(2), TrackId(3)], 0);
    rig.run(10_000);

    assert_eq!(
        to_i16(&rig.sink(0).played()),
        sine(44_100, 2, 440.0, 0, 61_111),
        "not one continuous wave"
    );
    let events = rig.drain_events();
    assert_eq!(heard(&events), vec![1, 2, 3]);
    assert_eq!(rig.engine.state(), PlaybackState::Ended);
    assert!(events.contains(&Event::PlayerStateChanged {
        state: PlaybackState::Ended,
        track: Some(TrackId(3))
    }));
}

#[test]
fn gapless_through_the_resampler_has_no_silence() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = Vec::new();
    for (i, (start, frames)) in [(0u64, 30_000u64), (30_000, 20_000)].iter().enumerate() {
        let path = dir.path().join(format!("{i}.flac"));
        write_flac(&path, 44_100, 1, &sine(44_100, 1, 440.0, *start, *frames));
        files.push((i as i64 + 1, path));
    }
    let format = SinkFormat {
        sample_rate: 48_000,
        channels: 2,
    };
    let mut rig = Rig::new(format, &files);

    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    rig.run(10_000);

    let played = rig.sink(0).played();
    let expected_frames = 30_000 * 48_000 / 44_100 + 20_000 * 48_000 / 44_100;
    assert!(
        (played.len() as i64 / 2 - expected_frames as i64).abs() <= 2,
        "frames lost or added"
    );
    // a 440 Hz sine at 0.25 is under 0.01 for at most 2 samples per zero crossing
    assert!(
        longest_quiet_run(&played[2_000..], 2, 0.01) <= 3,
        "silence at the track boundary"
    );
}

#[test]
fn next_track_is_preloaded_ten_seconds_before_the_end() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[12, 1]);
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);

    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    let mut preloaded_at = None;
    for _ in 0..1_000 {
        rig.engine.pump();
        if rig.engine.preloaded().is_some() {
            preloaded_at = Some(rig.engine.position_ms());
            break;
        }
        sink.play(400, 1); // 50 ms of "device time" per pump
    }
    // the decoder runs ahead of the device by the 500 ms buffer plus one packet
    let at = preloaded_at.expect("never preloaded");
    assert!((1_300..=2_000).contains(&at), "preloaded at {at} ms");
    assert_eq!(rig.engine.preloaded(), Some(TrackId(2)));
}

#[test]
fn editing_the_queue_replaces_the_preloaded_track() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[3, 1, 1]);
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    rig.engine.pump();
    assert_eq!(rig.engine.preloaded(), Some(TrackId(2)), "3 s track: preloaded at once");

    rig.engine.queue_add(&[TrackId(3)]);
    assert_eq!(rig.engine.preloaded(), None);
    rig.engine.pump();
    assert_eq!(rig.engine.preloaded(), Some(TrackId(3)));

    sink.hold(false);
    sink.play(usize::MAX / 2, 1);
    rig.run(10_000);
    assert_eq!(heard(&rig.drain_events()), vec![1, 3, 2]);
}

#[test]
fn queue_edit_while_the_successor_is_buffered_restarts_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[1, 1, 1]);
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    for _ in 0..100 {
        rig.engine.pump();
        if rig.engine.buffered_tracks().len() == 2 {
            break;
        }
        sink.play(400, 1);
    }
    assert_eq!(rig.engine.buffered_tracks(), vec![TrackId(1), TrackId(2)]);
    let clears = sink.clears();

    rig.engine.queue_add(&[TrackId(3)]);

    assert_eq!(rig.engine.buffered_tracks(), vec![TrackId(1)]);
    assert_eq!(sink.clears(), clears + 1);
    sink.hold(false);
    sink.play(usize::MAX / 2, 1);
    rig.run(10_000);
    assert_eq!(heard(&rig.drain_events()), vec![1, 3, 2]);
    let total = sink.played().len() as i64;
    assert!((total - 24_000).abs() <= 8, "1 s lost or repeated: {total} frames");
}

#[test]
fn unreadable_and_unknown_tracks_are_reported_and_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = short_tracks(dir.path(), &[1, 1]);
    let bad = dir.path().join("bad.flac");
    std::fs::write(&bad, b"not audio").unwrap();
    files.push((3, bad));
    let mut rig = Rig::new(LOW, &files);

    // 4 is not in the library at all
    rig.engine
        .play_context(vec![TrackId(4), TrackId(1), TrackId(3), TrackId(2)], 0);
    rig.run(10_000);

    let events = rig.drain_events();
    assert_eq!(heard(&events), vec![1, 2]);
    let errors: Vec<i64> = events
        .iter()
        .filter_map(|e| match e {
            Event::PlaybackError { track, .. } => Some(track.0),
            _ => None,
        })
        .collect();
    assert_eq!(errors, vec![4, 3]);
    assert_eq!(rig.engine.state(), PlaybackState::Ended);
}

#[test]
fn a_queue_of_broken_files_on_repeat_ends_instead_of_looping() {
    let dir = tempfile::tempdir().unwrap();
    let files: Vec<(i64, PathBuf)> = (1..=2)
        .map(|i| {
            let p = dir.path().join(format!("{i}.mp3"));
            std::fs::write(&p, b"garbage").unwrap();
            (i, p)
        })
        .collect();
    let mut rig = Rig::new(LOW, &files);
    rig.engine.set_repeat(RepeatMode::All);

    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);

    assert_eq!(rig.engine.state(), PlaybackState::Ended);
    let errors = rig
        .drain_events()
        .iter()
        .filter(|e| matches!(e, Event::PlaybackError { .. }))
        .count();
    assert_eq!(errors, 3, "each queued track tried once, plus one");
}

#[test]
fn repeat_one_loops_the_track_gaplessly() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[1]);
    let mut rig = Rig::new(LOW, &files);
    rig.engine.set_repeat(RepeatMode::One);

    rig.engine.play_context(vec![TrackId(1)], 0);
    for _ in 0..40 {
        rig.engine.pump();
    }

    let once = sine(8_000, 1, 440.0, 0, 8_000);
    let played = to_i16(&rig.sink(0).played());
    assert!(played.len() >= 16_000);
    assert_eq!(&played[..8_000], &once[..]);
    assert_eq!(&played[8_000..16_000], &once[..]);
}

#[test]
fn seek_lands_on_the_exact_sample() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ramp.flac");
    write_flac(&path, 8_000, 1, &ramp(30_000));
    let mut rig = Rig::new(LOW, &[(1, path)]);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.engine.pump();

    rig.engine.seek(1_500);
    assert_eq!(rig.engine.position_ms(), 1_500);
    rig.engine.pump();
    sink.play(1, 1);

    let played = sink.played();
    assert_eq!(to_i16(&played[played.len() - 1..]), vec![12_000]);
}

#[test]
fn pause_resume_and_position() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[5]);
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.advance(0, 8_000);
    assert_eq!(rig.engine.position_ms(), 1_000);

    rig.engine.pause();
    sink.play(8_000, 1); // a paused device plays nothing
    assert_eq!(
        (rig.engine.state(), rig.engine.position_ms()),
        (PlaybackState::Paused, 1_000)
    );
    rig.engine.resume();
    assert_eq!(rig.engine.state(), PlaybackState::Playing);
    rig.advance(0, 4_000);
    assert_eq!(rig.engine.position_ms(), 1_500);
}

#[test]
fn next_previous_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[5, 5, 5]);
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1), TrackId(2), TrackId(3)], 0);

    rig.engine.next();
    assert_eq!(rig.engine.current(), Some(TrackId(2)));
    rig.engine.previous(); // at 0 ms: back to the previous track
    assert_eq!(rig.engine.current(), Some(TrackId(1)));

    rig.advance(0, 32_000);
    assert!(rig.engine.position_ms() > 3_000);
    rig.engine.previous(); // after 3 s: restart
    assert_eq!((rig.engine.current(), rig.engine.position_ms()), (Some(TrackId(1)), 0));
}

#[test]
fn volume_is_immediate_and_replaygain_applies_to_new_tracks() {
    use lofty::config::WriteOptions;
    use lofty::prelude::*;
    use lofty::tag::{ItemKey, Tag, TagType};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("loud.wav");
    write_wav(&path, 8_000, 1, &vec![16_384; 800]); // constant 0.5
    let mut tag = Tag::new(TagType::Id3v2);
    tag.insert_text(ItemKey::ReplayGainTrackGain, "-6.02 dB".into());
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    let mut rig = Rig::new(LOW, &[(1, path)]);

    rig.engine.set_volume(0.5);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.run(1_000);
    assert!((rig.sink(0).played()[0] - 0.5 * 0.125).abs() < 1e-4);

    rig.engine.set_volume(1.0);
    rig.engine.set_replaygain(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.run(1_000);
    assert!((rig.sink(0).played().last().unwrap() - 0.25).abs() < 1e-3);
}

#[test]
fn lost_device_pauses_and_resumes_on_the_default_device() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ramp.wav");
    write_wav(&path, 8_000, 1, &ramp(30_000));
    let mut rig = Rig::new(LOW, &[(1, path)]);
    let first = rig.sink(0);
    first.hold(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.advance(0, 8_000);

    first.fail(SinkError::Device("unplugged".into()));
    rig.factory_fails.store(true, std::sync::atomic::Ordering::SeqCst);
    rig.engine.pump();

    assert_eq!(rig.engine.state(), PlaybackState::Paused);
    assert!(rig.engine.needs_sink());
    assert_eq!(rig.engine.position_ms(), 1_000);
    assert!(rig.drain_events().contains(&Event::AudioDeviceLost {
        reason: "audio device error: unplugged".into()
    }));
    assert!(!rig.engine.try_reopen_sink(), "still no device");

    rig.factory_fails.store(false, std::sync::atomic::Ordering::SeqCst);
    assert!(rig.engine.try_reopen_sink());
    assert_eq!(
        (rig.engine.state(), rig.engine.position_ms()),
        (PlaybackState::Paused, 1_000)
    );
    rig.engine.resume();
    rig.engine.pump();
    assert_eq!(
        to_i16(&rig.sink(1).played()[..1]),
        vec![8_000],
        "resumes at the same sample"
    );
}

#[test]
fn restore_starts_paused_at_the_saved_position() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ramp.wav");
    write_wav(&path, 8_000, 1, &ramp(30_000));
    let mut rig = Rig::new(LOW, &[(1, path)]);
    let mut queue = goymusic_core::queue::Queue::new();
    queue.play_context(vec![TrackId(1)], 0, &mut rand::thread_rng());

    rig.engine.restore(queue, 1_500);

    let snapshot = rig.engine.snapshot();
    assert_eq!(
        (snapshot.state, snapshot.track, snapshot.position_ms),
        (PlaybackState::Paused, Some(TrackId(1)), 1_500)
    );
    rig.engine.pump();
    assert!(rig.sink(0).played().is_empty(), "paused: nothing plays");
    rig.engine.resume();
    rig.engine.pump();
    assert_eq!(to_i16(&rig.sink(0).played()[..1]), vec![12_000]);
}

#[test]
fn position_events_only_while_playing() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[5]);
    let mut rig = Rig::new(LOW, &files);
    rig.sink(0).hold(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.engine.pump();
    rig.sink(0).play(4_000, 1);
    rig.drain_events();

    rig.engine.publish_position();
    rig.engine.pause();
    rig.engine.publish_position();

    let positions: Vec<Event> = rig
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, Event::PositionChanged { .. }))
        .collect();
    assert_eq!(
        positions,
        vec![Event::PositionChanged {
            track: TrackId(1),
            position_ms: 500
        }]
    );
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-audio --test engine`
Expected: FAIL de compilación (`could not find engine`). Los demás tests de audio tampoco compilan mientras tanto, porque comparten `common`.

- [ ] **Step 3: Implementar**

`crates/audio/src/engine.rs`:
```rust
//! The playback state machine: owns the queue, decodes the current track into the sink and,
//! near the end, the next one right behind it (gapless). Synchronous and deterministic: the
//! `Player` thread drives it, and tests drive it directly by calling [`Engine::pump`].
//!
//! Invariant: `queue.current()` is the track being *heard*. A successor already decoded into
//! the sink buffer is only committed to the queue (`on_track_ended`) when it becomes audible.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use goymusic_core::events::{Event, EventBus, PlaybackState};
use goymusic_core::queue::{Previous, Queue, RepeatMode};
use goymusic_core::TrackId;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::convert::Converter;
use crate::decoder::Decoder;
use crate::gain::{self, ReplayGain};
use crate::sink::{AudioSink, SinkFactory, SinkFormat};

/// Decode ahead until the sink holds this much audio.
const BUFFER_TARGET_MS: u64 = 500;
/// Open the next track's decoder when this much of the current one is left to decode.
pub const PRELOAD_MS: u64 = 10_000;
/// Bound on the work done by one `pump` call, so commands stay responsive.
const MAX_CHUNKS_PER_PUMP: usize = 16;

/// Track id → file on disk. The library provides it in the app; tests use a map.
pub type Resolver = Box<dyn Fn(TrackId) -> Option<PathBuf> + Send>;

/// Everything needed to save and restore the player (spec §5: restored paused).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSnapshot {
    pub state: PlaybackState,
    pub track: Option<TrackId>,
    pub position_ms: u64,
    pub volume: f32,
    pub replaygain: bool,
    pub queue: Queue,
}

/// A track being decoded and converted to the sink's format.
struct Feed {
    track: TrackId,
    decoder: Decoder,
    converter: Converter,
    replaygain: f32,
    /// Frames produced so far, in sink frames, counted from the start of the track.
    frames_out: u64,
    /// Track length in sink frames, when the container declares it.
    total_out: Option<u64>,
}

impl Feed {
    /// Next converted block; `None` once the decoder and the resampler are both drained.
    fn next_samples(&mut self, channels: usize) -> Result<Option<Vec<f32>>, String> {
        let mut samples = match self.decoder.next_chunk().map_err(|e| e.to_string())? {
            Some(chunk) => self.converter.push(&chunk).map_err(|e| e.to_string())?,
            None => {
                let tail = self.converter.finish().map_err(|e| e.to_string())?;
                if tail.is_empty() {
                    return Ok(None);
                }
                tail
            }
        };
        gain::apply(&mut samples, self.replaygain);
        self.frames_out += (samples.len() / channels) as u64;
        Ok(Some(samples))
    }

    fn frames_left(&self) -> Option<u64> {
        self.total_out.map(|t| t.saturating_sub(self.frames_out))
    }
}

/// A track's audio inside the sink buffer, starting at frame `start` of what was written.
struct Segment {
    track: TrackId,
    start: u64,
    /// Track position (sink frames) of the segment's first frame: non-zero after a seek.
    offset: u64,
}

pub struct Engine {
    factory: SinkFactory,
    sink: Option<Box<dyn AudioSink>>,
    resolver: Resolver,
    events: Arc<EventBus>,
    queue: Queue,
    rng: StdRng,
    state: PlaybackState,
    published: Option<(PlaybackState, Option<TrackId>)>,
    volume: f32,
    replaygain: bool,
    feed: Option<Feed>,
    preload: Option<Feed>,
    /// Next-track id already tried for preloading, so a broken file is not reopened each pump.
    preload_tried: Option<TrackId>,
    /// Converted samples the sink has not accepted yet.
    pending: Vec<f32>,
    /// Frames accepted by the sink since the last reset.
    written: u64,
    /// `sink.played_frames()` at the last reset.
    played_base: u64,
    segments: VecDeque<Segment>,
    /// Where to resume once a sink is available again (device lost / none at start).
    resume_at: Option<(TrackId, u64)>,
}

impl Engine {
    pub fn new(mut factory: SinkFactory, resolver: Resolver, events: Arc<EventBus>) -> Self {
        let sink = factory().ok();
        let mut engine = Self {
            factory,
            sink,
            resolver,
            events,
            queue: Queue::new(),
            rng: StdRng::from_entropy(),
            state: PlaybackState::Idle,
            published: None,
            volume: 1.0,
            replaygain: false,
            feed: None,
            preload: None,
            preload_tried: None,
            pending: Vec::new(),
            written: 0,
            played_base: 0,
            segments: VecDeque::new(),
            resume_at: None,
        };
        engine.apply_volume();
        engine
    }

    // ---- queries ----

    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// The track being heard.
    pub fn current(&self) -> Option<TrackId> {
        self.segments.front().map(|s| s.track).or(self.queue.current())
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn position_ms(&self) -> u64 {
        let (Some(sink), Some(heard)) = (&self.sink, self.segments.front()) else {
            return self.resume_at.map_or(0, |(_, ms)| ms);
        };
        let played = self.played();
        let frames = heard.offset + played.saturating_sub(heard.start);
        frames * 1000 / u64::from(sink.format().sample_rate)
    }

    /// Next track whose decoder is already open (diagnostics and tests).
    pub fn preloaded(&self) -> Option<TrackId> {
        self.preload.as_ref().map(|f| f.track)
    }

    /// Tracks present in the sink buffer, heard first (diagnostics and tests).
    pub fn buffered_tracks(&self) -> Vec<TrackId> {
        self.segments.iter().map(|s| s.track).collect()
    }

    pub fn needs_sink(&self) -> bool {
        self.sink.is_none()
    }

    pub fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            state: self.state,
            track: self.current(),
            position_ms: self.position_ms(),
            volume: self.volume,
            replaygain: self.replaygain,
            queue: self.queue.clone(),
        }
    }

    // ---- commands ----

    pub fn play_context(&mut self, tracks: Vec<TrackId>, start_index: usize) {
        let first = self.queue.play_context(tracks, start_index, &mut self.rng);
        self.start(first, 0, PlaybackState::Playing);
        self.events.publish(Event::QueueChanged);
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.set_state(PlaybackState::Paused);
        }
    }

    pub fn resume(&mut self) {
        if self.state == PlaybackState::Paused {
            if self.sink.is_none() && !self.try_reopen_sink() {
                return;
            }
            self.set_state(PlaybackState::Playing);
        }
    }

    /// Manual "next": always moves on, even with repeat-one.
    pub fn next(&mut self) {
        if self.queue.current().is_none() {
            return;
        }
        let next = self.queue.next();
        self.start(next, 0, PlaybackState::Playing);
        self.events.publish(Event::QueueChanged);
    }

    pub fn previous(&mut self) {
        let Some(current) = self.queue.current() else {
            return;
        };
        let target = match self.queue.previous(self.position_ms()) {
            Previous::Restart => current,
            Previous::Track(track) => track,
        };
        self.start(Some(target), 0, PlaybackState::Playing);
        self.events.publish(Event::QueueChanged);
    }

    pub fn seek(&mut self, position_ms: u64) {
        if let Some(track) = self.queue.current() {
            let then = if self.state == PlaybackState::Paused {
                PlaybackState::Paused
            } else {
                PlaybackState::Playing
            };
            self.start(Some(track), position_ms, then);
        }
    }

    pub fn set_shuffle(&mut self, on: bool) {
        self.queue.set_shuffle(on, &mut self.rng);
        self.queue_changed();
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.queue.set_repeat(mode);
        self.queue_changed();
    }

    pub fn queue_add(&mut self, tracks: &[TrackId]) {
        self.queue.queue_add(tracks);
        self.queue_changed();
    }

    pub fn queue_remove(&mut self, index: usize) {
        if self.queue.queue_remove(index).is_some() {
            self.queue_changed();
        }
    }

    pub fn queue_move(&mut self, from: usize, to: usize) {
        if self.queue.queue_move(from, to) {
            self.queue_changed();
        }
    }

    /// Volume 0.0–1.0; heard immediately (applied by the sink).
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.apply_volume();
    }

    /// Takes effect from the next track that is opened.
    pub fn set_replaygain(&mut self, on: bool) {
        self.replaygain = on;
    }

    /// Restores a saved queue, paused at `position_ms` of its current track.
    pub fn restore(&mut self, queue: Queue, position_ms: u64) {
        self.queue = queue;
        let current = self.queue.current();
        self.start(current, position_ms, PlaybackState::Paused);
        self.events.publish(Event::QueueChanged);
    }

    // ---- driving ----

    /// Moves audio from the decoder to the sink and notices track changes and the end of
    /// the queue. Call it in a loop; it does a bounded amount of work.
    pub fn pump(&mut self) {
        if let Some(error) = self.sink.as_mut().and_then(|s| s.take_error()) {
            self.device_lost(error.to_string());
            return;
        }
        if self.state != PlaybackState::Playing {
            return;
        }
        let Some(format) = self.sink.as_ref().map(|s| s.format()) else {
            return;
        };
        let target = BUFFER_TARGET_MS * u64::from(format.sample_rate) / 1000;
        for _ in 0..MAX_CHUNKS_PER_PUMP {
            if !self.flush_pending(format) {
                break;
            }
            if self.sink.as_ref().is_some_and(|s| s.queued_frames() >= target) {
                break;
            }
            if !self.decode_more(format) {
                break;
            }
            self.maybe_preload(format);
        }
        self.flush_pending(format);
        self.maybe_preload(format);
        self.update_heard();
        let drained = self.sink.as_ref().is_some_and(|s| s.queued_frames() == 0);
        if self.feed.is_none() && self.pending.is_empty() && drained {
            match self.queue.on_track_ended() {
                Some(next) => self.start(Some(next), 0, PlaybackState::Playing),
                None => {
                    self.set_state(PlaybackState::Ended);
                    self.reset_output();
                }
            }
            self.events.publish(Event::QueueChanged);
        }
    }

    pub fn publish_position(&self) {
        if let (PlaybackState::Playing, Some(track)) = (self.state, self.current()) {
            self.events.publish(Event::PositionChanged {
                track,
                position_ms: self.position_ms(),
            });
        }
    }

    /// Tries to open the output again after a failure. Returns whether a sink is available.
    pub fn try_reopen_sink(&mut self) -> bool {
        if self.sink.is_some() {
            return true;
        }
        match (self.factory)() {
            Ok(sink) => {
                self.sink = Some(sink);
                self.played_base = self.sink.as_ref().map_or(0, |s| s.played_frames());
                self.apply_volume();
                if let Some((track, position_ms)) = self.resume_at.take() {
                    self.start(Some(track), position_ms, PlaybackState::Paused);
                }
                true
            }
            Err(_) => false,
        }
    }

    // ---- internals ----

    fn played(&self) -> u64 {
        self.sink
            .as_ref()
            .map_or(0, |s| s.played_frames())
            .saturating_sub(self.played_base)
    }

    fn apply_volume(&mut self) {
        let amplitude = gain::volume_to_amplitude(self.volume);
        if let Some(sink) = self.sink.as_mut() {
            sink.set_volume(amplitude);
        }
    }

    fn set_state(&mut self, state: PlaybackState) {
        self.state = state;
        if let Some(sink) = self.sink.as_mut() {
            sink.set_paused(state != PlaybackState::Playing);
        }
        let now = (state, self.current());
        if self.published != Some(now) {
            self.published = Some(now);
            self.events.publish(Event::PlayerStateChanged {
                state: now.0,
                track: now.1,
            });
        }
    }

    /// Drops all buffered audio and decoders.
    fn reset_output(&mut self) {
        if let Some(sink) = self.sink.as_mut() {
            sink.clear();
        }
        self.played_base = self.sink.as_ref().map_or(0, |s| s.played_frames());
        self.written = 0;
        self.pending.clear();
        self.feed = None;
        self.preload = None;
        self.preload_tried = None;
        self.segments.clear();
    }

    /// Starts `first` at `position_ms`. A track that cannot be opened is reported with
    /// `PlaybackError` and skipped, at most once per track in the queue, so a queue of
    /// broken files on repeat ends instead of looping forever.
    fn start(&mut self, first: Option<TrackId>, position_ms: u64, then: PlaybackState) {
        self.reset_output();
        if self.sink.is_none() && !self.try_reopen_sink() {
            if let Some(track) = first {
                self.resume_at = Some((track, position_ms));
                self.set_state(PlaybackState::Paused);
                self.events.publish(Event::AudioDeviceLost {
                    reason: "no audio output device".into(),
                });
            }
            return;
        }
        let format = self.sink.as_ref().expect("sink checked above").format();
        let mut attempts = self.queue.context().len() + self.queue.user_queue().len() + 1;
        let mut candidate = first;
        let mut position_ms = position_ms;
        while let Some(track) = candidate {
            self.segments.clear();
            self.segments.push_back(Segment {
                track,
                start: 0,
                offset: 0,
            });
            self.set_state(PlaybackState::Loading);
            match self.open_feed(track, position_ms, format) {
                Ok(feed) => {
                    self.segments[0].offset = feed.frames_out;
                    self.feed = Some(feed);
                    self.set_state(then);
                    return;
                }
                Err(reason) => {
                    self.events.publish(Event::PlaybackError { track, reason });
                    attempts -= 1;
                    if attempts == 0 {
                        break;
                    }
                    candidate = self.queue.next();
                    position_ms = 0;
                }
            }
        }
        self.segments.clear();
        self.set_state(if first.is_some() {
            PlaybackState::Ended
        } else {
            PlaybackState::Idle
        });
    }

    fn open_feed(&self, track: TrackId, position_ms: u64, format: SinkFormat) -> Result<Feed, String> {
        let path = (self.resolver)(track).ok_or_else(|| "track not in library".to_string())?;
        let mut decoder = Decoder::open(&path).map_err(|e| e.to_string())?;
        let in_rate = u64::from(decoder.sample_rate());
        let out_rate = u64::from(format.sample_rate);
        let start_frame = position_ms * in_rate / 1000;
        if start_frame > 0 {
            decoder.seek(start_frame).map_err(|e| e.to_string())?;
        }
        let converter = Converter::new(
            decoder.sample_rate(),
            decoder.channels(),
            format.sample_rate,
            format.channels as usize,
        )
        .map_err(|e| e.to_string())?;
        let replaygain = if self.replaygain {
            ReplayGain::read(&path).factor()
        } else {
            1.0
        };
        Ok(Feed {
            track,
            total_out: decoder.total_frames().map(|n| n * out_rate / in_rate),
            frames_out: start_frame * out_rate / in_rate,
            decoder,
            converter,
            replaygain,
        })
    }

    /// Writes pending samples; `true` when nothing is left pending.
    fn flush_pending(&mut self, format: SinkFormat) -> bool {
        if self.pending.is_empty() {
            return true;
        }
        let Some(sink) = self.sink.as_mut() else {
            return false;
        };
        let accepted = sink.write(&self.pending);
        self.written += (accepted / format.channels as usize) as u64;
        self.pending.drain(..accepted);
        self.pending.is_empty()
    }

    /// Decodes one block into `pending`. At the end of a track (or on a decode error) the
    /// successor starts feeding right behind it. `false` when there is nothing to decode.
    fn decode_more(&mut self, format: SinkFormat) -> bool {
        let Some(feed) = self.feed.as_mut() else {
            return false;
        };
        match feed.next_samples(format.channels as usize) {
            Ok(Some(samples)) => {
                self.pending.extend(samples);
                true
            }
            Ok(None) => {
                self.feed = None;
                self.start_successor(format);
                true
            }
            Err(reason) => {
                let track = feed.track;
                self.feed = None;
                self.events.publish(Event::PlaybackError { track, reason });
                self.start_successor(format);
                true
            }
        }
    }

    /// The track that follows the last one in the buffer, if the queue were left alone.
    fn successor(&self) -> Option<TrackId> {
        let mut queue = self.queue.clone();
        let mut next = None;
        for _ in 0..self.segments.len() {
            next = queue.on_track_ended();
        }
        next
    }

    fn start_successor(&mut self, format: SinkFormat) {
        let Some(track) = self.successor() else {
            return;
        };
        let feed = match self.preload.take() {
            Some(preloaded) if preloaded.track == track => Ok(preloaded),
            _ => self.open_feed(track, 0, format),
        };
        // A successor that fails to open is left alone: when the buffer drains, `pump`
        // advances the queue and `start` reports the error and skips it.
        if let Ok(feed) = feed {
            let start = self.written + (self.pending.len() / format.channels as usize) as u64;
            self.segments.push_back(Segment {
                track,
                start,
                offset: 0,
            });
            self.feed = Some(feed);
        }
    }

    fn maybe_preload(&mut self, format: SinkFormat) {
        if self.preload.is_some() || self.segments.len() != 1 {
            return;
        }
        let rate = u64::from(format.sample_rate);
        let near_end = self
            .feed
            .as_ref()
            .and_then(Feed::frames_left)
            .is_some_and(|left| left <= PRELOAD_MS * rate / 1000);
        if !near_end {
            return;
        }
        let Some(next) = self.queue.peek_next() else {
            return;
        };
        if self.preload_tried == Some(next) {
            return;
        }
        self.preload_tried = Some(next);
        self.preload = self.open_feed(next, 0, format).ok();
    }

    /// Commits successors that became audible to the queue and announces them.
    fn update_heard(&mut self) {
        let played = self.played();
        while self.segments.len() > 1 && self.segments[1].start <= played {
            self.segments.pop_front();
            let heard = self.segments[0].track;
            let committed = self.queue.on_track_ended();
            debug_assert_eq!(committed, Some(heard), "successor must match the queue");
            self.preload_tried = None;
            let state = self.state;
            self.set_state(state);
            self.events.publish(Event::QueueChanged);
        }
    }

    /// After a queue edit: a preload or a buffered successor that is no longer next is
    /// dropped. A buffered one means restarting the heard track where it is (rare: only in
    /// the last half second of a track).
    fn queue_changed(&mut self) {
        let next = self.queue.peek_next();
        if self.preload.as_ref().is_some_and(|p| Some(p.track) != next) {
            self.preload = None;
        }
        self.preload_tried = None;
        if self.segments.len() > 1 && Some(self.segments[1].track) != next {
            if let Some(track) = self.queue.current() {
                let position_ms = self.position_ms();
                let state = self.state;
                self.start(Some(track), position_ms, state);
            }
        }
        self.events.publish(Event::QueueChanged);
    }

    fn device_lost(&mut self, reason: String) {
        if let Some(track) = self.current() {
            self.resume_at = Some((track, self.position_ms()));
        }
        self.sink = None;
        self.feed = None;
        self.preload = None;
        self.pending.clear();
        self.segments.clear();
        self.set_state(PlaybackState::Paused);
        self.events.publish(Event::AudioDeviceLost { reason });
    }
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS (15 tests en `engine`; el resto de tests de audio vuelve a compilar y pasar).

- [ ] **Step 5: Commit**

```bash
git add crates/audio
git commit -m "Audio: motor de reproducción gapless con precarga y recuperación de errores"
```

---

### Task 7: Hilo de audio y API del servicio Player

**Files:**
- Create: `crates/audio/src/player.rs`
- Modify: `crates/audio/src/lib.rs` (añadir `pub mod player;` tras `pub mod gain;`)
- Test: `crates/audio/tests/player.rs`

**Interfaces:**
- Consumes: `Engine`, `PlayerSnapshot`, `Resolver` (T6), `SinkFactory` (T5).
- Produces: `goymusic_audio::player::Player`:
  - `spawn(SinkFactory, Resolver, Arc<EventBus>) -> Player`.
  - Métodos de la spec §4:
    - `play(Vec<TrackId>, start_index)`, `pause`, `resume`, `next`, `previous`, `seek(ms)`
    - `set_shuffle`, `set_repeat`, `set_volume`, `set_replaygain`
    - `queue_add(Vec<TrackId>)`, `queue_remove`, `queue_move`, `restore(Queue, ms)`
    - `state() -> Option<PlayerSnapshot>`
  - `Drop` para el hilo y espera a que termine.
  - El hilo bombea cada 5 ms mientras suena y cada 250 ms en reposo.
  - Publica la posición cada 250 ms y, sin dispositivo, reintenta abrirlo cada 2 s.

- [ ] **Step 1: Escribir el test que falla**

`crates/audio/tests/player.rs`:
```rust
mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{capture_factory, resolver, sine, write_wav};
use goymusic_audio::player::Player;
use goymusic_audio::sink::SinkFormat;
use goymusic_core::events::{Event, EventBus, PlaybackState};
use goymusic_core::TrackId;

#[test]
fn plays_a_context_on_its_own_thread_and_stops_on_drop() {
    let dir = tempfile::tempdir().unwrap();
    let files: Vec<_> = (1..=2)
        .map(|i| {
            let p = dir.path().join(format!("{i}.wav"));
            write_wav(&p, 8_000, 1, &sine(8_000, 1, 440.0, 0, 4_000));
            (i, p)
        })
        .collect();
    let bus = Arc::new(EventBus::new());
    let events = bus.subscribe();
    let (factory, sinks, _) = capture_factory(SinkFormat {
        sample_rate: 8_000,
        channels: 1,
    });
    let player = Player::spawn(factory, resolver(&files), bus.clone());

    player.set_volume(1.0);
    player.play(vec![TrackId(1), TrackId(2)], 0);
    let deadline = Instant::now() + Duration::from_secs(5);
    let ended = Event::PlayerStateChanged {
        state: PlaybackState::Ended,
        track: Some(TrackId(2)),
    };
    loop {
        let e = events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("no Ended event");
        if e == ended {
            break;
        }
    }

    let state = player.state().expect("thread answers");
    assert_eq!(state.state, PlaybackState::Ended);
    assert_eq!(sinks.lock().unwrap()[0].played().len(), 8_000);
    drop(player);
    assert_eq!(bus.subscriber_count(), 1, "thread joined, bus intact");
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-audio --test player`
Expected: FAIL de compilación (`could not find player`).

- [ ] **Step 3: Implementar**

`crates/audio/src/player.rs`:
```rust
//! Runs the [`Engine`] on its own thread (spec §5) and exposes the player service API (§4).
//! Commands go in over a channel; state comes out as events on the core `EventBus`.

use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use goymusic_core::events::{EventBus, PlaybackState};
use goymusic_core::queue::{Queue, RepeatMode};
use goymusic_core::TrackId;

use crate::engine::{Engine, PlayerSnapshot, Resolver};
use crate::sink::SinkFactory;

/// Poll interval while audio is flowing: well under the 500 ms buffer.
const ACTIVE_TICK: Duration = Duration::from_millis(5);
const IDLE_TICK: Duration = Duration::from_millis(250);
const POSITION_EVERY: Duration = Duration::from_millis(250);
const DEVICE_RETRY_EVERY: Duration = Duration::from_secs(2);

enum Command {
    PlayContext(Vec<TrackId>, usize),
    Pause,
    Resume,
    Next,
    Previous,
    Seek(u64),
    SetShuffle(bool),
    SetRepeat(RepeatMode),
    SetVolume(f32),
    SetReplayGain(bool),
    QueueAdd(Vec<TrackId>),
    QueueRemove(usize),
    QueueMove(usize, usize),
    Restore(Queue, u64),
    Snapshot(Sender<PlayerSnapshot>),
    Shutdown,
}

/// Handle to the player thread. Dropping it stops playback and joins the thread.
pub struct Player {
    commands: Sender<Command>,
    thread: Option<JoinHandle<()>>,
}

impl Player {
    pub fn spawn(factory: SinkFactory, resolver: Resolver, events: Arc<EventBus>) -> Self {
        let (commands, rx) = channel();
        let thread = std::thread::Builder::new()
            .name("goymusic-audio".into())
            .spawn(move || run(Engine::new(factory, resolver, events), rx))
            .expect("spawn audio thread");
        Self {
            commands,
            thread: Some(thread),
        }
    }

    fn send(&self, command: Command) {
        // the thread only exits on Shutdown, so a send error means it panicked; nothing to do
        let _ = self.commands.send(command);
    }

    pub fn play(&self, context: Vec<TrackId>, start_index: usize) {
        self.send(Command::PlayContext(context, start_index));
    }
    pub fn pause(&self) {
        self.send(Command::Pause);
    }
    pub fn resume(&self) {
        self.send(Command::Resume);
    }
    pub fn next(&self) {
        self.send(Command::Next);
    }
    pub fn previous(&self) {
        self.send(Command::Previous);
    }
    pub fn seek(&self, position_ms: u64) {
        self.send(Command::Seek(position_ms));
    }
    pub fn set_shuffle(&self, on: bool) {
        self.send(Command::SetShuffle(on));
    }
    pub fn set_repeat(&self, mode: RepeatMode) {
        self.send(Command::SetRepeat(mode));
    }
    pub fn set_volume(&self, volume: f32) {
        self.send(Command::SetVolume(volume));
    }
    pub fn set_replaygain(&self, on: bool) {
        self.send(Command::SetReplayGain(on));
    }
    pub fn queue_add(&self, tracks: Vec<TrackId>) {
        self.send(Command::QueueAdd(tracks));
    }
    pub fn queue_remove(&self, index: usize) {
        self.send(Command::QueueRemove(index));
    }
    pub fn queue_move(&self, from: usize, to: usize) {
        self.send(Command::QueueMove(from, to));
    }
    pub fn restore(&self, queue: Queue, position_ms: u64) {
        self.send(Command::Restore(queue, position_ms));
    }

    /// Current state, answered by the audio thread after every command sent before it.
    pub fn state(&self) -> Option<PlayerSnapshot> {
        let (tx, rx) = channel();
        self.send(Command::Snapshot(tx));
        rx.recv_timeout(Duration::from_secs(2)).ok()
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(mut engine: Engine, commands: Receiver<Command>) {
    let mut last_position = Instant::now();
    let mut last_retry = Instant::now();
    loop {
        let tick = if engine.state() == PlaybackState::Playing {
            ACTIVE_TICK
        } else {
            IDLE_TICK
        };
        match commands.recv_timeout(tick) {
            Ok(command) => {
                if !apply(&mut engine, command) {
                    return;
                }
                while let Ok(command) = commands.try_recv() {
                    if !apply(&mut engine, command) {
                        return;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        engine.pump();
        if last_position.elapsed() >= POSITION_EVERY {
            engine.publish_position();
            last_position = Instant::now();
        }
        if engine.needs_sink() && last_retry.elapsed() >= DEVICE_RETRY_EVERY {
            engine.try_reopen_sink();
            last_retry = Instant::now();
        }
    }
}

/// `false` on shutdown.
fn apply(engine: &mut Engine, command: Command) -> bool {
    match command {
        Command::PlayContext(tracks, start) => engine.play_context(tracks, start),
        Command::Pause => engine.pause(),
        Command::Resume => engine.resume(),
        Command::Next => engine.next(),
        Command::Previous => engine.previous(),
        Command::Seek(ms) => engine.seek(ms),
        Command::SetShuffle(on) => engine.set_shuffle(on),
        Command::SetRepeat(mode) => engine.set_repeat(mode),
        Command::SetVolume(v) => engine.set_volume(v),
        Command::SetReplayGain(on) => engine.set_replaygain(on),
        Command::QueueAdd(tracks) => engine.queue_add(&tracks),
        Command::QueueRemove(i) => engine.queue_remove(i),
        Command::QueueMove(from, to) => engine.queue_move(from, to),
        Command::Restore(queue, ms) => engine.restore(queue, ms),
        Command::Snapshot(reply) => {
            let _ = reply.send(engine.snapshot());
        }
        Command::Shutdown => return false,
    }
    true
}
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS (1 test en `player`).

- [ ] **Step 5: Commit**

```bash
git add crates/audio
git commit -m "Audio: hilo de reproducción con la API del servicio Player"
```

---

### Task 8: Salida real con cpal, ejemplo de escucha y CI

**Files:**
- Create: `crates/audio/src/cpal_sink.rs`, `crates/audio/examples/play.rs`
- Modify: `crates/audio/src/lib.rs` (versión final abajo), `.github/workflows/ci.yml`
- Test: `crates/audio/tests/cpal_device.rs`, `crates/audio/tests/mp3_gapless.rs` (ambos `#[ignore]`)

**Interfaces:**
- Consumes: `AudioSink`, `SinkError`, `SinkFormat` (T5); `Player` (T7); `Rig` (T6).
- Produces: `goymusic_audio::cpal_sink::CpalSink::open_default() -> Result<CpalSink, SinkError>`. Usa el formato por defecto del dispositivo (f32/i16/u16/i32) y un búfer circular de 1 s. La pausa emite silencio sin parar el stream. `clear` espera hasta 200 ms a que el callback vacíe el búfer.

- [ ] **Step 1: Escribir los tests que fallan**

`crates/audio/tests/cpal_device.rs`:
```rust
//! Needs a real output device: `cargo test -p goymusic-audio --test cpal_device -- --ignored`

use std::time::{Duration, Instant};

use goymusic_audio::cpal_sink::CpalSink;
use goymusic_audio::sink::AudioSink;

#[test]
#[ignore = "needs an audio device"]
fn default_device_consumes_written_audio() {
    let mut sink = CpalSink::open_default().expect("default output device");
    let format = sink.format();
    sink.set_volume(0.0); // silent check
    let second = vec![0.1; format.sample_rate as usize * format.channels as usize / 2];
    assert!(sink.write(&second) > 0);
    let deadline = Instant::now() + Duration::from_secs(3);
    while sink.played_frames() == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(sink.played_frames() > 0);
    sink.clear();
    assert_eq!(sink.queued_frames(), 0);
}
```

`crates/audio/tests/mp3_gapless.rs`:
```rust
//! Needs ffmpeg with libmp3lame (MP3 encoders are not pure Rust):
//! `cargo test -p goymusic-audio --test mp3_gapless -- --ignored`
mod common;

use std::path::Path;
use std::process::Command;

use common::{heard, longest_quiet_run, sine, write_wav, Rig};
use goymusic_audio::sink::SinkFormat;
use goymusic_core::TrackId;

fn to_mp3(wav: &Path, mp3: &Path) {
    let status = Command::new("ffmpeg")
        .args(["-loglevel", "error", "-y", "-i"])
        .arg(wav)
        .args(["-c:a", "libmp3lame", "-b:a", "192k"])
        .arg(mp3)
        .status()
        .expect("ffmpeg not installed");
    assert!(status.success());
}

#[test]
#[ignore = "needs ffmpeg with libmp3lame"]
fn mp3_album_plays_without_gaps() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = Vec::new();
    for (i, (start, frames)) in [(0u64, 50_000u64), (50_000, 40_000)].iter().enumerate() {
        let wav = dir.path().join(format!("{i}.wav"));
        let mp3 = dir.path().join(format!("{i}.mp3"));
        write_wav(&wav, 44_100, 2, &sine(44_100, 2, 440.0, *start, *frames));
        to_mp3(&wav, &mp3);
        files.push((i as i64 + 1, mp3));
    }
    let mut rig = Rig::new(
        SinkFormat {
            sample_rate: 44_100,
            channels: 2,
        },
        &files,
    );

    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    rig.run(10_000);

    let played = rig.sink(0).played();
    assert_eq!(played.len() / 2, 90_000, "encoder delay and padding trimmed exactly");
    assert!(
        longest_quiet_run(&played[2_000..], 2, 0.01) <= 3,
        "silence at the boundary"
    );
    assert_eq!(heard(&rig.drain_events()), vec![1, 2]);
}
```

- [ ] **Step 2: Ejecutar y verificar que falla**

Run: `cargo test -p goymusic-audio --test cpal_device -- --ignored`
Expected: FAIL de compilación (`could not find cpal_sink`).

- [ ] **Step 3: Implementar**

`crates/audio/src/lib.rs` final:
```rust
//! GoyMusic audio engine: file → symphonia → channel mapping + rubato → ReplayGain → sink.

pub mod convert;
pub mod cpal_sink;
pub mod decoder;
pub mod engine;
pub mod error;
pub mod gain;
pub mod player;
pub mod sink;

pub use error::AudioError;
```

`crates/audio/src/cpal_sink.rs`:
```rust
//! The real output: default device through cpal, fed by a lock-free ring buffer. The audio
//! callback never blocks or allocates in the steady state; it pops samples, applies the
//! volume and counts played frames. Pause outputs silence instead of stopping the stream,
//! so `clear` can always rely on the callback to empty the buffer.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};

use crate::sink::{AudioSink, SinkError, SinkFormat};

/// Ring buffer size: one second of audio, double the engine's 500 ms target.
const BUFFER_SECONDS: usize = 1;

#[derive(Default)]
struct Shared {
    played_frames: AtomicU64,
    paused: AtomicBool,
    flush: AtomicBool,
    /// `f32` bits of the output amplitude.
    amplitude: AtomicU32,
    error: Mutex<Option<String>>,
}

pub struct CpalSink {
    format: SinkFormat,
    producer: HeapProd<f32>,
    shared: Arc<Shared>,
    // Keeps the stream alive. Not `Send`: the sink lives on the engine thread.
    _stream: cpal::Stream,
}

impl CpalSink {
    pub fn open_default() -> Result<Self, SinkError> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(SinkError::NoDevice)?;
        let supported = device
            .default_output_config()
            .map_err(|e| SinkError::Device(e.to_string()))?;
        let config = supported.config();
        let format = SinkFormat {
            sample_rate: config.sample_rate.0,
            channels: config.channels,
        };
        let capacity = format.sample_rate as usize * format.channels as usize * BUFFER_SECONDS;
        let (producer, consumer) = HeapRb::<f32>::new(capacity).split();
        let shared = Arc::new(Shared {
            amplitude: AtomicU32::new(1f32.to_bits()),
            ..Default::default()
        });
        let stream = match supported.sample_format() {
            SampleFormat::F32 => build::<f32>(&device, &config, consumer, shared.clone()),
            SampleFormat::I16 => build::<i16>(&device, &config, consumer, shared.clone()),
            SampleFormat::U16 => build::<u16>(&device, &config, consumer, shared.clone()),
            SampleFormat::I32 => build::<i32>(&device, &config, consumer, shared.clone()),
            other => return Err(SinkError::Device(format!("unsupported sample format {other}"))),
        }?;
        stream.play().map_err(|e| SinkError::Device(e.to_string()))?;
        Ok(Self {
            format,
            producer,
            shared,
            _stream: stream,
        })
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: HeapCons<f32>,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, SinkError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let errors = shared.clone();
    let mut scratch: Vec<f32> = Vec::with_capacity(16_384);
    device
        .build_output_stream(
            config,
            move |out: &mut [T], _| {
                // empty first, acknowledge second: `clear` returns as soon as it sees `false`
                if shared.flush.load(Ordering::Acquire) {
                    consumer.clear();
                    shared.flush.store(false, Ordering::Release);
                }
                let n = if shared.paused.load(Ordering::Acquire) {
                    0
                } else {
                    scratch.resize(out.len(), 0.0);
                    consumer.pop_slice(&mut scratch)
                };
                let amplitude = f32::from_bits(shared.amplitude.load(Ordering::Relaxed));
                for (o, s) in out.iter_mut().zip(&scratch[..n]) {
                    *o = T::from_sample(s * amplitude);
                }
                for o in &mut out[n..] {
                    *o = T::EQUILIBRIUM;
                }
                shared.played_frames.fetch_add((n / channels) as u64, Ordering::Release);
            },
            move |err| {
                *errors.error.lock().unwrap() = Some(err.to_string());
            },
            None,
        )
        .map_err(|e| SinkError::Device(e.to_string()))
}

impl AudioSink for CpalSink {
    fn format(&self) -> SinkFormat {
        self.format
    }

    fn write(&mut self, samples: &[f32]) -> usize {
        let channels = self.format.channels as usize;
        let room = self.producer.vacant_len() / channels * channels;
        self.producer.push_slice(&samples[..samples.len().min(room)])
    }

    fn queued_frames(&self) -> u64 {
        (self.producer.occupied_len() / self.format.channels as usize) as u64
    }

    fn played_frames(&self) -> u64 {
        self.shared.played_frames.load(Ordering::Acquire)
    }

    fn set_paused(&mut self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Release);
    }

    fn set_volume(&mut self, amplitude: f32) {
        self.shared.amplitude.store(amplitude.to_bits(), Ordering::Relaxed);
    }

    /// Asks the callback to empty the buffer and waits for it (it runs every few ms even
    /// when paused). Gives up after 200 ms, e.g. if the device died meanwhile.
    fn clear(&mut self) {
        self.shared.flush.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_millis(200);
        while self.shared.flush.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn take_error(&mut self) -> Option<SinkError> {
        self.shared.error.lock().unwrap().take().map(SinkError::Device)
    }
}
```

**Por qué el orden importa en el callback.** Primero se vacía y después se pone `flush` a `false`. En el orden inverso, `clear()` ve el `false` y vuelve antes de que el búfer esté vacío. Es una carrera real, encontrada con el dispositivo de esta máquina.

`crates/audio/examples/play.rs`:
```rust
//! Manual check on a real device: plays the given files in order, gapless.
//! `cargo run -p goymusic-audio --example play -- a.flac b.flac`

use std::path::PathBuf;
use std::sync::Arc;

use goymusic_audio::cpal_sink::CpalSink;
use goymusic_audio::player::Player;
use goymusic_audio::sink::AudioSink;
use goymusic_core::events::{Event, EventBus, PlaybackState};
use goymusic_core::TrackId;

fn main() {
    let files: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if files.is_empty() {
        eprintln!("usage: play <file>...");
        std::process::exit(2);
    }
    let bus = Arc::new(EventBus::new());
    let events = bus.subscribe();
    let paths = files.clone();
    let player = Player::spawn(
        Box::new(|| CpalSink::open_default().map(|s| Box::new(s) as Box<dyn AudioSink>)),
        Box::new(move |id: TrackId| paths.get(id.0 as usize).cloned()),
        bus,
    );
    player.play((0..files.len() as i64).map(TrackId).collect(), 0);
    for event in events {
        match event {
            Event::PlayerStateChanged {
                state: PlaybackState::Ended,
                ..
            } => break,
            Event::PlayerStateChanged { state, track: Some(t) } => {
                println!("{state:?}: {}", files[t.0 as usize].display())
            }
            Event::PlaybackError { track, reason } => {
                eprintln!("error in {}: {reason}", files[track.0 as usize].display())
            }
            Event::AudioDeviceLost { reason } => eprintln!("device lost: {reason}"),
            _ => {}
        }
    }
}
```

En `.github/workflows/ci.yml`:
- cambiar la línea de apt de la Task 2 por:
```yaml
      - run: sudo apt-get update && sudo apt-get install -y libasound2-dev pkg-config ffmpeg
```
- añadir al final:
```yaml
      - run: cargo test -p goymusic-audio --test mp3_gapless -- --ignored
```

- [ ] **Step 4: Ejecutar y verificar que pasa**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS (los dos tests nuevos aparecen como `ignored`).

Run: `cargo test -p goymusic-audio --test cpal_device -- --ignored && cargo test -p goymusic-audio --test mp3_gapless -- --ignored`
Expected: PASS. Hace falta un dispositivo de salida y ffmpeg con libmp3lame. El primero es silencioso, porque pone el volumen a 0.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml crates/audio
git commit -m "Audio: salida real con cpal, ejemplo de escucha y MP3 gapless en CI"
```

---

## Verificación final

- [ ] `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`: todo en verde (55 tests más 2 `ignored`).
- [ ] **Prueba de escucha (criterio 3).** La hace Javi. Hay que reproducir un álbum gapless propio en FLAC y otro en MP3 (por ejemplo, uno en directo o uno conceptual):
  `cargo run --release -p goymusic-audio --example play -- /ruta/01.flac /ruta/02.flac /ruta/03.flac`
  Esperado: sin clic ni silencio entre pistas, y el título de cada pista aparece al empezar a oírse.
- [ ] Revisión final de toda la rama con un revisor Opus (método nativo). Atención especial a `engine.rs`: el invariante "la cola avanza al oírse", la reconstrucción tras editar la cola y la ruta de dispositivo perdido.
- [ ] Fusión a `master` con superpowers:finishing-a-development-branch.

## Cobertura de la spec en P2

| Spec | Tarea |
|---|---|
| §3 eventos `PlayerStateChanged`, `PositionChanged` (~4 Hz), `PlaybackError` | 1, 6, 7 |
| §4 Player: `play`, `pause`, `resume`, `seek`, `next`, `previous`, `set_shuffle`, `set_repeat`, `set_volume`, `queue_add/remove/move`, `state` | 6, 7 |
| §5 hilo dedicado, estados `Idle → Loading → Playing ⇄ Paused → Ended`, posición por contador de muestras | 6, 7 |
| §5 cadena symphonia → rubato → ganancia → búfer circular sin bloqueos → cpal | 2, 3, 4, 8 |
| §5 formatos MP3, FLAC, Ogg Vorbis, WAV, AAC/M4A, ALAC | 2 (features de symphonia; WAV, FLAC y MP3 con test) |
| §5 gapless: precarga ~10 s y recorte de retardo/relleno | 2 (`enable_gapless`), 6, 8 (MP3) |
| §5 cola: prioridad de la cola de usuario, shuffle, repeat, "anterior" > 3 s | P1 + 6 |
| §5 restauración en pausa | 6 (`restore`, `snapshot`); el guardado en disco va en P3 |
| §5 volumen por software, ReplayGain opcional (desactivado por defecto) | 4, 5, 6 |
| §5 `AudioSink` (cpal + implementación de prueba que captura muestras) | 5, 8 |
| §9 archivo ilegible → siguiente + `PlaybackError` | 6 |
| §9 dispositivo perdido → pausa, evento, reintento con el dispositivo por defecto | 6, 7 |
| §9 pruebas de audio: gapless sin silencio, seek a la muestra exacta, resampleo; audio generado en el test | 2, 3, 6, 8 |
| §10 criterio 3 (álbum gapless FLAC y MP3 sin silencio, también por test) | 6, 8 + prueba de escucha |
| §6 resolver `TrackId → ruta` desde la biblioteca, persistencia de `queue_state`; §7 UI; §8 MPRIS | P3, P4 |
