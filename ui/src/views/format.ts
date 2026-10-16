import type { Track } from "../bindings";

export function totalDuration(tracks: Track[]): string {
  const minutes = Math.round(tracks.reduce((sum, t) => sum + t.durationMs, 0) / 60_000);
  return minutes < 60 ? `${minutes} min` : `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
}

export const countLabel = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
