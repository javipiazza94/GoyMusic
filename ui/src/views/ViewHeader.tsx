import type { CSSProperties, ReactNode } from "react";
import { Cover } from "../library/Cover";
import { hueStyle } from "../library/hue";
import styles from "./views.module.css";

/**
 * Head of a collection view: cover, mono kind label, display title, meta line and actions,
 * over a colour band made from the item's own hues (`seed`, usually its name).
 */
export function ViewHeader(p: {
  kind: string;
  title: ReactNode;
  meta: string[];
  seed?: string;
  cover?: { albumId: number | null; round?: boolean };
  actions?: ReactNode;
}) {
  const seed = p.seed ?? (typeof p.title === "string" ? p.title : p.kind);
  return (
    <header className={styles.header} style={hueStyle(seed) as CSSProperties}>
      {p.cover && (
        <div className={styles.headerCover} data-round={p.cover.round || undefined}>
          <Cover albumId={p.cover.albumId} size={600} seed={seed} label={p.cover.albumId === null ? seed : ""} />
        </div>
      )}
      <div className={styles.headerText}>
        <span className={styles.kind}>{p.kind}</span>
        <h1 className={styles.title}>{p.title}</h1>
        {p.meta.length > 0 && <p className={styles.meta}>{p.meta.join(" · ")}</p>}
        {p.actions && <div className={styles.actions}>{p.actions}</div>}
      </div>
    </header>
  );
}

/** `name` completes the accessible label ("Reproducir <name>"); the visible text stays short. */
export function PlayButton({ onClick, name }: { onClick(): void; name: string }) {
  return (
    <button type="button" className={styles.playButton} aria-label={`Reproducir ${name}`} onClick={onClick}>
      <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M7 4.5v15l12-7.5z" fill="currentColor" />
      </svg>
      Reproducir
    </button>
  );
}
