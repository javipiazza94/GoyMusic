// Development shell: plain, unstyled views to try the core end to end. P3b replaces it with
// the designed UI.
import { open } from "@tauri-apps/plugin-dialog";
import { commands } from "./bindings";
import { usePlayer } from "./core/player-store";
import { artUrl, formatMs, useAlbums, useFolders, useTracks } from "./core/queries";
import { unwrap } from "./core/result";

export default function App() {
  const folders = useFolders();
  const albums = useAlbums();
  const tracks = useTracks();
  const trackIds = tracks.data?.map((t) => t.id) ?? [];
  const { view, positionMs, scan, notices, togglePlay, dismiss } = usePlayer();

  async function addFolder() {
    const path = await open({ directory: true });
    if (typeof path === "string") unwrap(await commands.addFolder(path));
  }

  return (
    <main>
      <h1>GoyMusic</h1>
      <section>
        <h2>Carpetas</h2>
        <ul>
          {folders.data?.map((f) => (
            <li key={f.id}>
              {f.path}{" "}
              <button aria-label={`Quitar ${f.path}`} onClick={() => void commands.removeFolder(f.id)}>
                Quitar
              </button>
            </li>
          ))}
        </ul>
        <button onClick={() => void addFolder()}>Añadir carpeta</button>
        {scan && <p>{`Escaneando… ${scan.done}/${scan.total}`}</p>}
      </section>
      <section>
        <h2>Álbumes</h2>
        <ul>
          {albums.data?.map((a) => (
            <li key={a.id}>
              <button onClick={() => void commands.play({ kind: "album", id: a.id }, 0)}>
                <img src={artUrl(a.id, 200)} alt="" width={64} height={64} />
                {a.title} — {a.artist ?? "Varios"}
              </button>
            </li>
          ))}
        </ul>
      </section>
      <section>
        <h2>Canciones</h2>
        <ol>
          {tracks.data?.map((t, i) => (
            <li key={t.id}>
              <button onClick={() => void commands.play({ kind: "tracks", id: trackIds }, i)}>
                {t.title} — {t.artists.join(", ") || "Desconocido"} · {formatMs(t.durationMs)}
              </button>
            </li>
          ))}
        </ol>
      </section>
      <footer>
        {view?.track && (
          <p>
            {view.track.title} — {view.track.artists.join(", ")} · {formatMs(positionMs)} /{" "}
            {formatMs(view.track.durationMs)}
          </p>
        )}
        <button onClick={() => void commands.previous()}>Anterior</button>
        <button onClick={() => void togglePlay()}>{view?.state === "playing" ? "Pausa" : "Reproducir"}</button>
        <button onClick={() => void commands.next()}>Siguiente</button>
        {notices.map((n) => (
          <p key={n.id} role="alert">
            {n.message} <button onClick={() => dismiss(n.id)}>Cerrar</button>
          </p>
        ))}
      </footer>
    </main>
  );
}
