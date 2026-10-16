import { useParams } from "react-router";
import type { PlayContext } from "../bindings";
import { playFrom } from "../core/play";
import { useArtist } from "../core/queries";
import { isNotFound } from "../core/result";
import { AlbumGrid } from "../library/AlbumGrid";
import { TrackList } from "../tracks/TrackList";
import { countLabel } from "./format";
import { LoadError, NotFound } from "./NotFound";
import { PlayButton, ViewHeader } from "./ViewHeader";
import styles from "./views.module.css";

export function ArtistView() {
  const id = Number(useParams().id);
  const { data, error } = useArtist(id);
  if (isNotFound(error)) return <NotFound what="Este artista ya no está en tu biblioteca." />;
  if (error) return <LoadError />;
  if (!data) return null;
  const { artist, albums, tracks } = data;
  const context: PlayContext = { kind: "artist", id };

  return (
    <section className={styles.page}>
      <ViewHeader
        kind="ARTISTA"
        title={artist.name}
        meta={[countLabel(albums.length, "álbum", "álbumes"), countLabel(tracks.length, "canción", "canciones")]}
        actions={<PlayButton name={artist.name} onClick={() => playFrom(context, 0)} />}
      />
      {albums.length > 0 && (
        <section className={styles.section} aria-labelledby="artist-albums">
          <h2 id="artist-albums" className={styles.sectionTitle}>
            Álbumes
          </h2>
          <AlbumGrid albums={albums} />
        </section>
      )}
      <section className={styles.section} aria-labelledby="artist-songs">
        <h2 id="artist-songs" className={styles.sectionTitle}>
          Canciones
        </h2>
        <TrackList
          label={`Canciones de ${artist.name}`}
          rows={tracks.map((track) => ({ track, key: String(track.id) }))}
          onPlay={(index) => playFrom(context, index)}
        />
      </section>
    </section>
  );
}
