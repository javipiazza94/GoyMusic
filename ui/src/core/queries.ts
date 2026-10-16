import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { commands, type TrackSort } from "../bindings";
import { unwrap } from "./result";

/** Query keys. Core events invalidate by prefix: `library`, `playlists`, `likes`. */
export const keys = {
  library: ["library"],
  albums: ["library", "albums"],
  album: (id: number) => ["library", "album", id],
  artist: (id: number) => ["library", "artist", id],
  folders: ["library", "folders"],
  artists: ["library", "artists"],
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

export const TRACK_PAGE = 500;

/** All songs in pages of 500; the next page loads when the list nears its end. */
export function useTrackPages(sort: TrackSort) {
  return useInfiniteQuery({
    queryKey: [...keys.tracks, sort],
    initialPageParam: 0,
    queryFn: async ({ pageParam }) => unwrap(await commands.tracks(sort, pageParam, TRACK_PAGE)),
    getNextPageParam: (last, pages) => (last.length < TRACK_PAGE ? undefined : pages.length * TRACK_PAGE),
  });
}

export function useAlbum(id: number) {
  return useQuery({ queryKey: keys.album(id), queryFn: async () => unwrap(await commands.album(id)) });
}

export function useArtist(id: number) {
  return useQuery({ queryKey: keys.artist(id), queryFn: async () => unwrap(await commands.artist(id)) });
}

export function useArtists() {
  return useQuery({ queryKey: keys.artists, queryFn: async () => unwrap(await commands.artists()) });
}

export function useRecentAlbums() {
  return useQuery({ queryKey: keys.recent, queryFn: async () => unwrap(await commands.recentAlbums(12)) });
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
