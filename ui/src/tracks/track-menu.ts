import { commands, type Playlist, type Track } from "../bindings";
import { runCommand, unwrap } from "../core/result";
import type { MenuItem } from "../menu/menu-store";


export interface TrackMenuDeps {
  playlists: Playlist[];
  navigate(to: string): void;
  liked: Set<number>;
  artistId(name: string): number | null;
}

/** Actions on one or more tracks (spec §7): queue, playlist, go to album or artist, like. */
export function trackMenu(tracks: Track[], deps: TrackMenuDeps): MenuItem[] {
  const ids = tracks.map((t) => t.id);
  const albums = new Set(tracks.map((t) => t.albumId));
  const albumId = albums.size === 1 ? tracks[0]?.albumId ?? null : null;
  const artistNames = new Set(tracks.flatMap((t) => t.artists));
  const artistId = artistNames.size === 1 ? deps.artistId([...artistNames][0]) : null;
  const allLiked = ids.every((id) => deps.liked.has(id));

  return [
    { label: "Añadir a la cola", onSelect: () => runCommand(commands.queueAdd(ids)) },
    {
      label: "Añadir a playlist",
      submenu: [
        {
          label: "Nueva playlist…",
          onSelect: () =>
            void commands
              .createPlaylist("Nueva playlist")
              .then(unwrap)
              .then(async (p) => {
                unwrap(await commands.addToPlaylist(p.id, ids));
                deps.navigate(`/playlist/${p.id}`);
              })
              .catch(console.error),
        },
        ...deps.playlists.map((p) => ({ label: p.name, onSelect: () => runCommand(commands.addToPlaylist(p.id, ids)) })),
      ],
    },
    { label: "Ir al álbum", disabled: albumId === null, onSelect: () => deps.navigate(`/album/${albumId}`) },
    { label: "Ir al artista", disabled: artistId === null, onSelect: () => deps.navigate(`/artist/${artistId}`) },
    allLiked
      ? { label: "Quitar de Me gusta", onSelect: () => ids.forEach((id) => runCommand(commands.unlike(id))) }
      : { label: "Añadir a Me gusta", onSelect: () => ids.filter((id) => !deps.liked.has(id)).forEach((id) => runCommand(commands.like(id))) },
  ];
}
