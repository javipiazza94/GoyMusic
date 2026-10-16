import { useState } from "react";
import { Icon } from "../components/Icon";
import { artUrl } from "../core/queries";
import styles from "./Cover.module.css";

/** Album cover, or a quiet placeholder when there is no album or no art. */
export function Cover({ albumId, size, label = "" }: { albumId: number | null; size: 200 | 600; label?: string }) {
  const [failed, setFailed] = useState<number | null>(null);
  if (albumId === null || failed === albumId) {
    return (
      <div className={styles.placeholder} role={label ? "img" : undefined} aria-label={label || undefined}>
        <Icon name="music" size={size === 600 ? 64 : 24} />
      </div>
    );
  }
  return (
    <img
      className={styles.cover}
      src={artUrl(albumId, size)}
      width={size}
      height={size}
      alt={label}
      loading="lazy"
      decoding="async"
      onError={() => setFailed(albumId)}
    />
  );
}
