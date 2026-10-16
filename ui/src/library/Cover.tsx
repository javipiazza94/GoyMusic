import { useState } from "react";
import { artUrl } from "../core/queries";
import { GenArt } from "./GenArt";
import styles from "./Cover.module.css";

/** Album cover, or generated art in the item's own colours when there is no album or no art. */
export function Cover({
  albumId,
  size,
  label = "",
  seed,
}: {
  albumId: number | null;
  size: 200 | 600;
  label?: string;
  /** Name the generated art is made from (album, track or playlist title). */
  seed?: string;
}) {
  const [failed, setFailed] = useState<number | null>(null);
  if (albumId === null || failed === albumId) {
    return <GenArt seed={seed ?? (label || String(albumId ?? ""))} label={label} />;
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
