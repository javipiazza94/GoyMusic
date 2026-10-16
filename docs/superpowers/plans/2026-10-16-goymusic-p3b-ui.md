# GoyMusic P3b — Interfaz definitiva con Hallmark: plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Sustituir el shell de desarrollo por la interfaz de la §7: barra lateral, vistas con rutas, barra de reproducción, panel de cola, listas virtualizadas, menú contextual, arrastrar y soltar, atajos, avisos y temas oscuro y claro, con una identidad visual propia definida con Hallmark.

**Architecture:** Solo cambia `ui/`, salvo una línea de `tauri.conf.json` (Task 7) y los iconos de la app (Task 10). Los datos siguen la capa de P3a: TanStack Query invalidado por eventos y Zustand para el reproductor. Se añaden:
- **React Router** con `HashRouter`, porque Tauri no reescribe rutas profundas a `index.html` al recargar;
- **TanStack Virtual** para las listas;
- **arrastrar y soltar HTML5 nativo**, sin librería;
- **CSS Modules** sobre un fichero de tokens.

Un modo `mock` de Vite sustituye el IPC de Tauri por datos inventados (`mockIPC` de `@tauri-apps/api/mocks`). Con él la UI se ve y se verifica en un navegador sin la app nativa.

**Tech Stack:** React 19.3, Vite 8.3, TypeScript 7.0, Vitest 5.0, TanStack Query 5.104, Zustand 5.0, React Router 8.4, TanStack Virtual 3.14, fuentes Geist, Geist Mono e Instrument Serif empaquetadas con Fontsource.

**Spec:** `docs/superpowers/specs/2026-10-16-goymusic-design.md`. Secciones: §7 completa, §9 (avisos y panel de errores), §10 criterios 4 y 5 en su parte de UI.

**Hoja de ruta:** después viene **P4** (MPRIS, notificaciones, bandeja, instancia única, mini-reproductor y empaquetado).

## Sistema visual (Hallmark)

Decidido con Javi el 2026-10-16: tono **atmosférico** y tema del catálogo elegido por Hallmark.

- **Género:** atmospheric. **Tema:** Lumen, con sus dos variantes:
  - *Night Foundry*: oscuro, es el predeterminado;
  - *Day Foundry*: claro.
- **Por qué Lumen:** es el único tema atmosférico con variante clara y oscura, y la spec pide las dos. El violeta nocturno con latón no se parece al negro con verde de Spotify.
- **Adaptación de app (rulings):**
  - Las macroestructuras de Hallmark son para páginas de aterrizaje. Aquí la estructura la fija la §7: barra lateral (N3 side-rail), vista principal y barra de reproducción en lugar de pie.
  - La regla de Lumen de «todo en minúsculas» **no** se aplica: los metadatos del usuario conservan sus mayúsculas, y la UI usa mayúscula inicial en español.
  - La verificación responsive se hace a 900 px (ancho mínimo de la ventana) y 1280 px, no a 320–768.
- **Tipografía (2+1):**
  - **Instrument Serif**, siempre en redonda (nunca cursiva), solo para títulos de vista (álbum, artista, playlist, «Me gusta», «Biblioteca») y la marca;
  - **Geist** para toda la interfaz;
  - **Geist Mono** para números (duraciones, n.º de pista, contadores) y para las micro-etiquetas en MAYÚSCULAS, por ejemplo `ÁLBUM · 1997 · 12 PISTAS · 48 MIN`.
- **Firma visual:**
  1. La barra de progreso es un «filamento»: un trazo fino de latón cuya parte reproducida brilla (`--color-glow`) en Night y no brilla en Day.
  2. Un único resplandor radial fijo de acento, con alfa ≤ 0,10, detrás de la vista principal.
  3. Superficies elevadas (`paper-2`/`paper-3`) en lugar de líneas finas. Sin glassmorphism y sin texto con degradado.
- **Movimiento:** solo fundidos (opacity). `--dur-fast 120ms`, `--dur-base 200ms`, más las easings `--ease-out`, `--ease-in` y `--ease-in-out`. Con `prefers-reduced-motion` desaparecen las transiciones. El anillo de foco aparece sin animación.

**Tokens** (`ui/src/styles/tokens.css`). Night en `:root`; Day en `:root[data-theme="light"]`.

| Token | Night | Day |
|---|---|---|
| `--color-paper` | `oklch(13% 0.014 265)` | `oklch(97% 0.008 265)` |
| `--color-paper-2` | `oklch(17% 0.016 265)` | `oklch(94% 0.010 265)` |
| `--color-paper-3` | `oklch(22% 0.018 265)` | `oklch(90% 0.012 265)` |
| `--color-ink` | `oklch(96% 0.006 262)` | `oklch(18% 0.014 265)` |
| `--color-ink-2` (secundario) | `oklch(76% 0.012 262)` | `oklch(38% 0.014 265)` |
| `--color-ink-3` (números, pistas) | `oklch(64% 0.012 262)` | `oklch(48% 0.014 265)` |
| `--color-rule` | `oklch(96% 0.006 262 / 0.08)` | `oklch(18% 0.014 265 / 0.10)` |
| `--color-accent` | `oklch(76% 0.17 50)` latón | `oklch(46% 0.24 268)` índigo |
| `--color-accent-ink` (texto sobre acento) | `oklch(18% 0.03 50)` | `oklch(98% 0.01 268)` |
| `--color-accent-2` (Me gusta, peligro) | `oklch(68% 0.16 18)` | `oklch(52% 0.19 18)` |
| `--color-glow` | `oklch(80% 0.16 50 / 0.42)` | `transparent` |
| `--color-bloom` | `oklch(76% 0.17 50 / 0.08)` | `oklch(46% 0.24 268 / 0.06)` |
| `--color-focus` | `var(--color-accent)` | `var(--color-accent)` |

Además:
- **Fuentes:** `--font-display: "Instrument Serif", Georgia, serif`; `--font-body: "Geist Variable", system-ui, sans-serif`; `--font-mono: "Geist Mono Variable", ui-monospace, monospace`.
- **Escala tipográfica:** `--text-xs .75rem`, `--text-sm .8125rem`, `--text-base .875rem`, `--text-md 1rem`, `--text-lg 1.25rem`, `--text-xl 1.75rem`, `--text-display clamp(2.5rem, 3vw + 1.5rem, 4.5rem)`.
- **Espaciado 4 pt:** `--space-2xs 4px`, `xs 8`, `sm 12`, `md 16`, `lg 24`, `xl 32`, `2xl 48`, `3xl 64`.
- **Radios:** `--radius-sm 4px`, `--radius-md 8px`, `--radius-pill 999px`.
- **Disposición:** `--sidebar-w 240px`, `--queue-w 320px`, `--player-h 84px`, `--row-h 52px`.

**Disciplinas de Hallmark que aplican:**
- Ningún color ni `font-family` fuera de tokens.
- Cada elemento interactivo con sus 8 estados.
- Sin datos inventados en la app real: el modo `mock` es solo de desarrollo y lo indica en pantalla.
- Encabezados sin cursiva.
- El sello `/* Hallmark · genre: atmospheric · theme: Lumen (Night/Day) · nav: N3 side-rail · footer: player bar */` va en la primera línea de `tokens.css`.
- `.hallmark/log.json` se crea en la Task 1.

## Global Constraints

**Dependencias**
- npm, versiones exactas sin `^`, todas publicadas hace más de dos semanas:
  - `react-router 8.4.0`
  - `@tanstack/react-virtual 3.14.13`
  - `@fontsource-variable/geist 5.3.0`
  - `@fontsource-variable/geist-mono 5.3.0`
  - `@fontsource/instrument-serif 5.3.0`
  - dev: `@testing-library/user-event 14.6.7`
- No hay más dependencias: iconos, menús, arrastrar y soltar y avisos se construyen a mano.
- Si una API no cuadra, consultar la documentación de esa versión (Context7 o el paquete instalado); no cambiar la versión.

**Producto**
- Textos de la UI en español; código en inglés.
- Los atajos son los de la spec:
  - espacio: reproducir o pausar;
  - `Ctrl+F`: buscar;
  - `Ctrl+←` / `Ctrl+→`: anterior / siguiente.
- Se ignoran cuando el foco está en un `input`, `textarea`, `select`, `[contenteditable]` o, para el espacio, en un `button`.
- Rutas exactas de la §7: `/`, `/album/:id`, `/artist/:id`, `/playlist/:id`, `/collection/tracks`, `/search?q=` y `/settings`, sobre `HashRouter`.
- Eventos: la UI nunca hace polling. Cada mutación se refleja por el evento del núcleo correspondiente (`playlistsChanged`, `likesChanged`, `queueChanged`, …), no por un refetch manual.
- Borrar una playlist pide confirmación en línea, porque no se puede deshacer. Nunca se usa `window.confirm`.
- Ninguna vista depende de que las pistas tengan álbum o artista: con `albumId === null` no hay carátula (se muestra un marcador), y «Ir al álbum» queda deshabilitado.

**Proceso**
- Al cerrar cada tarea deben pasar:
  ```
  source ~/.nvm/nvm.sh && pnpm --dir ui test && pnpm --dir ui build
  ```
  En las Tasks 7 y 10, además, el gate completo de Rust: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
- Tests con Vitest + Testing Library + `user-event`. Los bindings se simulan **globalmente** en `ui/src/test/setup.ts` (Task 1); cada test ajusta solo lo que necesita.
- Commits en español; nada de código, nombres ni recursos de Spotify. El modo `mock` usa artistas y álbumes inventados.
- Flujo de commits: tras cada tarea, `git reset --soft <base de la tarea>` y un único commit descriptivo.

## Review Focus

1. **Biblioteca grande (50 000 pistas) en «Canciones»:** se pagina de 500 en 500 y solo se pintan las filas visibles; abrir la vista no carga todo. Test en la Task 4 (`renders only the visible rows of a long list`).
2. **Pistas sin etiquetas de álbum o artista** (el caso real de Javi): las filas, la barra de reproducción, el menú contextual y la cola se ven sin errores, y «Ir al álbum» está deshabilitado. Tests en las Tasks 3, 4 y 7.
3. **Atajos mientras se escribe:** escribir un espacio en la búsqueda no pausa la música, y `Ctrl+←` dentro de un input no cambia de pista. Test en la Task 9.
4. **Ruta a algo que ya no existe** (playlist borrada con su vista abierta, álbum eliminado por un reescaneo): la vista muestra «No encontrado» con un enlace a Inicio, nunca una pantalla en blanco. Al borrar la playlist abierta se navega a `/`. Tests en las Tasks 5 y 6.
5. **Tema del sistema cambiando con la app abierta:** en modo «Sistema», un cambio de `prefers-color-scheme` se aplica sin recargar. Test en la Task 1.

---

### Task 1: Fundamentos — dependencias, tokens, tema, iconos, infraestructura de tests y modo mock

**Files:**
- Modify: `ui/package.json` (deps; script `dev:mock`: `vite --mode mock`), `ui/src/main.tsx`, `ui/src/test/setup.ts`, `ui/vite.config.ts` (si hace falta para CSS Modules en Vitest: `css: { modules: { classNameStrategy: "non-scoped" } }` en `test`)
- Create:
  - `ui/src/styles/tokens.css`, `ui/src/styles/base.css`
  - `ui/src/core/theme.ts` + `theme.test.ts`
  - `ui/src/components/Icon.tsx`
  - `ui/src/test/mock-bindings.ts`, `ui/src/test/render.tsx`
  - `ui/src/dev/mock-ipc.ts`
  - `.hallmark/log.json`
- Migrate: `ui/src/App.test.tsx` and `ui/src/core/*.test.ts` to the global mock.

**Interfaces (Produces):**

```ts
// core/theme.ts
export type ThemePref = "system" | "dark" | "light";
export function useTheme(): { pref: ThemePref; resolved: "dark" | "light"; setPref(p: ThemePref): void };
export function initTheme(): void; // reads localStorage "goymusic.theme" (default "system"), sets <html data-theme>, follows matchMedia changes
// components/Icon.tsx: hand-drawn SVG, viewBox 0 0 24 24, stroke 1.75, currentColor, aria-hidden
export type IconName = "play"|"pause"|"next"|"previous"|"shuffle"|"repeat"|"repeatOne"|"heart"|"heartFilled"|"queue"
  |"volume"|"volumeMuted"|"search"|"home"|"settings"|"plus"|"folder"|"more"|"close"|"music";
export function Icon(props: { name: IconName; size?: number }): JSX.Element;
// test/mock-bindings.ts: one vi.fn per command in bindings.ts, all resolving ok(...)
// (list commands → [], playerView → view() fixture, settings → {replaygain:false, watch:{mode:"live"}}, others → null)
export const commands: Record<keyof typeof import("../bindings").commands, Mock>;
export const events: { coreEvent: { listen: Mock } };
export function resetCommandMocks(): void; // restores the defaults; called in beforeEach of setup.ts
// test/render.tsx
export function renderAt(path: string, ui?: ReactNode): RenderResult & { user: UserEvent; qc: QueryClient };
// wraps QueryClientProvider (retry:false) + MemoryRouter(initialEntries=[path]) + <AppRoutes/> when ui is omitted
```

`setup.ts` does:
- `vi.mock("../bindings", () => import("./mock-bindings"))` and `vi.mock("@tauri-apps/plugin-dialog", ...)`;
- stubs `ResizeObserver` and `matchMedia`;
- in `beforeEach`: `resetCommandMocks()`, `localStorage.clear()` and resets `usePlayer`.

`dev/mock-ipc.ts` uses `mockIPC` from `@tauri-apps/api/mocks`:
- it answers each command with invented data (3 artists, 4 albums, 40 tracks, 2 playlists, 1 folder);
- it keeps an in-memory player that responds to play/pause/next.

`main.tsx` imports it only when `import.meta.env.MODE === "mock"`, and in that mode shows a fixed «MODO MOCK» label (mono, small).

`base.css`:
- `@import`s the 3 Fontsource packages, with `font-display: swap`;
- reset, `html, body { overflow-x: clip; background: var(--color-paper); color: var(--color-ink); font: var(--text-base)/1.45 var(--font-body) }`;
- `:focus-visible { outline: 2px solid var(--color-focus); outline-offset: 2px }`;
- `@media (prefers-reduced-motion: reduce) { *, ::before, ::after { transition-duration: 0ms !important; animation: none !important } }`;
- thin scrollbars on `--color-paper-3`.

- [ ] **Step 1: Write the failing tests**

```ts
// theme.test.ts
test("defaults to the system preference")            // matchMedia dark → initTheme() → html.dataset.theme === "dark"
test("an explicit choice is applied and remembered") // setPref("light") → dataset "light", localStorage "goymusic.theme" === "light"
test("follows a system change while on system")      // pref system, matchMedia listener fires light → dataset "light"
test("ignores system changes when the choice is explicit")
```

- [ ] **Step 2: Run** `source ~/.nvm/nvm.sh && pnpm --dir ui test`. Expected: FAIL (`theme` does not exist). The migrated tests still pass.
- [ ] **Step 3: Install the dependencies** (exact versions above) **and implement** the Interfaces, the tokens from the table, `base.css`, the mock and the log. The `.hallmark/log.json` entry:
  `{ "date": "2026-10-16", "macrostructure": "app shell (§7)", "theme": "Lumen", "enrichment": "none", "brief": "GoyMusic · reproductor de música local" }`.
- [ ] **Step 4: Run** the task gate. Expected: PASS; `pnpm --dir ui build` with no type errors.
- [ ] **Step 5: Commit** «UI: sistema visual Lumen (tokens, fuentes, tema claro/oscuro), iconos e infraestructura de tests y modo mock»

### Task 2: Estructura de la app — disposición, rutas y barra lateral

**Files:**
- Create:
  - `ui/src/app/AppShell.tsx` + `.module.css`, `ui/src/app/routes.tsx`
  - `ui/src/app/Sidebar.tsx` + `.module.css`
  - `ui/src/app/ui-store.ts` (Zustand: `queueOpen`, persisted in localStorage)
  - `ui/src/views/NotFound.tsx`
  - tests `ui/src/app/AppShell.test.tsx`, `ui/src/app/Sidebar.test.tsx`
- Modify: `ui/src/main.tsx` (`HashRouter` + `AppShell`), `ui/src/core/queries.ts` (`usePlaylists`, `usePlaylistFolders`)
- Delete: `ui/src/App.tsx`, `ui/src/App.test.tsx`. This is the development shell: P3a's plan already marks it as replaced by P3b. Its coverage (folders, play) moves to the tests of Tasks 4–5.

**Interfaces:**
- Consumes: `renderAt`, `Icon`, `keys`.
- Produces:
  - `AppRoutes()`: the 7 routes plus `*` → `NotFound`. Views that don't exist yet render a `<h1>` with their name; Tasks 5, 6 and 8 replace them.
  - `AppShell`: CSS grid `"sidebar main queue" "player player player"`. The `queue` column exists only when `queueOpen`. `<main>` has its own scroll and the bloom (`radial-gradient` with `--color-bloom`, fixed).
  - `useUi = create<{ queueOpen: boolean; toggleQueue(): void }>`.

**Sidebar:**
- Wordmark «GoyMusic» in `--font-display`.
- `NavLink`s to Inicio (`/`), Buscar (`/search`) and Me gusta (`/collection/tracks`), with `aria-current` on the active one.
- «PLAYLISTS» section (mono micro-label) with buttons «Nueva playlist» (creates «Nueva playlist» and navigates to it), «Nueva carpeta» and «Importar M3U8».
- Tree: folders (`<details>`, open by default) with their playlists, then loose playlists. The tree has its own scroll.
- Ajustes (`/settings`) at the bottom.

- [ ] **Step 1: Write the failing tests**

```ts
test("renders the view for each route")              // "/settings" → heading "Ajustes"; "/nope" → "No encontrado" + link "Ir a Inicio"
test("marks the current section in the sidebar")     // at "/collection/tracks" the "Me gusta" link has aria-current="page"
test("groups playlists under their folders")         // folders [{id:1,name:"Ánimo"}], playlists one with folderId 1 and one loose → structure by role "group"/"list"
test("creates a playlist and opens it")              // click "Nueva playlist" → createPlaylist("Nueva playlist") → route /playlist/<id>
test("the queue column toggles and is remembered")   // toggleQueue → localStorage "goymusic.queueOpen" === "true"
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** «Importar M3U8» uses `open({ filters: [{ name: "M3U8", extensions: ["m3u8","m3u"] }] })` → `importM3u(path)` → navigates to the new playlist. If `missing.length > 0`, it raises a notice through `reportError`/`notify` (Task 9; until then, `console.warn`).
- [ ] **Step 4: Run** the gate. Expected: PASS.
- [ ] **Step 5: Commit** «UI: disposición de la app, rutas de la §7 y barra lateral con playlists y carpetas»

### Task 3: Barra de reproducción

**Files:**
- Create: `ui/src/player/PlayerBar.tsx` + `.module.css`, `ui/src/player/SeekBar.tsx`, `ui/src/player/VolumeControl.tsx`, `ui/src/core/likes.ts`, `ui/src/core/artist-index.ts`, test `ui/src/player/PlayerBar.test.tsx`
- Modify: `AppShell` (mounts `PlayerBar` in the `player` area)

**Interfaces (Produces):**

```ts
// core/likes.ts
export function useLikedIds(): Set<number>;          // query keys.likes → likedIds()
export function useToggleLike(): (id: number, liked: boolean) => Promise<void>; // like/unlike; the cache updates through likesChanged
// core/artist-index.ts: Track.artists are names; artist links resolve the id from artists()
export function useArtistId(name: string | null): number | null; // query ["library","artists"], case-insensitive match
```

**PlayerBar:**
- **Left:**
  - cover at 56 px (`artUrl(albumId, 200)`, or the `music` icon on `--color-paper-3` when `albumId === null`);
  - the title, linked to `/album/:id` only if there is an album;
  - the artists, each linked to `/artist/:id` only if `useArtistId` resolves it;
  - the like heart, with `aria-pressed`.
- **Centre:**
  - shuffle (`aria-pressed`), previous, play/pause, next, repeat;
  - play/pause is an accent pill with `aria-label` «Reproducir»/«Pausa»;
  - repeat cycles off → all → one, with `aria-label` «Repetir: no» / «Repetir: todo» / «Repetir: una»;
  - below them: `formatMs(positionMs)`, then `SeekBar`, then the total, in mono.
- **`SeekBar`:** `<input type="range">` with a value in ms and the filament style.
  - While dragging it shows a local value.
  - `seek(ms)` is called only on `pointerup` or `keyup` (one seek per gesture).
- **Right:**
  - queue button (`aria-pressed={queueOpen}`);
  - volume (`setVolume`, 0–1, step 0.01);
  - mute, which remembers the previous volume and sets 0.
- **Without a `view.track`:** controls disabled and the empty text «Elige algo que escuchar».

- [ ] **Step 1: Write the failing tests**

```ts
test("shows the current track and toggles playback") // view playing → "Pausa" → click → pause()
test("seeks once when the drag ends")                // change ×3 + pointerUp → seek called once with the last value
test("cycles repeat off → all → one → off")          // setRepeat("all"), then "one", then "off"
test("likes the current track")                      // likedIds [] → click heart → like(1)
test("a track without album has no album link and shows a placeholder cover") // track albumId null → no link to /album, img absent
test("disables the controls when nothing is loaded")
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the gate. Expected: PASS.
- [ ] **Step 5: Commit** «UI: barra de reproducción con progreso, volumen, aleatorio, repetir y Me gusta»

### Task 4: Lista de pistas virtualizada

**Files:**
- Create: `ui/src/tracks/TrackList.tsx` + `.module.css`, `ui/src/tracks/selection.ts`, tests `ui/src/tracks/TrackList.test.tsx`, `ui/src/tracks/selection.test.ts`

**Interfaces (Produces):**

```ts
export type TrackRow = { track: Track; key: string; position?: number }; // position: playlists (removal/reordering)
export interface TrackListProps {
  rows: TrackRow[];
  columns?: Array<"number" | "album" | "added">;   // "number" = trackNo (album) or index; "album" by default
  onPlay(index: number): void;                       // double-click or Enter on a row
  onEndReached?(): void;                             // last 20 rows visible → load more
  menuExtra?: (rows: TrackRow[]) => MenuItem[];      // Task 7
  reorderable?: boolean;                             // Task 7
  label: string;                                     // aria-label of the grid
}
export function TrackList(p: TrackListProps): JSX.Element;
// selection.ts (pure): click = only that one, Ctrl = toggle, Shift = range from the anchor
export function nextSelection(sel: Selection, index: number, mods: { ctrl: boolean; shift: boolean }): Selection;
export type Selection = { keys: Set<number>; anchor: number | null };
```

**Behaviour:**
- `role="grid"` with rows of height `--row-h`.
- `useVirtualizer` with `overscan: 10` and `initialRect: { width: 1024, height: 768 }`, so that jsdom renders the first rows.
- Columns: n.º (mono), title + artists (`--color-ink-2`), album (link if `albumId`), like, duration (mono).
- The current track (`view.track.id`) is highlighted in accent with an `aria-current` mark.
- Arrow keys move the focus between rows; Enter plays.

- [ ] **Step 1: Write the failing tests**

```ts
// selection.test.ts
test("a plain click selects only that row")
test("ctrl toggles and shift selects a range from the anchor")
// TrackList.test.tsx
test("renders only the visible rows of a long list")  // 50_000 rows → rendered rows < 100
test("double click plays from that row")              // dblclick row 3 → onPlay(3)
test("enter plays the focused row and arrows move focus")
test("marks the track that is playing")               // view.track.id === rows[1].track.id → that row aria-current="true"
test("asks for more rows near the end")               // scroll to the end → onEndReached called
test("rows without album or artists still render")    // album null, artists [] → title visible, no album link
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the gate. Expected: PASS.
- [ ] **Step 5: Commit** «UI: lista de pistas virtualizada con selección, teclado y pista actual»

### Task 5: Vistas de biblioteca y Ajustes

**Files:**
- Create:
  - `ui/src/views/Home.tsx`, `ui/src/views/AlbumView.tsx`, `ui/src/views/ArtistView.tsx`, `ui/src/views/SettingsView.tsx`
  - `ui/src/views/ViewHeader.tsx` (cover/initials + display title + mono meta + actions) and `ui/src/views/views.module.css`
  - `ui/src/library/AlbumGrid.tsx`, `ui/src/library/Cover.tsx`
  - tests `ui/src/views/Home.test.tsx`, `AlbumView.test.tsx`, `ArtistView.test.tsx`, `SettingsView.test.tsx`
- Modify: `ui/src/core/queries.ts`. `useTracks` becomes `useTrackPages(sort)` with `useInfiniteQuery` (pages of 500; ends when a page has fewer than 500). Adds `useAlbum`, `useArtist`, `useArtists`, `useRecentAlbums(12)`.

**Interfaces (Produces):**

```ts
export function ViewHeader(p: { kind: string; title: string; meta: string[]; cover?: { albumId: number | null };
  actions?: ReactNode }): JSX.Element;            // kind = mono micro-label ("ÁLBUM", "ARTISTA", "PLAYLIST")
export function Cover(p: { albumId: number | null; size: 200 | 600; label: string }): JSX.Element; // initials placeholder if null or the image fails
export function AlbumGrid(p: { albums: Album[] }): JSX.Element; // grid minmax(160px,1fr), cover + title + artist · year
export function totalDuration(tracks: Track[]): string;          // "48 min" / "1 h 12 min"
```

**Views:**
- **Home `/`:**
  - title «Biblioteca»;
  - a «Escuchado hace poco» row (if `recentAlbums` has items);
  - radio tabs «Canciones / Álbumes / Artistas» (`role="tablist"`, no scroll jump);
  - the active tab is remembered in localStorage `goymusic.homeTab`; by default it is «Álbumes» if there are albums, otherwise «Canciones»;
  - «Canciones» has a sort `select` (Título, Artista, Álbum, Añadidas recientemente) and a `TrackList` with `onEndReached → fetchNextPage`; play is `{kind:"tracks", id: <loaded ids>}` from the index;
  - with no folders: an empty state «Tu biblioteca está vacía» with an «Añadir carpeta» button (same dialog as Ajustes).
- **Album `/album/:id`:**
  - `ViewHeader` with a 600 px cover and the meta `[artist, year, "N pistas", duration]`;
  - «Reproducir» → `play({kind:"album",id},0)`;
  - TrackList with `columns ["number"]`, which plays the album from the index;
  - `notFound` → `NotFound`.
- **Artist `/artist/:id`:**
  - header without a cover (initials) and «Reproducir» (artist context);
  - «Álbumes» section with an `AlbumGrid`;
  - «Canciones» section with a `TrackList` that plays the artist context from the index.
- **Ajustes `/settings`:**
  - **Carpetas:** list with «Quitar {ruta}», «Añadir carpeta», «Reescanear» and progress «Escaneando… done/total»;
  - **Reproducción:** a ReplayGain switch (`setReplaygain`) on `role="switch"`;
  - **Apariencia:** radio Sistema/Oscuro/Claro bound to `useTheme`;
  - **Vigilancia:** «En vivo», or «Comprobando cada 10 min: {reason}».

- [ ] **Step 1: Write the failing tests**

```ts
test("home shows albums and plays one from its page")          // migrated from the dev shell's App.test
test("home falls back to songs when there are no albums")      // albums [] + tracks [2] → "Canciones" tab selected, plays {kind:"tracks", id:[5,9]} from 1
test("home invites to add a folder when the library is empty") // folders [] → "Añadir carpeta" → open() → addFolder(path)
test("album view plays the album from the clicked track")      // dblclick row 2 → play({kind:"album",id:7}, 2)
test("an album that no longer exists shows not found")         // album() → error notFound → "No encontrado"
test("artist view lists albums and songs")
test("settings removes a folder and switches theme")           // "Quitar /home/javi/Música" → removeFolder(1); "Claro" → html data-theme "light"
test("settings shows when folders are only polled")            // watch {mode:"polling", reason:"inotify"} → text contains "cada 10 min"
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the gate. Expected: PASS.
- [ ] **Step 5: Commit** «UI: vistas de biblioteca (inicio, álbum, artista) y ajustes»

### Task 6: Playlists y Me gusta

**Files:**
- Create: `ui/src/views/PlaylistView.tsx`, `ui/src/views/LikedView.tsx`, `ui/src/views/ConfirmInline.tsx`, tests `PlaylistView.test.tsx`, `LikedView.test.tsx`
- Modify: `queries.ts` (`usePlaylist`, `useLikedTracks` under the `keys.likes` prefix)

**Behaviour:**
- **`/playlist/:id`:**
  - `ViewHeader` (kind «PLAYLIST», meta `["N canciones", duration]`).
  - Actions:
    - «Reproducir» → `play({kind:"playlist",id}, 0)`;
    - «Renombrar»: the title becomes an `input`; Enter or blur saves with `renamePlaylist` (trimmed, ignored if empty or unchanged), Esc cancels;
    - «Más» menu with «Exportar M3U8» (`save({ defaultPath: "<name>.m3u8" })` → `exportM3u`), «Mover a carpeta ▸» (folders + «Sin carpeta») and «Eliminar»;
    - «Eliminar» shows `ConfirmInline` («¿Eliminar «X»? No se puede deshacer.» with «Eliminar» / «Cancelar» buttons); confirming calls `deletePlaylist` and navigates to `/`.
  - TrackList with `rows[].position` (the playlist item's position). Play goes from the index.
  - Empty state: «Arrastra canciones aquí o usa «Añadir a playlist»».
- **`/collection/tracks`:** «Me gusta» (kind «COLECCIÓN») with a TrackList of `likedTracks()`; play is `{kind:"liked"}`. Empty state: «Las canciones que marques con ♥ aparecen aquí».
- **Playlist deleted while open:** `playlist()` → notFound → `NotFound`.

- [ ] **Step 1: Write the failing tests**

```ts
test("plays the playlist from the clicked track")
test("renames on enter and cancels on escape")         // "Mix 2" + Enter → renamePlaylist(3,"Mix 2"); Esc → not called
test("deleting asks first and then goes home")         // "Eliminar" → deletePlaylist not called; confirm → called and route "/"
test("exports to the chosen file")                     // save() → "/tmp/x.m3u8" → exportM3u(3, "/tmp/x.m3u8")
test("moves the playlist into a folder")               // movePlaylistToFolder(3, 1)
test("a deleted playlist shows not found")
test("liked view plays the liked context")             // dblclick row 0 → play({kind:"liked"}, 0)
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** `@tauri-apps/plugin-dialog` `save` is mocked in `setup.ts`.
- [ ] **Step 4: Run** the gate. Expected: PASS.
- [ ] **Step 5: Commit** «UI: vista de playlist (renombrar, exportar, mover, eliminar con confirmación) y Me gusta»

### Task 7: Menú contextual y arrastrar y soltar

**Files:**
- Create:
  - `ui/src/menu/ContextMenu.tsx` + `.module.css`, `ui/src/menu/menu-store.ts`
  - `ui/src/tracks/track-menu.ts`
  - `ui/src/dnd/drag.ts`
  - tests `ui/src/menu/ContextMenu.test.tsx`, `ui/src/dnd/drag.test.tsx`
- Modify:
  - `TrackList` (right-click / Shift+F10 / context-menu key → menu; `draggable` rows; reordering);
  - `Sidebar` (playlists accept tracks; the «Me gusta» link too → `like` for each one);
  - `PlaylistView` (`menuExtra`: «Quitar de la playlist» → `removeFromPlaylist(id, positions)`; reordering → `movePlaylistItem`);
  - `apps/desktop/tauri.conf.json` (`"dragDropEnabled": false` on the window, so the webview doesn't intercept HTML5 drag and drop).

**Interfaces (Produces):**

```ts
export type MenuItem = { label: string; onSelect?(): void; disabled?: boolean; submenu?: MenuItem[]; danger?: boolean };
export const useMenu = create<{ open: null | { x: number; y: number; items: MenuItem[] }; show(x: number, y: number, items: MenuItem[]): void; close(): void }>;
export function ContextMenu(): JSX.Element; // mounted in AppShell; role="menu"; arrows, Enter, → opens submenu, Esc/click outside closes and returns focus
export function trackMenu(tracks: Track[], deps: { playlists: Playlist[]; navigate(to: string): void; liked: Set<number>;
  artistId(name: string): number | null }): MenuItem[];
// order: "Añadir a la cola" (queueAdd ids) · "Añadir a playlist ▸" (playlists + "Nueva playlist…" which creates «Nueva playlist» and adds) ·
// "Ir al álbum" (disabled if !albumId or several distinct albums) · "Ir al artista" (disabled if not resolved) · "Me gusta"/"Quitar de Me gusta"
export const TRACKS_MIME = "application/x-goymusic-tracks"; // dataTransfer: JSON { ids: number[]; from?: { playlist: number; positions: number[] } }
export function writeDrag(dt: DataTransfer, payload: DragPayload): void;
export function readDrag(dt: DataTransfer): DragPayload | null; // null if the MIME is missing or the JSON is invalid
```

Dragging a selected row drags the whole selection; dragging an unselected row drags only that row.

- [ ] **Step 1: Write the failing tests**

```ts
test("right click opens the track menu and adds to the queue")  // contextmenu on row → "Añadir a la cola" → queueAdd([5])
test("adds the selection to a playlist from the submenu")       // 2 selected → "Añadir a playlist" → "Mix" → addToPlaylist(3,[5,9])
test("go to album is disabled for tracks without album")        // Review Focus 2
test("escape closes the menu and returns focus to the row")
test("dropping tracks on a sidebar playlist adds them")         // drop with TRACKS_MIME {ids:[5,9]} → addToPlaylist(3,[5,9])
test("reordering inside a playlist moves the item")             // drag position 0 → drop over row 2 → movePlaylistItem(3,0,2)
test("drops without our data are ignored")                      // readDrag with text/plain → null; addToPlaylist not called
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** Use `fireEvent.dragStart/dragOver/drop` with a fake `dataTransfer` (`{ data: {}, setData, getData, types }`) in the tests.
- [ ] **Step 4: Run** the UI gate **and** the Rust gate (because of `tauri.conf.json`). Expected: PASS.
- [ ] **Step 5: Commit** «UI: menú contextual de pistas y arrastrar y soltar a playlists, Me gusta y reordenación»

### Task 8: Panel de cola y búsqueda

**Files:**
- Create: `ui/src/queue/QueuePanel.tsx` + `.module.css`, `ui/src/views/SearchView.tsx`, tests `QueuePanel.test.tsx`, `SearchView.test.tsx`
- Modify: `AppShell` (`QueuePanel` in the `queue` area when `queueOpen`), `queries.ts` (`useSearch(q)` with `placeholderData: keepPreviousData`)

**Behaviour:**
- **QueuePanel:**
  - heading «Cola»;
  - «Sonando» (current track, compact);
  - «En cola» (`userQueue`): rows with a «Quitar» button (`queueRemove(i)`) and drag reordering (`queueMove(from,to)`); it accepts drops with `TRACKS_MIME` → `queueAdd(ids)`;
  - «A continuación» (`upNext`, read-only, mono numbers);
  - empty state for «En cola»: «Arrastra canciones aquí o usa «Añadir a la cola»».
- **SearchView `/search?q=`:**
  - large input with `autoFocus` that syncs `q` in the URL (`replace: true`) while typing;
  - query with `useDeferredValue(q)`, `limit 20`, only if `q.trim()` is non-empty;
  - sections, each only if it has results: «Mejor resultado» (artist / album / track card with its action), «Canciones» (TrackList of the first 8; play `{kind:"tracks", id}`), «Álbumes» (`AlbumGrid`), «Artistas» (links);
  - no results: «Sin resultados para «q»».

- [ ] **Step 1: Write the failing tests**

```ts
test("lists the user queue and up next")
test("removes a queued track")                          // "Quitar Levels" → queueRemove(0)
test("dropping tracks on the panel queues them")        // drop {ids:[7]} → queueAdd([7])
test("reorders the user queue by dragging")             // queueMove(0, 1)
test("search updates the url and shows the top result") // type "radioheaf" → route ?q=radioheaf; top artist → link /artist/2
test("search with no results says so")
test("an empty query does not search")                  // commands.search not called
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the gate. Expected: PASS.
- [ ] **Step 5: Commit** «UI: panel de cola con reordenación y búsqueda mientras se escribe»

### Task 9: Atajos, avisos y panel de errores

**Files:**
- Create:
  - `ui/src/app/shortcuts.ts` + test
  - `ui/src/app/Notices.tsx` + `.module.css` + test
  - `ui/src/core/report.ts` + test
- Modify:
  - `player-store.ts`: `Notice.kind` adds `"error" | "info"`, plus `grave: string | null`, `setGrave(m)` and `clearGrave()`;
  - `AppShell`: mounts `useShortcuts()` and `<Notices/>`;
  - all the `catch` blocks of the views/actions use `reportError`;
  - `Sidebar`'s M3U8 import uses `notify("info", …)` for the missing ones.

**Interfaces (Produces):**

```ts
export function useShortcuts(): void;             // keydown on window; respects the Global Constraints exclusions
export function isTypingTarget(t: EventTarget | null): boolean;
export function reportError(e: unknown): void;    // CommandError notFound|invalidInput|folderOverlap → notify("error", message);
                                                  // io|database|internal or unknown error → setGrave(message)
```

**Notices:**
- Toasts in the bottom-right corner above the player bar, `role="status"` (`role="alert"` for `deviceLost`), maximum 3 visible.
- `playbackError`/`info`/`error` close by themselves after 6 s and have a «Cerrar» button.
- `deviceLost` stays until the user closes it.
- Grave errors (`grave`) are a banner at the top of `<main>` with the message, the text «Los detalles están en ~/.local/state/goymusic/logs/» and «Cerrar».

- [ ] **Step 1: Write the failing tests**

```ts
test("space toggles playback outside of inputs")
test("typing a space in the search box does not pause")   // Review Focus 3
test("ctrl+arrows skip tracks but not inside inputs")
test("ctrl+f opens search and focuses the box")           // route /search, document.activeElement is the input
test("playback errors appear and go away on their own")   // fake timers, 6000 ms
test("device lost stays until dismissed")
test("grave errors open the error panel")                 // reportError(new CommandError({code:"database",message:"x"})) → banner with "x"
test("not found errors are a notice, not the panel")
```

- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the gate. Expected: PASS.
- [ ] **Step 5: Commit** «UI: atajos de teclado, avisos y panel de errores graves»

### Task 10: Icono, verificación visual y prueba anti-slop

**Files:**
- Create: `apps/desktop/icons/source.svg` (1024×1024: a brass ring like a filament over a rounded square in `paper` Night, no text)
- Regenerate: `apps/desktop/icons/*` with `pnpm tauri icon` from a 1024 px PNG
- Modify: whatever the visual review finds

**Steps:**
- [ ] **Step 1: Icon.**
  1. Rasterize `source.svg` to PNG 1024 with headless Brave (`brave-browser --headless --screenshot=… --window-size=1024,1024 file://…/source.svg`) or Pillow.
  2. Then `source ~/.nvm/nvm.sh && pnpm tauri icon <png> -o apps/desktop/icons`.
  3. Keep in `bundle.icon` the same names `tauri.conf.json` already uses.
- [ ] **Step 2: Visual verification in mock mode.**
  1. Run `pnpm --dir ui dev:mock` in the background.
  2. Take screenshots with headless Brave, at 900×600 and 1280×800 and in both themes, of `/`, `/album/1`, `/playlist/1` and `/search?q=…`, plus the queue open and a menu open.
  3. Look at each screenshot.
  4. Fix what fails: overflow, two-line text in buttons, contrast, alignment.
- [ ] **Step 3: Hallmark slop test.** Load `slop-test.md` from the skill, run the gates that apply to an app (universal, a11y, contrast, typography, microinteractions), fix the failures, and record the result in the ledger.
- [ ] **Step 4: Full gate** (UI + Rust, `cargo build -p goymusic-desktop`). Expected: PASS.
- [ ] **Step 5: Commit** «UI: icono de la app y ajustes de la revisión visual»
- [ ] **Step 6: Mandatory stop for Javi:** he opens `pnpm dev` with his real library and confirms the flow (play, queue, playlist, theme). Afterwards come the final review and the merge.
