import { vi, type Mock } from "vitest";
import type { commands as real } from "../bindings";
import { ok, view } from "./fixtures";

type CommandName = keyof typeof real;

/** Default answer of each mocked command: lists are empty, the player has a track loaded. */
const defaults = (): Partial<Record<CommandName, unknown>> => ({
  folders: [], albums: [], artists: [], tracks: [], recentAlbums: [], playlists: [], playlistFolders: [],
  likedTracks: [], likedIds: [],
  search: { top: null, tracks: [], albums: [], artists: [] },
  playerView: view(),
  settings: { replaygain: false, watch: { mode: "live" } },
});

const names: CommandName[] = [
  "folders", "addFolder", "removeFolder", "rescan", "albums", "album", "artists", "artist", "tracks", "track",
  "recentAlbums", "search", "playlists", "playlistFolders", "playlist", "createPlaylist", "renamePlaylist",
  "deletePlaylist", "addToPlaylist", "removeFromPlaylist", "movePlaylistItem", "createPlaylistFolder",
  "movePlaylistToFolder", "like", "unlike", "likedTracks", "likedIds", "importM3u", "exportM3u", "play", "pause",
  "resume", "next", "previous", "seek", "setShuffle", "setRepeat", "setVolume", "setReplaygain", "queueAdd",
  "queueRemove", "queueMove", "playerView", "settings",
];

export const commands = Object.fromEntries(names.map((n) => [n, vi.fn()])) as unknown as Record<CommandName, Mock>;

export const events = { coreEvent: { listen: vi.fn(async () => () => {}) } };

export function resetCommandMocks(): void {
  const d = defaults();
  for (const n of names) {
    commands[n].mockReset();
    commands[n].mockImplementation(async () => ok(n in d ? d[n] : null));
  }
}
