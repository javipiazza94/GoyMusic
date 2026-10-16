import { open } from "@tauri-apps/plugin-dialog";
import { NavLink, useNavigate } from "react-router";
import { commands, type Playlist } from "../bindings";
import { Icon, type IconName } from "../components/Icon";
import { useLikedIds } from "../core/likes";
import { usePlaylistFolders, usePlaylists } from "../core/queries";
import { dropTarget } from "../dnd/drag";
import { runCommand, unwrap } from "../core/result";
import styles from "./Sidebar.module.css";

const NEW_PLAYLIST = "Nueva playlist";
const NEW_FOLDER = "Nueva carpeta";
const IMPORT = "Importar M3U8";


function NavItem({ to, icon, label, drop }: { to: string; icon: IconName; label: string; drop?: ReturnType<typeof dropTarget> }) {
  return (
    <NavLink to={to} end className={styles.navItem} {...drop}>
      <Icon name={icon} />
      <span>{label}</span>
    </NavLink>
  );
}

function PlaylistLink({ playlist }: { playlist: Playlist }) {
  return (
    <li>
      <NavLink
        to={`/playlist/${playlist.id}`}
        className={styles.playlist}
        {...dropTarget(({ ids }) => runCommand(commands.addToPlaylist(playlist.id, ids)))}
      >
        {playlist.name}
      </NavLink>
    </li>
  );
}

function ActionButton({ label, icon, onClick }: { label: string; icon: IconName; onClick(): Promise<void> }) {
  return (
    <button
      type="button"
      className={styles.iconButton}
      aria-label={label}
      title={label}
      onClick={() => void onClick().catch(console.error)}
    >
      <Icon name={icon} size={18} />
    </button>
  );
}

export function Sidebar() {
  const navigate = useNavigate();
  const liked = useLikedIds();
  const playlists = usePlaylists().data ?? [];
  const folders = usePlaylistFolders().data ?? [];
  const inFolder = (p: Playlist) => folders.some((f) => f.id === p.folderId);

  async function createPlaylist() {
    const playlist = unwrap(await commands.createPlaylist(NEW_PLAYLIST));
    navigate(`/playlist/${playlist.id}`);
  }

  async function createFolder() {
    unwrap(await commands.createPlaylistFolder(NEW_FOLDER, null));
  }

  async function importM3u() {
    const path = await open({ filters: [{ name: "M3U8", extensions: ["m3u8", "m3u"] }] });
    if (typeof path !== "string") return;
    const report = unwrap(await commands.importM3u(path));
    if (report.missing.length > 0) console.warn(`${report.missing.length} entradas no están en la biblioteca`);
    navigate(`/playlist/${report.playlist.id}`);
  }

  return (
    <aside className={styles.sidebar}>
      <div className={styles.brand}>GoyMusic</div>
      <nav aria-label="Principal" className={styles.nav}>
        <NavItem to="/" icon="home" label="Inicio" />
        <NavItem to="/search" icon="search" label="Buscar" />
        <NavItem
          to="/collection/tracks"
          icon="heart"
          label="Me gusta"
          drop={dropTarget(({ ids }) => ids.filter((id) => !liked.has(id)).forEach((id) => runCommand(commands.like(id))))}
        />
      </nav>

      <div className={styles.section}>
        <span className={styles.label}>PLAYLISTS</span>
        <div className={styles.actions}>
          <ActionButton label={NEW_PLAYLIST} icon="plus" onClick={createPlaylist} />
          <ActionButton label={NEW_FOLDER} icon="folder" onClick={createFolder} />
          <ActionButton label={IMPORT} icon="import" onClick={importM3u} />
        </div>
      </div>

      <nav aria-label="Playlists" className={styles.tree}>
        {folders.map((folder) => (
          <details key={folder.id} open role="group" aria-label={folder.name} className={styles.folder}>
            <summary className={styles.folderName}>
              <Icon name="folder" size={16} />
              {folder.name}
            </summary>
            <ul className={styles.list}>
              {playlists
                .filter((p) => p.folderId === folder.id)
                .map((p) => (
                  <PlaylistLink key={p.id} playlist={p} />
                ))}
            </ul>
          </details>
        ))}
        <ul className={styles.list}>
          {playlists
            .filter((p) => !inFolder(p))
            .map((p) => (
              <PlaylistLink key={p.id} playlist={p} />
            ))}
        </ul>
      </nav>

      <nav aria-label="Aplicación" className={styles.footer}>
        <NavItem to="/settings" icon="settings" label="Ajustes" />
      </nav>
    </aside>
  );
}
