# GoyMusic P4 — Integración de escritorio y empaquetado: plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que GoyMusic se comporte como un reproductor más del escritorio Linux. Eso significa:
- MPRIS: teclas multimedia, GNOME y `playerctl`;
- notificación con carátula al cambiar de canción;
- bandeja: cerrar la ventana no corta la música;
- instancia única y mini-reproductor;
- paquetes AppImage y `.deb` instalables.

**Architecture:** todo vive en `apps/desktop`, como adaptadores sobre `Services`, igual que los comandos Tauri. El núcleo no cambia.

MPRIS y notificaciones se escriben contra un trait propio, `PlayerControl`:
- `Services` lo implementa en producción;
- los tests usan un reproductor falso, así que la lógica se prueba sin D-Bus;
- un test de integración (`#[ignore]`) levanta el servidor MPRIS en un bus de sesión aislado (`dbus-run-session`) y lo controla como lo haría `playerctl`.

Un hilo escucha el `EventBus` y traduce los eventos del núcleo a señales MPRIS y notificaciones. La bandeja usa la del propio Tauri (`tray-icon`). El mini-reproductor es una segunda ventana Tauri con una ruta propia de la UI.

**Tech Stack:**
- Rust: Tauri 2.12.1 (feature `tray-icon`), `mpris-server`, `notify-rust` y `tauri-plugin-single-instance`, los tres sobre zbus 5.
- UI: React (ruta `/mini`).
- Bundler de Tauri: `deb` y `appimage`.

**Spec:** `docs/superpowers/specs/2026-10-16-goymusic-design.md`. Secciones: §8 (integración de escritorio), §9 (empaquetado), §10 criterios 7 y 8.

**Diseño visual:** `design.md` («Color vivo»). La UI nueva (mini-reproductor) usa sus tokens y `GenArt`.

## Global Constraints

**Dependencias**
- Versiones fijadas con `=`, todas publicadas hace más de dos semanas:
  - `mpris-server =0.10.0`
  - `notify-rust =4.18.1`, con `default-features = false` y features `["z"]` (backend zbus); no hace falta el crate `image`, porque se pasa la ruta del fichero.
  - `tauri-plugin-single-instance =2.5.2`
  - `tauri` añade las features `["tray-icon", "image-png"]`
- Si una API no cuadra, consultar la documentación de esa versión (docs.rs o el código descargado); no cambiar la versión.

**Producto**
- Identidad MPRIS: `Identity` = «GoyMusic», `DesktopEntry` = el nombre del `.desktop` que genere el bundler (se comprueba en la Task 5).
- Bus name: `org.mpris.MediaPlayer2.goymusic`.
- `TrackId` MPRIS: `/org/mpris/MediaPlayer2/Track/<id>`, o `/org/mpris/MediaPlayer2/TrackList/NoTrack` si no hay pista.
- Carátula en MPRIS y en la notificación: `file://` de la miniatura grande (`Services::album_art(album, ArtSize::Large)`). Si no hay (pistas sin álbum, el caso de Javi): sin `artUrl` en MPRIS y el icono de la app en la notificación.
- `Position` no se emite en cada tick: MPRIS lo lee bajo demanda, y al saltar con seek se emite la señal `Seeked`.
- Textos visibles en español.

**Proceso**
- Gate por tarea:
  ```
  cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && source ~/.nvm/nvm.sh && pnpm --dir ui test && pnpm --dir ui build
  ```
- Los tests de D-Bus se ejecutan con:
  ```
  dbus-run-session -- cargo test -p goymusic-desktop -- --ignored
  ```
- Commits en español; un commit por tarea (reset `--soft` a la base de la tarea).
- **Parada obligatoria:**
  - Javi comprueba a mano en GNOME las teclas multimedia, la notificación, la bandeja y la instancia única. No se pueden automatizar desde aquí.
  - Instalar el `.deb` requiere sudo en su terminal.

## Review Focus

1. **Segunda instancia:**
   - La segunda instancia no abre la base de datos, ni el motor de audio, ni lanza un escaneo, antes de ceder y enfocar la primera. Por eso `Services::start` pasa al `setup` de Tauri, detrás del plugin de instancia única.
   - Test: `services_start_after_the_single_instance_check` (Task 3).
2. **Cerrar la ventana:**
   - Con la bandeja disponible, la oculta y la música sigue.
   - «Salir» (bandeja o MPRIS `Quit`) apaga una sola vez y guarda la cola.
   - Si la bandeja no se pudo crear (GNOME sin la extensión AppIndicator), cerrar = salir. Así nunca queda una app invisible sin forma de volver.
   - Test: `closing_hides_only_when_the_tray_exists` (Task 3).
3. **Sin pista o con pistas sin etiquetas:**
   - MPRIS publica metadatos mínimos (título del fichero, sin álbum ni carátula) y `NoTrack` cuando no hay nada; nunca hay pánico.
   - Tests en la Task 1.
4. **Sin servidor de notificaciones o sin bus de sesión:**
   - Se registra un aviso en el log y la música sigue. Ni la notificación ni MPRIS pueden bloquear o tumbar el hilo de eventos.
   - Tests: `a_failing_notifier_does_not_stop_the_event_loop` (Task 2) y `mpris_unavailable_is_logged_not_fatal` (Task 1).
5. **Ráfagas de eventos** (cola larga, saltos rápidos):
   - Solo se notifica la pista que de verdad empieza a sonar, y una notificación sustituye a la anterior en vez de apilarse.
   - Test: `only_the_track_that_starts_playing_is_notified` (Task 2).

---

### Task 1: MPRIS sobre un trait de control

**Files:**
- Modify: `Cargo.toml` (deps del workspace), `apps/desktop/Cargo.toml`, `apps/desktop/src/lib.rs`
- Create:
  - `apps/desktop/src/control.rs` (trait `PlayerControl` + impl para `Services`)
  - `apps/desktop/src/mpris.rs`
  - `apps/desktop/tests/mpris_bus.rs` (`#[ignore]`, D-Bus real)

**Interfaces (Produces):**

```rust
// control.rs — what desktop integrations need from the player; Services implements it
pub trait PlayerControl: Send + Sync + 'static {
    fn view(&self) -> Option<PlayerView>;                 // None when the player is gone (shutdown)
    fn art_path(&self, album: AlbumId) -> Option<PathBuf>; // large cached thumbnail
    fn play_pause(&self); fn play(&self); fn pause(&self); fn stop(&self);
    fn next(&self); fn previous(&self);
    fn seek_to(&self, position_ms: u64);
    fn set_volume(&self, volume: f32);
    fn set_shuffle(&self, on: bool); fn set_repeat(&self, mode: RepeatMode);
}
// mpris.rs
pub fn metadata(track: &Track, art: Option<&Path>) -> Metadata; // trackid, title, artist[], album, albumArtist, length (µs), artUrl (file://)
pub fn playback_status(state: PlaybackState) -> PlaybackStatus;  // Playing / Paused / Stopped (Idle|Ended → Stopped, Loading → Playing)
pub fn loop_status(mode: RepeatMode) -> LoopStatus;  pub fn repeat_mode(status: LoopStatus) -> RepeatMode;
pub struct MprisHooks { pub raise: Box<dyn Fn() + Send + Sync>, pub quit: Box<dyn Fn() + Send + Sync> }
pub fn spawn(control: Arc<dyn PlayerControl>, events: Receiver<Event>, hooks: MprisHooks); // own thread + executor; logs and returns if the bus is unavailable
```

**Behaviour:**
- Root interface:
  - `CanRaise` and `CanQuit` are true;
  - `Raise` and `Quit` call the hooks.
- Player interface:
  - `CanPlay`, `CanPause` and `CanSeek` are true when there is a track;
  - `CanGoNext` and `CanGoPrevious` are true when there is a track.
- Events:
  - `PlayerStateChanged` and `QueueChanged` → `properties_changed` (PlaybackStatus, Metadata, LoopStatus, Shuffle, Volume);
  - a seek requested over MPRIS → `Seeked`.

- [ ] **Step 1: Write the failing tests** (unit tests in `mpris.rs` and `control.rs`, with a `FakeControl`):

```rust
#[test] fn metadata_maps_a_tagged_track()            // title, artist ["Ana"], album, length 200_000_000 µs, trackid ends "/Track/7", artUrl "file:///…/7-600.webp"
#[test] fn metadata_for_an_untagged_track_is_minimal() // album None, artists [] → no album/artist keys, no artUrl, title kept
#[test] fn states_map_to_mpris()                      // Idle→Stopped, Loading→Playing, Playing→Playing, Paused→Paused, Ended→Stopped
#[test] fn loop_status_round_trips()                  // off/all/one ↔ None/Playlist/Track
#[test] fn mpris_unavailable_is_logged_not_fatal()    // spawn with DBUS_SESSION_BUS_ADDRESS pointing nowhere → returns, no panic
// tests/mpris_bus.rs (#[ignore], dbus-run-session): FakeControl playing track 7 → a zbus proxy reads
// PlaybackStatus "Playing" and Metadata title; calls PlayPause → FakeControl recorded play_pause; Next → next
```

- [ ] **Step 2: Run.** Expected: FAIL (modules do not exist).
- [ ] **Step 3: Implement.** `PlayerControl` for `Services`: each method maps to the existing call; `art_path` → `album_art(album, ArtSize::Large).ok().flatten()`. In `run()`, `mpris::spawn(services, services.events(), hooks)`: `raise` shows and focuses `main`, `quit` calls `app.exit(0)`.
- [ ] **Step 4: Run** the gate and `dbus-run-session -- cargo test -p goymusic-desktop -- --ignored`. Expected: PASS.
- [ ] **Step 5: Commit** «Escritorio: MPRIS (teclas multimedia, GNOME y playerctl)»

### Task 2: Notificación al cambiar de canción

**Files:**
- Create: `apps/desktop/src/notify.rs`
- Modify: `apps/desktop/src/lib.rs`

**Interfaces (Produces):**

```rust
pub trait Notifier: Send + 'static { fn show(&mut self, summary: &str, body: &str, image: Option<&Path>) -> Result<(), String>; }
pub struct DesktopNotifier { last: Option<u32> }   // notify-rust; reuses the id so each new one replaces the previous
pub fn text_for(track: &Track) -> (String, String); // (title, "Artista — Álbum" | "Artista" | "")
pub struct TrackNotifications<N: Notifier> { … }
impl<N: Notifier> TrackNotifications<N> {
    pub fn new(notifier: N, control: Arc<dyn PlayerControl>, focused: Arc<dyn Fn() -> bool + Send + Sync>) -> Self;
    pub fn on_event(&mut self, event: &Event); // notifies when state=Playing and the track differs from the last one notified, unless the main window is focused
}
```

- The hint is `category = "x-gnome.music"`.
- The icon:
  - is the cover if there is one;
  - otherwise the app icon (`goymusic` in the theme; `icons/128x128.png` next to the binary in dev).

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn text_for_tagged_and_untagged_tracks()           // ("Faro", "Ana — Cartografía") / ("Grabación 1", "")
#[test] fn only_the_track_that_starts_playing_is_notified() // Loading(7), Playing(7), Paused(7), Playing(7), Playing(8) → notified [7, 8]
#[test] fn nothing_is_notified_while_the_window_is_focused()
#[test] fn a_failing_notifier_does_not_stop_the_event_loop() // Notifier returns Err → on_event returns normally; the next track is still attempted
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** The same event thread as MPRIS (or its own thread with its own `subscribe()`) calls `on_event`. `focused` reads `main.is_focused()` (false on error).
- [ ] **Step 4: Run** the gate. Expected: PASS.
- [ ] **Step 5: Commit** «Escritorio: notificación con carátula al cambiar de canción»

### Task 3: Bandeja, cerrar sin cortar la música e instancia única

**Files:**
- Create: `apps/desktop/src/tray.rs`, `apps/desktop/src/window.rs`
- Modify:
  - `apps/desktop/src/lib.rs`: `Services::start` moves into `setup`, after the plugins; the `art://` protocol takes the services from `app.state()`;
  - `Cargo.toml` (features of `tauri`, plugin).

**Interfaces (Produces):**

```rust
// tray.rs
pub fn menu_labels(view: Option<&PlayerView>) -> Vec<(&'static str, String, bool)>;
// (id, label, enabled): play_pause "Reproducir"/"Pausa", previous "Anterior", next "Siguiente",
// show "Mostrar GoyMusic", mini "Mini reproductor", quit "Salir"; transport disabled with no track
pub fn create(app: &AppHandle, control: Arc<dyn PlayerControl>) -> Result<(), String>; // icon + menu; left click shows the window
// window.rs
pub enum CloseAction { Hide, Quit }
pub fn on_close(tray_available: bool) -> CloseAction;
```

**Behaviour:**
- `tauri_plugin_single_instance::init(|app, _, _| show main)` is the **first** plugin.
- `Services::start` is called inside `.setup`. On Linux the plugin resolves the second instance before that, so a second instance never opens the database.
- If `tray::create` fails, it is logged and `tray_available = false`.
- `WindowEvent::CloseRequested` on `main`: if `Hide`, `api.prevent_close()` + `window.hide()`.
- «Salir» → `app.exit(0)` → `RunEvent::Exit` → `services.shutdown()` (already idempotent).
- The tray menu updates its labels on `PlayerStateChanged`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn menu_labels_follow_the_player()               // playing → "Pausa"; paused → "Reproducir"; no track → transport disabled
#[test] fn closing_hides_only_when_the_tray_exists()     // on_close(true) == Hide; on_close(false) == Quit
#[test] fn services_start_after_the_single_instance_check() // structural: run() registers the plugin before any Services::start
                                                            // (assert on a small `startup_order()` that returns the steps in order)
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the gate. Expected: PASS. Manual check in dev: close the window → the music goes on; run `pnpm dev` again → the existing window comes forward.
- [ ] **Step 5: Commit** «Escritorio: bandeja, cerrar sin cortar la música e instancia única»

### Task 4: Mini-reproductor

**Files:**
- Modify:
  - `apps/desktop/tauri.conf.json`: `mini` window, 360×120, `visible: false`, no decorations, not resizable, `skipTaskbar: false`;
  - `apps/desktop/capabilities/default.json`: windows `["main", "mini"]`;
  - `apps/desktop/src/commands.rs`: `show_mini_player()`, `hide_mini_player()`, `set_mini_on_top(on: bool)`, `show_main_window()`;
  - the regenerated bindings;
  - `ui/src/main.tsx`: the `mini` window renders `<MiniPlayer/>` instead of the shell;
  - `PlayerBar` (button «Mini reproductor»).
- Create: `ui/src/mini/MiniPlayer.tsx` + `.module.css` + test

**MiniPlayer:**
- Contents:
  - generated cover or art (64 px) and the title / artist;
  - previous / play-pause / next and the filament (progress);
  - «Fijar encima» (`aria-pressed`, `set_mini_on_top`), «Abrir GoyMusic» (`show_main_window`) and «Cerrar» (`hide_mini_player`).
- Dragging the area moves the window (`data-tauri-drag-region`).
- Tokens from `design.md`; its background is the bloom of the track playing (`hueStyle`).

- [ ] **Step 1: Write the failing tests** (UI)

```ts
test("the mini player shows the track and toggles playback")
test("pin on top toggles and calls the core")  // setMiniOnTop(true) then (false); aria-pressed follows
test("open goes back to the main window")       // showMainWindow called
test("the player bar opens the mini player")    // showMiniPlayer called
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement** (the new commands use `app.get_webview_window("mini")`). Regenerate the bindings with `UPDATE_BINDINGS=1 cargo test -p goymusic-desktop --test bindings`.
- [ ] **Step 4: Run** the gate. Expected: PASS. Visual check in mock mode (port 1430, `#/` with the window simulated: `?ventana=mini`).
- [ ] **Step 5: Commit** «Escritorio: mini reproductor con opción de quedar siempre encima»

### Task 5: Empaquetado AppImage y .deb, CI

**Files:**
- Modify:
  - `apps/desktop/tauri.conf.json`: `bundle.active: true`, `targets: ["deb", "appimage"]`, `category: "Audio"`, `shortDescription`, `longDescription`, `linux.deb.depends`: `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1`, `libasound2t64 | libasound2`, `publisher`, `license: "GPL-3.0-or-later"`;
  - `.github/workflows/ci.yml`: install `dbus` and run the ignored D-Bus tests under `dbus-run-session`; a `bundle` job (only on push to `master`) runs `pnpm tauri build --bundles deb appimage` and uploads both as artifacts;
  - `apps/desktop/src/mpris.rs`: `DesktopEntry` = the real name of the generated `.desktop`.

**Steps:**
- [ ] **Step 1:** Run `source ~/.nvm/nvm.sh && pnpm tauri build --bundles deb appimage`. Expected: `target/release/bundle/deb/*.deb` and `target/release/bundle/appimage/*.AppImage`.
- [ ] **Step 2:** Inspect the `.deb` (`dpkg-deb -c` / `-I`): binary, `.desktop` (name → `DesktopEntry`), icons and dependencies. Fix and rebuild if something is missing.
- [ ] **Step 3:** Check the AppImage: `chmod +x`, start it with `--version` or start it and close it, plus `ldd`-like checks if it fails.
- [ ] **Step 4:** Run the full gate. Commit «Empaquetado: AppImage y .deb; CI con pruebas de D-Bus y paquetes».
- [ ] **Step 5: Mandatory stop.** Javi:
  1. installs the `.deb` in his terminal (`sudo apt install ./…deb`) or runs the AppImage;
  2. checks the multimedia keys, the GNOME panel, `playerctl status`, the notification, closing the window with music playing, the tray, opening it a second time and the mini player.

  Final review and merge afterwards.
