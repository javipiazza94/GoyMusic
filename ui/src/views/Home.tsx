import { useCallback, useState } from "react";
import { Link } from "react-router";
import type { TrackSort } from "../bindings";
import { playFrom } from "../core/play";
import { useAlbums, useArtists, useFolders, useRecentAlbums, useTrackPages } from "../core/queries";
import { addFolderFromDialog } from "../library/add-folder";
import { AlbumGrid } from "../library/AlbumGrid";
import { TrackList } from "../tracks/TrackList";
import styles from "./views.module.css";
import { reportError } from "../core/report";

type Tab = "songs" | "albums" | "artists";
const TAB_KEY = "goymusic.homeTab";
const tabs: Array<{ id: Tab; label: string }> = [
  { id: "songs", label: "Canciones" },
  { id: "albums", label: "Álbumes" },
  { id: "artists", label: "Artistas" },
];
const sorts: Array<{ id: TrackSort; label: string }> = [
  { id: "title", label: "Título" },
  { id: "artist", label: "Artista" },
  { id: "album", label: "Álbum" },
  { id: "recentlyAdded", label: "Añadidas recientemente" },
];

function storedTab(): Tab | null {
  try {
    const v = localStorage.getItem(TAB_KEY);
    return v === "songs" || v === "albums" || v === "artists" ? v : null;
  } catch {
    return null;
  }
}

function Songs() {
  const [sort, setSort] = useState<TrackSort>("title");
  const pages = useTrackPages(sort);
  const rows = (pages.data?.pages.flat() ?? []).map((track) => ({ track, key: String(track.id) }));
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = pages;
  const loadMore = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) void fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  return (
    <>
      <div className={styles.toolbar}>
        <label className={styles.field}>
          <span>Ordenar por</span>
          <select className={styles.select} value={sort} onChange={(e) => setSort(e.target.value as TrackSort)}>
            {sorts.map((s) => (
              <option key={s.id} value={s.id}>
                {s.label}
              </option>
            ))}
          </select>
        </label>
      </div>
      <TrackList
        label="Canciones"
        rows={rows}
        onEndReached={loadMore}
        onPlay={(index) => playFrom({ kind: "library", id: sort }, index)}
      />
    </>
  );
}

function Artists() {
  const artists = useArtists().data ?? [];
  return (
    <ul className={styles.artistGrid}>
      {artists.map((a) => (
        <li key={a.id}>
          <Link to={`/artist/${a.id}`} className={styles.artistCard}>
            <span className={styles.initials} aria-hidden="true">
              {a.name.slice(0, 1).toUpperCase()}
            </span>
            <span className={styles.artistName}>{a.name}</span>
          </Link>
        </li>
      ))}
    </ul>
  );
}

export function Home() {
  const folders = useFolders();
  const albums = useAlbums();
  const recent = useRecentAlbums().data ?? [];
  const [chosen, setChosen] = useState<Tab | null>(storedTab);

  if (folders.data && folders.data.length === 0) {
    return (
      <section className={styles.page}>
        <h1 className={styles.title}>Biblioteca</h1>
        <div className={styles.empty}>
          <h2 className={styles.emptyTitle}>Tu biblioteca está vacía</h2>
          <p>Añade la carpeta donde guardas tu música. GoyMusic la escanea y vigila los cambios.</p>
          <button type="button" className={styles.playButton} onClick={() => void addFolderFromDialog().catch(reportError)}>
            Añadir carpeta
          </button>
        </div>
      </section>
    );
  }

  const fallback: Tab | null = albums.data ? (albums.data.length > 0 ? "albums" : "songs") : null;
  const tab = chosen ?? fallback;

  function choose(next: Tab) {
    setChosen(next);
    try {
      localStorage.setItem(TAB_KEY, next);
    } catch {
      // storage unavailable: the tab is remembered for this session only
    }
  }

  return (
    <section className={styles.page}>
      <h1 className={styles.title}>Biblioteca</h1>
      {recent.length > 0 && (
        <section className={styles.section} aria-labelledby="recent">
          <h2 id="recent" className={styles.sectionTitle}>
            Escuchado hace poco
          </h2>
          <AlbumGrid albums={recent} row />
        </section>
      )}
      {tab && (
        <div role="tablist" aria-label="Biblioteca" className={styles.tabs}>
          {tabs.map((t) => (
            <button
              key={t.id}
              type="button"
              role="tab"
              id={`tab-${t.id}`}
              aria-selected={tab === t.id}
              aria-controls="library-panel"
              className={styles.tab}
              onClick={() => choose(t.id)}
            >
              {t.label}
            </button>
          ))}
        </div>
      )}
      {tab && (
        <div role="tabpanel" id="library-panel" aria-labelledby={`tab-${tab}`} className={styles.panel}>
          {tab === "songs" && <Songs />}
          {tab === "albums" && <AlbumGrid albums={albums.data ?? []} />}
          {tab === "artists" && <Artists />}
        </div>
      )}
    </section>
  );
}
