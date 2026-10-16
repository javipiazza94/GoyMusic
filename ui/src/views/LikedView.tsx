import { playFrom } from "../core/play";
import { useLikedTracks } from "../core/queries";
import { TrackList } from "../tracks/TrackList";
import { countLabel, totalDuration } from "./format";
import { PlayButton, ViewHeader } from "./ViewHeader";
import styles from "./views.module.css";

export function LikedView() {
  const tracks = useLikedTracks().data;
  if (!tracks) return null;
  return (
    <section className={styles.page}>
      <ViewHeader
        kind="COLECCIÓN"
        title="Me gusta"
        meta={[countLabel(tracks.length, "canción", "canciones"), ...(tracks.length ? [totalDuration(tracks)] : [])]}
        actions={tracks.length > 0 && <PlayButton name="Me gusta" onClick={() => playFrom({ kind: "liked" }, 0)} />}
      />
      {tracks.length === 0 ? (
        <p className={styles.hint}>Las canciones que marques con el corazón aparecen aquí.</p>
      ) : (
        <TrackList
          label="Canciones que te gustan"
          rows={tracks.map((track) => ({ track, key: String(track.id) }))}
          onPlay={(index) => playFrom({ kind: "liked" }, index)}
        />
      )}
    </section>
  );
}
