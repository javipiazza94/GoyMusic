# Design — GoyMusic

A locked design system for the app. Every view redesign reads this file before emitting
code. Extend or amend it when the system needs to grow; do not regenerate it per view.

Decided with Javi on 2026-10-17: the first theme (Lumen, near-black) read as "todo a
negro". Direction chosen: **Color vivo** — a dark plum base full of colour, where every
playlist, album and artist carries its own generated colour, even without cover art
(most of Javi's files have no album tags, so colour cannot depend on covers).

## Genre
atmospheric (custom theme)

## Structure
App shell of spec §7: sidebar (N3 side-rail), main view, player bar, queue panel. Views
share one shape: a **colour band header** (generated art or cover + display title + meta +
actions) over the content (track list, grids).

## Theme — "Color vivo"

Vibe: "late-night plum, neon magenta and sunset coral". Dark is the default; light is a
warm-cream counterpart with the same accents.

| Token | Dark | Light |
|---|---|---|
| `--color-paper` | `oklch(15% 0.035 300)` | `oklch(97% 0.015 60)` |
| `--color-paper-2` | `oklch(19% 0.04 300)` | `oklch(94% 0.02 50)` |
| `--color-paper-3` | `oklch(24% 0.045 300)` | `oklch(90% 0.03 45)` |
| `--color-ink` | `oklch(97% 0.01 320)` | `oklch(20% 0.04 310)` |
| `--color-ink-2` | `oklch(82% 0.03 320)` | `oklch(38% 0.04 310)` |
| `--color-ink-3` | `oklch(70% 0.035 320)` | `oklch(46% 0.04 310)` |
| `--color-accent` (coral) | `oklch(74% 0.19 35)` | `oklch(56% 0.2 30)` |
| `--color-accent-2` (magenta) | `oklch(70% 0.2 350)` | `oklch(52% 0.2 350)` |
| `--color-focus` | accent | accent |

Atmosphere: two fixed blooms behind the main view — magenta top-left, coral top-right.

**Generated colour** (`ui/src/library/hue.ts`): every item gets a hue pair from its name,
drawn from a curated set (magentas, corals, ambers, violets, blues, teals — no muddy
greens or yellows). Lightness and chroma are tokens (`--gen-*`); only the hue varies per
item. Used for generated covers, the view header band and sidebar thumbnails.

## Typography
- Display: Instrument Serif 400, roman (never italic) — view titles, brand, initials on generated covers.
- Body: Geist Variable.
- Mono: Geist Mono Variable — numbers, durations, uppercase micro-labels.

## Spacing, radius, motion
4 pt scale (`--space-2xs` 4 px … `--space-3xl` 64 px). Radius 4 / 8 / pill. Fades only
(`--dur-fast` 120 ms, `--dur-base` 200 ms); `prefers-reduced-motion` removes them.

## Rules
- Every colour and font through tokens; generated colour only through `--gen-*` + a hue.
- Covers: real art when it exists, generated art otherwise — never an empty grey box.
- Accent fills only actions (play, active states); colour volume comes from art and blooms.
