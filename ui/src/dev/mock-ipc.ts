// Development-only stand-in for the Rust core (`pnpm dev:mock`): invented library data and a toy player, so the UI
// can be seen and checked in a plain browser. Never bundled in a normal build.
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type {
  Album, Artist, Event, PlayContext, PlayerView, Playlist, PlaylistFolder, RepeatMode, Track,
} from "../bindings";

const artists: Artist[] = [
  { id: 1, name: "Lumbre Sur" },
  { id: 2, name: "Marea Alta" },
  { id: 3, name: "Los Ciervos de Papel" },
];

const albums: Album[] = [
  { id: 1, title: "Cartografía del humo", artist: "Lumbre Sur", year: 2019, trackCount: 10 },
  { id: 2, title: "Noches de cobre", artist: "Lumbre Sur", year: 2022, trackCount: 8 },
  { id: 3, title: "Bajamar", artist: "Marea Alta", year: 2016, trackCount: 9 },
  { id: 4, title: "Un bosque en la azotea", artist: "Los Ciervos de Papel", year: 2024, trackCount: 7 },
];

const words = ["Brasa", "Faro", "Vigilia", "Salitre", "Ceniza", "Órbita", "Musgo", "Cobre", "Deriva", "Umbral",
  "Neblina", "Ámbar", "Relámpago", "Raíz", "Marea", "Hilo", "Tormenta", "Fósforo", "Andén", "Lino"];

const tracks: Track[] = [];
for (const album of albums) {
  for (let n = 1; n <= album.trackCount; n++) {
    const id = tracks.length + 1;
    tracks.push({
      id, path: `/música/${album.title}/${n}.flac`, title: `${words[id % words.length]} ${n === 1 ? "" : `(${n})`}`.trim(),
      artists: [album.artist!], album: album.title, albumId: album.id, albumArtist: album.artist, trackNo: n, discNo: 1,
      year: album.year, genre: null, durationMs: 150_000 + ((id * 37_000) % 160_000),
    });
  }
}
for (let n = 0; n < 6; n++) {
  const id = tracks.length + 1;
  tracks.push({
    id, path: `/música/sueltas/${id}.mp3`, title: `Grabación ${n + 1}`, artists: [], album: null, albumId: null,
    albumArtist: null, trackNo: null, discNo: null, year: null, genre: null, durationMs: 95_000 + n * 20_000,
  });
}

const folders: PlaylistFolder[] = [{ id: 1, name: "Ánimo", parentId: null }];
let playlists: Playlist[] = [
  { id: 1, name: "Para escribir", folderId: 1, trackCount: 0 },
  { id: 2, name: "Carretera", folderId: null, trackCount: 0 },
];
const items = new Map<number, number[]>([[1, [3, 9, 14, 22, 30]], [2, [1, 2, 5, 18, 27, 35, 40]]]);
const liked = new Set<number>([3, 14, 22]);

const byId = (id: number) => tracks.find((t) => t.id === id)!;
const notFound = (what: string) => {
  throw { code: "notFound", message: `${what} no existe` };
};

const player = {
  state: "paused" as PlayerView["state"], list: [] as number[], index: -1, positionMs: 0, volume: 0.8 as number | null,
  shuffle: false, repeat: "off" as RepeatMode, userQueue: [] as number[],
};

const publish = (e: Event) => void emit("core-event", e);
const changed = () => {
  const track = player.list[player.index];
  publish({ type: "playerStateChanged", state: player.state, track: track ?? null });
};

function contextTracks(c: PlayContext): number[] {
  switch (c.kind) {
    case "album": return tracks.filter((t) => t.albumId === c.id).map((t) => t.id);
    case "artist": return tracks.filter((t) => t.artists.includes(artists.find((a) => a.id === c.id)?.name ?? "")).map((t) => t.id);
    case "playlist": return items.get(c.id) ?? [];
    case "liked": return [...liked];
    case "tracks": return c.id;
  }
}

function view(): PlayerView {
  const id = player.list[player.index];
  return {
    state: player.state, track: id ? byId(id) : null, positionMs: player.positionMs, volume: player.volume,
    replaygain: false, shuffle: player.shuffle, repeat: player.repeat, userQueue: player.userQueue.map(byId),
    upNext: player.list.slice(player.index + 1).map(byId),
  };
}

setInterval(() => {
  if (player.state !== "playing") return;
  player.positionMs += 250;
  const id = player.list[player.index];
  if (id && player.positionMs >= byId(id).durationMs) skip(1);
  else if (id) publish({ type: "positionChanged", track: id, positionMs: player.positionMs });
}, 250);

function skip(step: number) {
  player.index = Math.max(0, Math.min(player.list.length - 1, player.index + step));
  player.positionMs = 0;
  changed();
}

const playlistView = (p: Playlist): Playlist => ({ ...p, trackCount: items.get(p.id)?.length ?? 0 });

type Args = Record<string, any>;
const handlers: Record<string, (a: Args) => unknown> = {
  folders: () => [{ id: 1, path: "/home/usuario/Música" }],
  add_folder: (a) => ({ id: 2, path: a.path }),
  remove_folder: () => null,
  rescan: () => null,
  albums: () => albums,
  album: (a) => {
    const album = albums.find((x) => x.id === a.id) ?? notFound("El álbum");
    return { album, tracks: tracks.filter((t) => t.albumId === a.id) };
  },
  artists: () => artists,
  artist: (a) => {
    const artist = artists.find((x) => x.id === a.id) ?? notFound("El artista");
    return { artist, albums: albums.filter((x) => x.artist === artist.name), tracks: tracks.filter((t) => t.artists.includes(artist.name)) };
  },
  tracks: (a) => [...tracks].sort((x, y) => x.title.localeCompare(y.title)).slice(a.offset, a.offset + a.limit),
  track: (a) => byId(a.id),
  recent_albums: () => albums.slice(0, 3),
  search: (a) => {
    const q = String(a.query).toLowerCase();
    const t = tracks.filter((x) => x.title.toLowerCase().includes(q));
    const al = albums.filter((x) => x.title.toLowerCase().includes(q));
    const ar = artists.filter((x) => x.name.toLowerCase().includes(q));
    const top = ar[0] ? { kind: "artist", item: ar[0] } : al[0] ? { kind: "album", item: al[0] } : t[0] ? { kind: "track", item: t[0] } : null;
    return { top, tracks: t.slice(0, a.limit), albums: al, artists: ar };
  },
  playlists: () => playlists.map(playlistView),
  playlist_folders: () => folders,
  playlist: (a) => {
    const playlist = playlists.find((p) => p.id === a.id) ?? notFound("La playlist");
    return { playlist: playlistView(playlist), items: (items.get(a.id) ?? []).map((id, position) => ({ position, track: byId(id) })) };
  },
  create_playlist: (a) => {
    const p = { id: Math.max(0, ...playlists.map((x) => x.id)) + 1, name: a.name, folderId: null, trackCount: 0 };
    playlists.push(p);
    items.set(p.id, []);
    publish({ type: "playlistsChanged" });
    return p;
  },
  rename_playlist: (a) => {
    playlists = playlists.map((p) => (p.id === a.id ? { ...p, name: a.name } : p));
    publish({ type: "playlistsChanged" });
    return null;
  },
  delete_playlist: (a) => {
    playlists = playlists.filter((p) => p.id !== a.id);
    publish({ type: "playlistsChanged" });
    return null;
  },
  add_to_playlist: (a) => {
    items.set(a.id, [...(items.get(a.id) ?? []), ...a.tracks]);
    publish({ type: "playlistsChanged" });
    return null;
  },
  remove_from_playlist: (a) => {
    items.set(a.id, (items.get(a.id) ?? []).filter((_, i) => !a.positions.includes(i)));
    publish({ type: "playlistsChanged" });
    return null;
  },
  move_playlist_item: (a) => {
    const list = [...(items.get(a.id) ?? [])];
    const [moved] = list.splice(a.from, 1);
    list.splice(a.to, 0, moved);
    items.set(a.id, list);
    publish({ type: "playlistsChanged" });
    return null;
  },
  create_playlist_folder: (a) => {
    const f = { id: folders.length + 1, name: a.name, parentId: a.parent };
    folders.push(f);
    publish({ type: "playlistsChanged" });
    return f;
  },
  move_playlist_to_folder: (a) => {
    playlists = playlists.map((p) => (p.id === a.id ? { ...p, folderId: a.folder } : p));
    publish({ type: "playlistsChanged" });
    return null;
  },
  like: (a) => (liked.add(a.track), publish({ type: "likesChanged" }), null),
  unlike: (a) => (liked.delete(a.track), publish({ type: "likesChanged" }), null),
  liked_tracks: () => [...liked].map(byId),
  liked_ids: () => [...liked],
  import_m3u: () => ({ playlist: playlistView(playlists[0]), matched: 3, missing: ["/viejo/disco/pista.mp3"] }),
  export_m3u: () => null,
  play: (a) => {
    player.list = contextTracks(a.context);
    player.index = a.startIndex;
    player.positionMs = 0;
    player.state = "playing";
    changed();
    return null;
  },
  pause: () => ((player.state = "paused"), changed(), null),
  resume: () => ((player.state = player.list.length ? "playing" : "idle"), changed(), null),
  next: () => (skip(1), null),
  previous: () => (skip(-1), null),
  seek: (a) => ((player.positionMs = a.positionMs), null),
  set_shuffle: (a) => ((player.shuffle = a.on), publish({ type: "queueChanged" }), null),
  set_repeat: (a) => ((player.repeat = a.mode), publish({ type: "queueChanged" }), null),
  set_volume: (a) => ((player.volume = a.volume), null),
  set_replaygain: () => null,
  queue_add: (a) => (player.userQueue.push(...a.tracks), publish({ type: "queueChanged" }), null),
  queue_remove: (a) => (player.userQueue.splice(a.index, 1), publish({ type: "queueChanged" }), null),
  queue_move: (a) => {
    const [moved] = player.userQueue.splice(a.from, 1);
    player.userQueue.splice(a.to, 0, moved);
    publish({ type: "queueChanged" });
    return null;
  },
  player_view: () => view(),
  settings: () => ({ replaygain: false, watch: { mode: "live" } }),
};

player.list = contextTracks({ kind: "album", id: 1 });
player.index = 2;
player.positionMs = 64_000;

mockWindows("main");
mockIPC(
  (cmd, payload) => {
    const handler = handlers[cmd];
    if (!handler) throw { code: "internal", message: `mock: ${cmd} no implementado` };
    return handler((payload ?? {}) as Args);
  },
  { shouldMockEvents: true },
);
