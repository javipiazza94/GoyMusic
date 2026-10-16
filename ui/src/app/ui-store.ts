import { create } from "zustand";

const QUEUE_KEY = "goymusic.queueOpen";

function load(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function save(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // storage unavailable: keep the value for this session only
  }
}

/** Layout state that belongs to this window only (not to the core). */
export const useUi = create<{ queueOpen: boolean; toggleQueue(): void }>((set, get) => ({
  queueOpen: load(QUEUE_KEY) === "true",
  toggleQueue() {
    const queueOpen = !get().queueOpen;
    save(QUEUE_KEY, String(queueOpen));
    set({ queueOpen });
  },
}));
