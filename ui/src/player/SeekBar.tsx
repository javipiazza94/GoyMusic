import { useState, type CSSProperties } from "react";
import { commands } from "../bindings";
import { usePlayer } from "../core/player-store";
import { formatMs } from "../core/queries";
import styles from "./PlayerBar.module.css";
import { runCommand } from "../core/report";

/** Progress "filament": shows the position, and seeks once per gesture (when the drag or key press ends). */
export function SeekBar({ durationMs, disabled }: { durationMs: number; disabled: boolean }) {
  const positionMs = usePlayer((s) => s.positionMs);
  const [dragMs, setDragMs] = useState<number | null>(null);
  const shown = Math.min(dragMs ?? positionMs, durationMs);
  const progress = durationMs > 0 ? (shown / durationMs) * 100 : 0;

  function commit() {
    if (dragMs === null) return;
    usePlayer.setState({ positionMs: dragMs });
    setDragMs(null);
    runCommand(commands.seek(dragMs));
  }

  return (
    <div className={styles.seek}>
      <span className={styles.time}>{formatMs(shown)}</span>
      <div className={styles.filament} style={{ "--progress": `${progress}%` } as CSSProperties}>
        <div className={styles.filamentFill} />
        <input
          type="range"
          aria-label="Posición"
          aria-valuetext={formatMs(shown)}
          min={0}
          max={durationMs}
          step={1000}
          value={shown}
          disabled={disabled}
          onChange={(e) => setDragMs(Number(e.target.value))}
          onPointerUp={commit}
          onKeyUp={commit}
          onBlur={commit}
        />
      </div>
      <span className={styles.time}>{formatMs(durationMs)}</span>
    </div>
  );
}
