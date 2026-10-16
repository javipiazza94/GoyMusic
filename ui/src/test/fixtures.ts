import type { PlayerView, Track } from "../bindings";

export function track(id: number, title = `Track ${id}`): Track {
  return {
    id, path: `/m/${id}.flac`, title, artists: ["Ana"], album: "Album", albumId: 1, albumArtist: "Ana",
    trackNo: id, discNo: null, year: 2001, genre: null, durationMs: 200_000,
  };
}

export function view(overrides: Partial<PlayerView> = {}): PlayerView {
  return {
    state: "playing", track: track(1), positionMs: 0, volume: 1, replaygain: false, shuffle: false,
    repeat: "off", userQueue: [], upNext: [], ...overrides,
  };
}

export const ok = <T,>(data: T) => ({ status: "ok" as const, data });
