import { Link } from "react-router";
import { commands, type RepeatMode, type Track } from "../bindings";
import { Icon } from "../components/Icon";
import { useArtistIndex } from "../core/artist-index";
import { useLikedIds, useToggleLike } from "../core/likes";
import { usePlayer } from "../core/player-store";
import { unwrap } from "../core/result";
import { useUi } from "../app/ui-store";
import { Cover } from "../library/Cover";
import styles from "./PlayerBar.module.css";
import { SeekBar } from "./SeekBar";
import { VolumeControl } from "./VolumeControl";

const nextRepeat: Record<RepeatMode, RepeatMode> = { off: "all", all: "one", one: "off" };
const repeatLabel: Record<RepeatMode, string> = { off: "Repetir: no", all: "Repetir: todo", one: "Repetir: una" };

/** Fires a player command; its effect arrives through the core's events. */
const run = (call: Promise<Parameters<typeof unwrap>[0]>) => void call.then(unwrap).catch(console.error);

function Artists({ track }: { track: Track }) {
  const index = useArtistIndex();
  if (track.artists.length === 0) return null;
  return (
    <span className={styles.artists}>
      {track.artists.map((name, i) => {
        const id = index.get(name.toLowerCase());
        return (
          <span key={name}>
            {i > 0 && ", "}
            {id ? <Link to={`/artist/${id}`}>{name}</Link> : name}
          </span>
        );
      })}
    </span>
  );
}

function NowPlaying({ track }: { track: Track | null }) {
  const liked = useLikedIds();
  const toggleLike = useToggleLike();
  if (!track) return <div className={styles.now}><span className={styles.idle}>Elige algo que escuchar</span></div>;
  const isLiked = liked.has(track.id);
  return (
    <div className={styles.now}>
      <div className={styles.thumb}>
        <Cover albumId={track.albumId} size={200} />
      </div>
      <div className={styles.meta}>
        {track.albumId !== null ? (
          <Link className={styles.title} to={`/album/${track.albumId}`}>
            {track.title}
          </Link>
        ) : (
          <span className={styles.title}>{track.title}</span>
        )}
        <Artists track={track} />
      </div>
      <button
        type="button"
        className={styles.iconButton}
        data-liked={isLiked || undefined}
        aria-label="Me gusta"
        aria-pressed={isLiked}
        onClick={() => void toggleLike(track.id, isLiked).catch(console.error)}
      >
        <Icon name={isLiked ? "heartFilled" : "heart"} />
      </button>
    </div>
  );
}

export function PlayerBar() {
  const view = usePlayer((s) => s.view);
  const togglePlay = usePlayer((s) => s.togglePlay);
  const { queueOpen, toggleQueue } = useUi();
  const track = view?.track ?? null;
  const empty = track === null;
  const playing = view?.state === "playing";
  const repeat = view?.repeat ?? "off";

  return (
    <div className={styles.bar}>
      <NowPlaying track={track} />

      <div className={styles.center}>
        <div className={styles.transport}>
          <button
            type="button"
            className={styles.iconButton}
            aria-label="Aleatorio"
            aria-pressed={view?.shuffle ?? false}
            disabled={empty}
            onClick={() => run(commands.setShuffle(!view?.shuffle))}
          >
            <Icon name="shuffle" />
          </button>
          <button type="button" className={styles.iconButton} aria-label="Anterior" disabled={empty} onClick={() => run(commands.previous())}>
            <Icon name="previous" />
          </button>
          <button
            type="button"
            className={styles.play}
            aria-label={playing ? "Pausa" : "Reproducir"}
            disabled={empty}
            onClick={() => void togglePlay().catch(console.error)}
          >
            <Icon name={playing ? "pause" : "play"} size={22} />
          </button>
          <button type="button" className={styles.iconButton} aria-label="Siguiente" disabled={empty} onClick={() => run(commands.next())}>
            <Icon name="next" />
          </button>
          <button
            type="button"
            className={styles.iconButton}
            aria-label={repeatLabel[repeat]}
            aria-pressed={repeat !== "off"}
            disabled={empty}
            onClick={() => run(commands.setRepeat(nextRepeat[repeat]))}
          >
            <Icon name={repeat === "one" ? "repeatOne" : "repeat"} />
          </button>
        </div>
        <SeekBar durationMs={track?.durationMs ?? 0} disabled={empty} />
      </div>

      <div className={styles.side}>
        <button type="button" className={styles.iconButton} aria-label="Cola" aria-pressed={queueOpen} onClick={toggleQueue}>
          <Icon name="queue" />
        </button>
        <VolumeControl />
      </div>
    </div>
  );
}
