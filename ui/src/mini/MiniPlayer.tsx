import { useState, type CSSProperties } from "react";
import { commands } from "../bindings";
import { Icon } from "../components/Icon";
import { usePlayer } from "../core/player-store";
import { reportError, runCommand } from "../core/report";
import { Cover } from "../library/Cover";
import { hueStyle } from "../library/hue";
import { SeekBar } from "../player/SeekBar";
import styles from "./MiniPlayer.module.css";

/** The compact window (spec §8): what is playing, the controls and the progress. */
export function MiniPlayer() {
  const view = usePlayer((s) => s.view);
  const togglePlay = usePlayer((s) => s.togglePlay);
  const [onTop, setOnTop] = useState(false);
  const track = view?.track ?? null;
  const playing = view?.state === "playing";
  const seed = track ? (track.album ?? track.title) : "GoyMusic";

  function pin() {
    const next = !onTop;
    setOnTop(next);
    void commands.setMiniOnTop(next).catch(reportError);
  }

  return (
    <div className={styles.mini} style={hueStyle(seed) as CSSProperties} data-tauri-drag-region>
      <div className={styles.art}>
        <Cover albumId={track?.albumId ?? null} size={200} seed={seed} />
      </div>
      <div className={styles.body} data-tauri-drag-region>
        <div className={styles.text} data-tauri-drag-region>
          <span className={styles.title}>{track?.title ?? "Elige algo que escuchar"}</span>
          {track && track.artists.length > 0 && <span className={styles.artists}>{track.artists.join(", ")}</span>}
        </div>
        <div className={styles.controls}>
          <button type="button" className={styles.icon} aria-label="Anterior" disabled={!track} onClick={() => runCommand(commands.previous())}>
            <Icon name="previous" size={18} />
          </button>
          <button
            type="button"
            className={styles.play}
            aria-label={playing ? "Pausa" : "Reproducir"}
            disabled={!track}
            onClick={() => void togglePlay().catch(reportError)}
          >
            <Icon name={playing ? "pause" : "play"} size={18} />
          </button>
          <button type="button" className={styles.icon} aria-label="Siguiente" disabled={!track} onClick={() => runCommand(commands.next())}>
            <Icon name="next" size={18} />
          </button>
        </div>
        <div className={styles.seek}>
          <SeekBar durationMs={track?.durationMs ?? 0} disabled={!track} />
        </div>
      </div>
      <div className={styles.side}>
        <button type="button" className={styles.icon} aria-label="Cerrar mini reproductor" onClick={() => void commands.hideMiniPlayer().catch(reportError)}>
          <Icon name="close" size={16} />
        </button>
        <button type="button" className={styles.icon} aria-label="Fijar encima" aria-pressed={onTop} onClick={pin}>
          <Icon name="pin" size={16} />
        </button>
        <button type="button" className={styles.icon} aria-label="Abrir GoyMusic" onClick={() => void commands.showMainWindow().catch(reportError)}>
          <Icon name="home" size={16} />
        </button>
      </div>
    </div>
  );
}
