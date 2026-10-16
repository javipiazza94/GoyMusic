import { useDeferredValue } from "react";
import { Link, useSearchParams } from "react-router";
import type { TopResult } from "../bindings";
import { playFrom } from "../core/play";
import { useSearch } from "../core/queries";
import { AlbumGrid } from "../library/AlbumGrid";
import { Cover } from "../library/Cover";
import { TrackList } from "../tracks/TrackList";
import styles from "./views.module.css";

const TOP_TRACKS = 8;

function TopCard({ top }: { top: TopResult }) {
  switch (top.kind) {
    case "artist":
      return (
        <Link to={`/artist/${top.item.id}`} className={styles.topCard}>
          <span className={styles.initials} aria-hidden="true">
            {top.item.name.slice(0, 1).toUpperCase()}
          </span>
          <span className={styles.topName}>{top.item.name}</span>
          <span className={styles.kind}>ARTISTA</span>
        </Link>
      );
    case "album":
      return (
        <Link to={`/album/${top.item.id}`} className={styles.topCard}>
          <div className={styles.topCover}>
            <Cover albumId={top.item.id} size={200} />
          </div>
          <span className={styles.topName}>{top.item.title}</span>
          <span className={styles.kind}>ÁLBUM{top.item.artist ? ` · ${top.item.artist.toUpperCase()}` : ""}</span>
        </Link>
      );
    case "track":
      return (
        <button
          type="button"
          className={styles.topCard}
          onClick={() => playFrom({ kind: "tracks", id: [top.item.id] }, 0)}
        >
          <div className={styles.topCover}>
            <Cover albumId={top.item.albumId} size={200} />
          </div>
          <span className={styles.topName}>{top.item.title}</span>
          <span className={styles.kind}>CANCIÓN · REPRODUCIR</span>
        </button>
      );
  }
}

export function SearchView() {
  const [params, setParams] = useSearchParams();
  const q = params.get("q") ?? "";
  const deferred = useDeferredValue(q);
  const { data } = useSearch(deferred);
  const query = deferred.trim();
  const empty = data && !data.top && !data.tracks.length && !data.albums.length && !data.artists.length;
  const tracks = data?.tracks.slice(0, TOP_TRACKS) ?? [];

  return (
    <section className={styles.page}>
      <input
        type="search"
        className={styles.searchBox}
        aria-label="Buscar en tu biblioteca"
        name="q"
        autoComplete="off"
        spellCheck={false}
        placeholder="Artistas, álbumes o canciones…"
        value={q}
        autoFocus
        onChange={(e) => setParams(e.target.value ? { q: e.target.value } : {}, { replace: true })}
      />
      {query && data && (
        <>
          {empty && <p className={styles.hint}>Sin resultados para «{query}»</p>}
          <div className={styles.searchTop}>
            {data.top && (
              <section aria-labelledby="s-top" className={styles.topSection}>
                <h2 id="s-top" className={styles.sectionTitle}>
                  Mejor resultado
                </h2>
                <TopCard top={data.top} />
              </section>
            )}
            {tracks.length > 0 && (
              <section aria-labelledby="s-tracks" className={styles.topTracks}>
                <h2 id="s-tracks" className={styles.sectionTitle}>
                  Canciones
                </h2>
                <TrackList
                  label="Canciones encontradas"
                  rows={tracks.map((track) => ({ track, key: String(track.id) }))}
                  showAlbum={false}
                  onPlay={(index) => playFrom({ kind: "tracks", id: tracks.map((t) => t.id) }, index)}
                />
              </section>
            )}
          </div>
          {data.albums.length > 0 && (
            <section aria-labelledby="s-albums" className={styles.section}>
              <h2 id="s-albums" className={styles.sectionTitle}>
                Álbumes
              </h2>
              <AlbumGrid albums={data.albums} />
            </section>
          )}
          {data.artists.length > 0 && (
            <section aria-labelledby="s-artists" className={styles.section}>
              <h2 id="s-artists" className={styles.sectionTitle}>
                Artistas
              </h2>
              <ul className={styles.chips}>
                {data.artists.map((a) => (
                  <li key={a.id}>
                    <Link to={`/artist/${a.id}`} className={styles.chip}>
                      {a.name}
                    </Link>
                  </li>
                ))}
              </ul>
            </section>
          )}
        </>
      )}
    </section>
  );
}
