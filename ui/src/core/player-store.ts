import { create } from "zustand";
import { commands, type PlayerView } from "../bindings";
import { unwrap } from "./result";

export type Notice = { id: number; kind: "playbackError" | "deviceLost"; message: string };

export interface PlayerStore {
  view: PlayerView | null;
  positionMs: number;
  scan: { done: number; total: number } | null;
  notices: Notice[];
  refresh(): Promise<void>;
  setPosition(trackId: number, ms: number): void;
  setScan(done: number, total: number): void;
  clearScan(): void;
  notify(kind: Notice["kind"], message: string): void;
  dismiss(id: number): void;
  setVolume(volume: number): Promise<void>;
  togglePlay(): Promise<void>;
}

let nextNotice = 1;

/** Player state, fed by core events (see `events.ts`); the UI never polls. */
export const usePlayer = create<PlayerStore>((set, get) => ({
  view: null,
  positionMs: 0,
  scan: null,
  notices: [],
  async refresh() {
    const view = unwrap(await commands.playerView());
    set({ view, positionMs: view.positionMs });
  },
  setPosition(trackId, ms) {
    if (get().view?.track?.id === trackId) set({ positionMs: ms });
  },
  setScan(done, total) {
    set({ scan: { done, total } });
  },
  clearScan() {
    set({ scan: null });
  },
  notify(kind, message) {
    set((s) => ({ notices: [...s.notices, { id: nextNotice++, kind, message }] }));
  },
  dismiss(id) {
    set((s) => ({ notices: s.notices.filter((n) => n.id !== id) }));
  },
  async setVolume(volume) {
    const view = get().view;
    if (view) set({ view: { ...view, volume } });
    unwrap(await commands.setVolume(volume));
  },
  async togglePlay() {
    unwrap(await (get().view?.state === "playing" ? commands.pause() : commands.resume()));
  },
}));
