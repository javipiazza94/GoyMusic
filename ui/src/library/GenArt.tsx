import type { CSSProperties } from "react";
import styles from "./GenArt.module.css";
import { hueStyle } from "./hue";

/** A cover made from the item's own colours and initial, for anything without cover art. */
export function GenArt({ seed, label = "" }: { seed: string; label?: string }) {
  const initial = [...seed.trim()][0]?.toLocaleUpperCase("es") ?? "♪";
  return (
    <div
      className={styles.art}
      style={hueStyle(seed) as CSSProperties}
      role={label ? "img" : undefined}
      aria-label={label || undefined}
      aria-hidden={label ? undefined : true}
    >
      <span className={styles.initial} aria-hidden="true">
        {initial}
      </span>
    </div>
  );
}
