import type { ReactNode } from "react";
import { Cover } from "../library/Cover";
import styles from "./views.module.css";

/** Head of a collection view: cover, mono kind label, display title, meta line and actions. */
export function ViewHeader(p: {
  kind: string;
  title: ReactNode;
  meta: string[];
  cover?: { albumId: number | null };
  actions?: ReactNode;
}) {
  return (
    <header className={styles.header}>
      {p.cover && (
        <div className={styles.headerCover}>
          <Cover albumId={p.cover.albumId} size={600} />
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
