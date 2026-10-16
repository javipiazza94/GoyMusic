import { save } from "@tauri-apps/plugin-dialog";
import { useRef, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { commands, type Playlist } from "../bindings";
import { playFrom } from "../core/play";
import { usePlaylist, usePlaylistFolders } from "../core/queries";
import { isNotFound, unwrap } from "../core/result";
import { reportError, runCommand } from "../core/report";
import { TrackList } from "../tracks/TrackList";
import { ConfirmInline } from "./ConfirmInline";
import { countLabel, totalDuration } from "./format";
import { LoadError, NotFound } from "./NotFound";
import { PlayButton, ViewHeader } from "./ViewHeader";
import styles from "./views.module.css";

const fail = reportError;

function RenameField({ playlist, onDone }: { playlist: Playlist; onDone(): void }) {
  const [name, setName] = useState(playlist.name);
  // Enter/Escape unmount the field, and some engines then fire blur: act only once.
  const finished = useRef(false);

  function finish(saveIt: boolean) {
    if (finished.current) return;
    finished.current = true;
    const trimmed = name.trim();
    if (saveIt && trimmed && trimmed !== playlist.name) {
      void commands.renamePlaylist(playlist.id, trimmed).then(unwrap).catch(fail);
    }
    onDone();
  }

  return (
    <input
      className={styles.titleInput}
      aria-label="Nombre de la playlist"
      value={name}
      autoFocus
      onChange={(e) => setName(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter") finish(true);
        if (e.key === "Escape") finish(false);
      }}
      onBlur={() => finish(true)}
    />
  );
}

export function PlaylistView() {
  const id = Number(useParams().id);
  const navigate = useNavigate();
  const { data, error } = usePlaylist(id);
  const folders = usePlaylistFolders().data ?? [];
  const [renaming, setRenaming] = useState(false);
  const [confirming, setConfirming] = useState(false);

  if (isNotFound(error)) return <NotFound what="Esta playlist ya no existe." />;
  if (error) return <LoadError />;
  if (!data) return null;
  const { playlist, items } = data;
  const tracks = items.map((i) => i.track);

  async function exportM3u() {
    const path = await save({ defaultPath: `${playlist.name}.m3u8`, filters: [{ name: "M3U8", extensions: ["m3u8"] }] });
    if (path) unwrap(await commands.exportM3u(playlist.id, path));
  }

  async function remove() {
    unwrap(await commands.deletePlaylist(playlist.id));
    navigate("/");
  }

  return (
    <section className={styles.page}>
      <ViewHeader
        kind="PLAYLIST"
        seed={playlist.name}
        cover={{ albumId: null }}
        title={renaming ? <RenameField playlist={playlist} onDone={() => setRenaming(false)} /> : playlist.name}
        meta={[countLabel(items.length, "canción", "canciones"), ...(items.length ? [totalDuration(tracks)] : [])]}
        actions={
          <>
            {items.length > 0 && <PlayButton name={playlist.name} onClick={() => playFrom({ kind: "playlist", id }, 0)} />}
            <button type="button" className={styles.secondaryButton} onClick={() => setRenaming(true)}>
              Renombrar
            </button>
            <button type="button" className={styles.secondaryButton} onClick={() => void exportM3u().catch(fail)}>
              Exportar M3U8
            </button>
            <label className={styles.field}>
              <span>Carpeta</span>
              <select
                className={styles.select}
                value={playlist.folderId ?? ""}
                onChange={(e) =>
                  void commands
                    .movePlaylistToFolder(id, e.target.value === "" ? null : Number(e.target.value))
                    .then(unwrap)
                    .catch(fail)
                }
              >
                <option value="">Sin carpeta</option>
                {folders.map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.name}
                  </option>
                ))}
              </select>
            </label>
            <button type="button" className={styles.textButton} onClick={() => setConfirming(true)}>
              Eliminar
            </button>
          </>
        }
      />
      {confirming && (
        <ConfirmInline
          message={`¿Eliminar «${playlist.name}»? No se puede deshacer.`}
          confirmLabel="Eliminar playlist"
          onConfirm={() => void remove().catch(fail)}
          onCancel={() => setConfirming(false)}
        />
      )}
      {items.length === 0 ? (
        <p className={styles.hint}>Arrastra canciones aquí o usa «Añadir a playlist» en el menú de una canción.</p>
      ) : (
        <TrackList
          label={`Canciones de ${playlist.name}`}
          rows={items.map((i) => ({ track: i.track, key: `${i.position}-${i.track.id}`, position: i.position }))}
          onPlay={(index) => playFrom({ kind: "playlist", id }, index)}
          onReorder={(from, to) => runCommand(commands.movePlaylistItem(id, items[from].position, items[to].position))}
          menuExtra={(rows) => [
            {
              label: "Quitar de la playlist",
              danger: true,
              onSelect: () => runCommand(commands.removeFromPlaylist(id, rows.map((r) => r.position!))),
            },
          ]}
        />
      )}
    </section>
  );
}
