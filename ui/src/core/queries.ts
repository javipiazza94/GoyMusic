import { useQuery } from "@tanstack/react-query";
import { commands } from "../bindings";
import { unwrap } from "./result";

/** Query keys. Core events invalidate by prefix: `library`, `playlists`, `likes`. */
export const keys = {
  library: ["library"],
  albums: ["library", "albums"],
  album: (id: number) => ["library", "album", id],
  artist: (id: number) => ["library", "artist", id],
  folders: ["library", "folders"],
  tracks: ["library", "tracks"],
  recent: ["library", "recent"],
  search: (q: string) => ["library", "search", q],
  playlists: ["playlists"],
  playlist: (id: number) => ["playlists", id],
  likes: ["likes"],
  settings: ["settings"],
} as const;

export function useAlbums() {
  return useQuery({ queryKey: keys.albums, queryFn: async () => unwrap(await commands.albums()) });
}

/** First page of songs by title; enough for the development shell. */
export function useTracks() {
  return useQuery({ queryKey: keys.tracks, queryFn: async () => unwrap(await commands.tracks("title", 0, 500)) });
}

export function useFolders() {
  return useQuery({ queryKey: keys.folders, queryFn: async () => unwrap(await commands.folders()) });
}

export function useSettings() {
  return useQuery({ queryKey: keys.settings, queryFn: async () => unwrap(await commands.settings()) });
}

/** Cover thumbnail served by the app's `art://` protocol. */
export const artUrl = (albumId: number, px: 200 | 600) => `art://localhost/album/${albumId}/${px}`;

export function formatMs(ms: number): string {
  const total = Math.floor(ms / 1000);
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
}

export function usePlaylists() {
  return useQuery({ queryKey: keys.playlists, queryFn: async () => unwrap(await commands.playlists()) });
}

export function usePlaylistFolders() {
  return useQuery({
    queryKey: [...keys.playlists, "folders"],
    queryFn: async () => unwrap(await commands.playlistFolders()),
  });
}
