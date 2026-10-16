import { create } from "zustand";

export type ThemePref = "system" | "dark" | "light";
type Resolved = "dark" | "light";

const STORAGE_KEY = "goymusic.theme";
const DARK_QUERY = "(prefers-color-scheme: dark)";

interface ThemeStore {
  pref: ThemePref;
  resolved: Resolved;
  setPref(p: ThemePref): void;
}

function readPref(): ThemePref {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return v === "dark" || v === "light" ? v : "system";
  } catch {
    return "system";
  }
}

const systemTheme = (): Resolved => (window.matchMedia(DARK_QUERY).matches ? "dark" : "light");

function apply(pref: ThemePref): Resolved {
  const resolved = pref === "system" ? systemTheme() : pref;
  document.documentElement.dataset.theme = resolved;
  return resolved;
}

const useThemeStore = create<ThemeStore>((set) => ({
  pref: "system",
  resolved: "dark",
  setPref(pref) {
    try {
      if (pref === "system") localStorage.removeItem(STORAGE_KEY);
      else localStorage.setItem(STORAGE_KEY, pref);
    } catch {
      // storage unavailable: the choice lasts for this session only
    }
    set({ pref, resolved: apply(pref) });
  },
}));

let unfollow: (() => void) | null = null;

/** Applies the saved choice to `<html data-theme>` and follows the system while the choice is "system". */
export function initTheme(): void {
  const pref = readPref();
  useThemeStore.setState({ pref, resolved: apply(pref) });
  unfollow?.();
  const media = window.matchMedia(DARK_QUERY);
  const onChange = () => {
    const { pref } = useThemeStore.getState();
    if (pref === "system") useThemeStore.setState({ resolved: apply("system") });
  };
  media.addEventListener("change", onChange);
  unfollow = () => media.removeEventListener("change", onChange);
}

export function useTheme(): ThemeStore {
  return useThemeStore();
}
