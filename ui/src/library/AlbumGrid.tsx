import { Link } from "react-router";
import { commands, type Album } from "../bindings";
import { Icon } from "../components/Icon";
import { unwrap } from "../core/result";
import { Cover } from "./Cover";
import styles from "./AlbumGrid.module.css";

export function AlbumGrid({ albums, row = false }: { albums: Album[]; row?: boolean }) {
  return (
    <ul className={styles.grid} data-row={row || undefined}>
      {albums.map((album) => (
        <li key={album.id} className={styles.card}>
          <div className={styles.art}>
            <Link to={`/album/${album.id}`} tabIndex={-1} aria-hidden="true">
              <Cover albumId={album.id} size={200} />
            </Link>
            <button
              type="button"
              className={styles.play}
              aria-label={`Reproducir ${album.title}`}
              onClick={() => void commands.play({ kind: "album", id: album.id }, 0).then(unwrap).catch(console.error)}
            >
              <Icon name="play" size={18} />
            </button>
          </div>
          <Link to={`/album/${album.id}`} className={styles.title}>
            {album.title}
          </Link>
          <span className={styles.sub}>{[album.artist, album.year].filter(Boolean).join(" · ")}</span>
        </li>
      ))}
    </ul>
  );
}
