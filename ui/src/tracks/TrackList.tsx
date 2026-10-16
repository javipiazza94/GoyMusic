import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import { Link } from "react-router";
import type { Track } from "../bindings";
import { useScrollParent } from "../app/scroll-parent";
import { Icon } from "../components/Icon";
import { useArtistIndex } from "../core/artist-index";
import { useLikedIds, useToggleLike } from "../core/likes";
import { usePlayer } from "../core/player-store";
import { formatMs } from "../core/queries";
import { emptySelection, nextSelection, type Selection } from "./selection";
import styles from "./TrackList.module.css";

/** One row: `position` is the item's place in a playlist (used to remove or reorder it). */
export type TrackRow = { track: Track; key: string; position?: number };

export interface TrackListProps {
  rows: TrackRow[];
  /** `trackNo` shows the tag's track number (album views); `index` the row number. */
  numbering?: "index" | "trackNo";
  showAlbum?: boolean;
  /** Double click or Enter on a row. */
  onPlay(index: number): void;
  /** Called when the last rows come into view, to load more. */
  onEndReached?(): void;
  label: string;
}

const ROW_HEIGHT = 52;
const END_THRESHOLD = 20;
/** jsdom has no layout: start from a plausible viewport so the first rows render. */
const INITIAL_RECT = { width: 1024, height: 768 };

function Artists({ names }: { names: string[] }) {
  const index = useArtistIndex();
  return (
    <span className={styles.artists}>
      {names.map((name, i) => {
        const id = index.get(name.toLowerCase());
        return (
          <span key={`${name}-${i}`}>
            {i > 0 && ", "}
            {id ? <Link to={`/artist/${id}`} onClick={(e) => e.stopPropagation()}>{name}</Link> : name}
          </span>
        );
      })}
    </span>
  );
}

export function TrackList({ rows, numbering = "index", showAlbum = true, onPlay, onEndReached, label }: TrackListProps) {
  const scrollParent = useScrollParent();
  const ownScroller = useRef<HTMLDivElement>(null);
  const gridRef = useRef<HTMLDivElement>(null);
  const [scrollMargin, setScrollMargin] = useState(0);
  const [selection, setSelection] = useState<Selection>(emptySelection);
  const [focusIndex, setFocusIndex] = useState(0);
  const pendingFocus = useRef(false);
  const currentId = usePlayer((s) => s.view?.track?.id ?? null);
  const liked = useLikedIds();
  const toggleLike = useToggleLike();

  useLayoutEffect(() => {
    if (!scrollParent || !gridRef.current) return;
    const top = gridRef.current.getBoundingClientRect().top - scrollParent.getBoundingClientRect().top;
    setScrollMargin(top + scrollParent.scrollTop);
  }, [scrollParent, rows.length]);

  useEffect(() => {
    setSelection(emptySelection);
    setFocusIndex(0);
  }, [rows.length]);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollParent ?? ownScroller.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
    scrollMargin: scrollParent ? scrollMargin : 0,
    initialRect: INITIAL_RECT,
  });
  const items = virtualizer.getVirtualItems();
  const lastVisible = items.at(-1)?.index ?? -1;

  useEffect(() => {
    if (onEndReached && rows.length > 0 && lastVisible >= rows.length - END_THRESHOLD) onEndReached();
  }, [lastVisible, rows.length, onEndReached]);

  useEffect(() => {
    if (!pendingFocus.current) return;
    pendingFocus.current = false;
    gridRef.current?.querySelector<HTMLElement>(`[data-index="${focusIndex}"]`)?.focus();
  }, [focusIndex, items]);

  function moveFocus(index: number) {
    const next = Math.max(0, Math.min(rows.length - 1, index));
    pendingFocus.current = true;
    setFocusIndex(next);
    virtualizer.scrollToIndex(next);
  }

  function onKeyDown(e: KeyboardEvent, index: number) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = index + (e.key === "ArrowDown" ? 1 : -1);
      moveFocus(next);
      if (e.shiftKey) setSelection((s) => nextSelection(s, Math.max(0, Math.min(rows.length - 1, next)), { ctrl: false, shift: true }));
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      moveFocus(e.key === "Home" ? 0 : rows.length - 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      onPlay(index);
    }
  }

  function onClick(e: MouseEvent, index: number) {
    setSelection((s) => nextSelection(s, index, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey }));
  }

  const grid = (
    <div
      ref={gridRef}
      role="grid"
      aria-label={label}
      aria-rowcount={rows.length + 1}
      aria-multiselectable="true"
      className={styles.grid}
      data-album={showAlbum || undefined}
    >
      <div role="row" aria-rowindex={1} className={`${styles.row} ${styles.header}`}>
        <span role="columnheader" className={styles.number}>#</span>
        <span role="columnheader">Título</span>
        {showAlbum && <span role="columnheader">Álbum</span>}
        <span role="columnheader" className="visually-hidden">Me gusta</span>
        <span role="columnheader" className={styles.duration}>Duración</span>
      </div>
      <div className={styles.body} style={{ height: virtualizer.getTotalSize() }}>
        {items.map((item) => {
          const { track } = rows[item.index];
          const isLiked = liked.has(track.id);
          const isCurrent = track.id === currentId;
          const number = numbering === "trackNo" ? (track.trackNo ?? "") : item.index + 1;
          return (
            <div
              key={rows[item.index].key}
              role="row"
              aria-rowindex={item.index + 2}
              aria-selected={selection.keys.has(item.index)}
              aria-current={isCurrent || undefined}
              data-index={item.index}
              tabIndex={item.index === focusIndex ? 0 : -1}
              className={styles.row}
              style={{ transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)` }}
              onClick={(e) => onClick(e, item.index)}
              onDoubleClick={() => onPlay(item.index)}
              onFocus={() => setFocusIndex(item.index)}
              onKeyDown={(e) => onKeyDown(e, item.index)}
            >
              <span role="gridcell" className={styles.number}>
                {isCurrent ? <Icon name="volume" size={16} /> : number}
              </span>
              <span role="gridcell" className={styles.main}>
                <span className={styles.title}>{track.title}</span>
                {track.artists.length > 0 && <Artists names={track.artists} />}
              </span>
              {showAlbum && (
                <span role="gridcell" className={styles.album}>
                  {track.albumId !== null && track.album ? (
                    <Link to={`/album/${track.albumId}`} onClick={(e) => e.stopPropagation()}>
                      {track.album}
                    </Link>
                  ) : null}
                </span>
              )}
              <span role="gridcell">
                <button
                  type="button"
                  tabIndex={-1}
                  className={styles.like}
                  data-liked={isLiked || undefined}
                  aria-label={`Me gusta: ${track.title}`}
                  aria-pressed={isLiked}
                  onClick={(e) => {
                    e.stopPropagation();
                    void toggleLike(track.id, isLiked).catch(console.error);
                  }}
                >
                  <Icon name={isLiked ? "heartFilled" : "heart"} size={16} />
                </button>
              </span>
              <span role="gridcell" className={styles.duration}>{formatMs(track.durationMs)}</span>
            </div>
          );
        })}
      </div>
    </div>
  );

  if (scrollParent) return grid;
  return (
    <div ref={ownScroller} className={styles.scroller}>
      {grid}
    </div>
  );
}
