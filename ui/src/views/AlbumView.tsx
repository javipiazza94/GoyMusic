import { useParams } from "react-router";
import type { PlayContext } from "../bindings";
import { playFrom } from "../core/play";
import { useAlbum } from "../core/queries";
import { isNotFound } from "../core/result";
import { TrackList } from "../tracks/TrackList";
import { countLabel, totalDuration } from "./format";
import { LoadError, NotFound } from "./NotFound";
import { PlayButton, ViewHeader } from "./ViewHeader";
import styles from "./views.module.css";

export function AlbumView() {
  const id = Number(useParams().id);
  const { data, error } = useAlbum(id);
  if (isNotFound(error)) return <NotFound what="Este álbum ya no está en tu biblioteca." />;
  if (error) return <LoadError />;
  if (!data) return null;
  const { album, tracks } = data;
  const context: PlayContext = { kind: "album", id };
  const meta = [album.artist, album.year && String(album.year), countLabel(tracks.length, "canción", "canciones"), totalDuration(tracks)];

  return (
    <section className={styles.page}>
      <ViewHeader
        kind="ÁLBUM"
        title={album.title}
        meta={meta.filter((m): m is string => Boolean(m))}
        seed={album.title}
        cover={{ albumId: album.id }}
        actions={<PlayButton name={album.title} onClick={() => playFrom(context, 0)} />}
      />
      <TrackList
        label={`Canciones de ${album.title}`}
        rows={tracks.map((track) => ({ track, key: String(track.id) }))}
        numbering="trackNo"
        showAlbum={false}
        onPlay={(index) => playFrom(context, index)}
      />
    </section>
  );
}
