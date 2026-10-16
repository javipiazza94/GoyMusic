/**
 * Generated colour for items without (or besides) cover art: a stable pair of hues per name.
 * Hues come from a curated set — magentas, violets, blues, teals, corals and ambers — so no
 * item lands on a muddy green or yellow. Lightness and chroma live in the `--gen-*` tokens.
 */
// In wheel order, teal → blue → violet → magenta → coral → amber (greens and yellows left out).
export const HUES = [185, 205, 235, 262, 285, 305, 330, 350, 10, 25, 40, 55] as const;

function hash(text: string): number {
  // FNV-1a, 32 bit
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h;
}

/** Two different hues for `seed`, always the same for the same seed. */
export function huesFor(seed: string): [number, number] {
  const h = hash(seed.trim().toLowerCase());
  const first = h % HUES.length;
  // a neighbour 1–2 steps along the wheel (never across the gap where greens would be):
  // related colours, so the gradient between them stays clean instead of turning grey
  const step = 1 + ((h >>> 8) % 2);
  const second = first + step < HUES.length ? first + step : first - step;
  return [HUES[first], HUES[second]];
}

/** Inline style that hands an item's hues to CSS (`--hue-a`, `--hue-b`). */
export function hueStyle(seed: string): Record<"--hue-a" | "--hue-b", string> {
  const [a, b] = huesFor(seed);
  return { "--hue-a": String(a), "--hue-b": String(b) };
}
