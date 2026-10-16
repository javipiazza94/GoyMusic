/** Controllable `prefers-color-scheme` for jsdom, which has no `matchMedia`. */
type Listener = (e: { matches: boolean }) => void;

let dark = true;
const listeners = new Set<Listener>();

export function installMatchMedia(): void {
  window.matchMedia = ((query: string) => ({
    get matches() {
      return query.includes("dark") ? dark : !dark;
    },
    media: query,
    addEventListener: (_: string, l: Listener) => listeners.add(l),
    removeEventListener: (_: string, l: Listener) => listeners.delete(l),
    addListener: () => {},
    removeListener: () => {},
    onchange: null,
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
}

export function setSystemDark(value: boolean): void {
  dark = value;
  listeners.forEach((l) => l({ matches: value }));
}

export function resetMatchMedia(): void {
  dark = true;
  listeners.clear();
}
