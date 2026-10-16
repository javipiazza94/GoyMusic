import { useRef, type CSSProperties } from "react";
import { Icon } from "../components/Icon";
import { usePlayer } from "../core/player-store";
import styles from "./PlayerBar.module.css";

export function VolumeControl() {
  const volume = usePlayer((s) => s.view?.volume ?? 1);
  const setVolume = usePlayer((s) => s.setVolume);
  const beforeMute = useRef(1);
  const muted = volume === 0;
  const change = (v: number) => void setVolume(v).catch(console.error);

  function toggleMute() {
    if (muted) change(beforeMute.current || 1);
    else {
      beforeMute.current = volume;
      change(0);
    }
  }

  return (
    <div className={styles.volume}>
      <button type="button" className={styles.iconButton} aria-label={muted ? "Activar sonido" : "Silenciar"} onClick={toggleMute}>
        <Icon name={muted ? "volumeMuted" : "volume"} />
      </button>
      <div className={styles.filament} style={{ "--progress": `${volume * 100}%` } as CSSProperties}>
        <div className={styles.filamentFill} data-quiet />
        <input
          type="range"
          aria-label="Volumen"
          min={0}
          max={1}
          step={0.01}
          value={volume}
          onChange={(e) => change(Number(e.target.value))}
        />
      </div>
    </div>
  );
}
