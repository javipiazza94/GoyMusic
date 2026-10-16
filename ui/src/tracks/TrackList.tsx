import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type DragEvent, type KeyboardEvent, type MouseEvent } from "react";
import { Link, useNavigate } from "react-router";
import type { Track } from "../bindings";
import { useScrollParent } from "../app/scroll-parent";
import { Icon } from "../components/Icon";
import { useArtistIndex } from "../core/artist-index";
import { useLikedIds, useToggleLike } from "../core/likes";
import { usePlayer } from "../core/player-store";
import { formatMs, usePlaylists } from "../core/queries";
import { carriesTracks, readDrag, writeDrag, type DragPayload } from "../dnd/drag";
import { useMenu, type MenuItem } from "../menu/menu-store";
import { trackMenu } from "./track-menu";
import { Cover } from "../library/Cover";
import { nextSelection, type Selection } from "./selection";
import styles from "./TrackList.module.css";
import { reportError } from "../core/report";

/** One row: `position` is the item's place in a playlist (used to remove or reorder it). */
export type TrackRow = { track: Track; key: string; position?: number };

export interface TrackListProps {
  rows: TrackRow[];
  /** `trackNo` shows the tag's track number (album views); `index` the row number. */
  numbering?: "index" | "trackNo";
  showAlbum?: boolean;
  /** Small cover (or generated art) per row; off in album views, where every row shares it. */
  showArt?: boolean;
  /** Double click or Enter on a row. */
  onPlay(index: number): void;
  /** Called when the last rows come into view, to load more. */
  onEndReached?(): void;
  /** Extra context-menu entries for the selected rows (e.g. "Quitar de la playlist"). */
  menuExtra?(rows: TrackRow[]): MenuItem[];
  /** Rows can be dragged to a new place in this same list. */
  onReorder?(from: number, to: number): void;
  label: string;
}

/** The drag in progress; `dragover` cannot read the payload, so lists check its source here. */
let activeDrag: DragPayload | null = null;

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

export function TrackList(props: TrackListProps) {
  const { rows, numbering = "index", showAlbum = true, onPlay, onEndReached, menuExtra, onReorder, label } = props;
  const showArt = props.showArt ?? showAlbum;
  const listId = useId();
  const navigate = useNavigate();
  const playlists = usePlaylists().data ?? [];
  const artistIndex = useArtistIndex();
  const showMenu = useMenu((s) => s.show);
  const [dropIndex, setDropIndex] = useState<number | null>(null);
  const scrollParent = useScrollParent();
  const ownScroller = useRef<HTMLDivElement>(null);
  const gridRef = useRef<HTMLDivElement>(null);
  const [scrollMargin, setScrollMargin] = useState(0);
  // The selection is kept by row key, so it survives more pages loading or a new order.
  const [picked, setPicked] = useState<{ keys: Set<string>; anchor: string | null }>({ keys: new Set(), anchor: null });
  const indexOf = useMemo(() => new Map(rows.map((r, i) => [r.key, i])), [rows]);
  const selection: Selection = useMemo(() => {
    const keys = new Set<number>();
    for (const key of picked.keys) {
      const index = indexOf.get(key);
      if (index !== undefined) keys.add(index);
    }
    return { keys, anchor: picked.anchor === null ? null : (indexOf.get(picked.anchor) ?? null) };
  }, [picked, indexOf]);
  const setSelection = (next: Selection | ((s: Selection) => Selection)) => {
    const value = typeof next === "function" ? next(selection) : next;
    setPicked({
      keys: new Set([...value.keys].map((i) => rows[i].key)),
      anchor: value.anchor === null ? null : (rows[value.anchor]?.key ?? null),
    });
  };
  const [focusIndexRaw, setFocusIndex] = useState(0);
  const focusIndex = Math.min(focusIndexRaw, Math.max(0, rows.length - 1));
  const pendingFocus = useRef(false);
  const currentId = usePlayer((s) => s.view?.track?.id ?? null);
  const liked = useLikedIds();
  const toggleLike = useToggleLike();

  useLayoutEffect(() => {
    if (!scrollParent || !gridRef.current) return;
    const top = gridRef.current.getBoundingClientRect().top - scrollParent.getBoundingClientRect().top;
    setScrollMargin(top + scrollParent.scrollTop);
  }, [scrollParent, rows.length]);

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
    // the row may not be rendered yet (scrolling to it is asynchronous): try again on the next render
    const row = gridRef.current?.querySelector<HTMLElement>(`[data-index="${focusIndex}"]`);
    if (!row) return;
    pendingFocus.current = false;
    row.focus();
  }, [focusIndex, items]);

  function moveFocus(index: number) {
    const next = Math.max(0, Math.min(rows.length - 1, index));
    pendingFocus.current = true;
    setFocusIndex(next);
    virtualizer.scrollToIndex(next);
  }

  function onKeyDown(e: KeyboardEvent, index: number) {
    if (e.altKey && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
      // keyboard alternative to dragging a row to a new place
      e.preventDefault();
      const to = index + (e.key === "ArrowDown" ? 1 : -1);
      if (!onReorder || to < 0 || to >= rows.length) return;
      onReorder(index, to);
      moveFocus(to);
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = index + (e.key === "ArrowDown" ? 1 : -1);
      moveFocus(next);
      if (e.shiftKey) setSelection((s) => nextSelection(s, Math.max(0, Math.min(rows.length - 1, next)), { ctrl: false, shift: true }));
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      moveFocus(e.key === "Home" ? 0 : rows.length - 1);
    } else if ((e.key === "F10" && e.shiftKey) || e.key === "ContextMenu") {
      e.preventDefault();
      const rect = e.currentTarget.getBoundingClientRect();
      openMenu(index, rect.left + 48, rect.bottom, e.currentTarget as HTMLElement);
    } else if (e.key === "Enter") {
      e.preventDefault();
      onPlay(index);
    }
  }

  /** The rows an action applies to: the selection if `index` is in it, otherwise just that row. */
  function targetIndexes(index: number): number[] {
    return selection.keys.has(index) ? [...selection.keys].sort((a, b) => a - b) : [index];
  }

  function openMenu(index: number, x: number, y: number, returnFocus: HTMLElement) {
    if (!selection.keys.has(index)) setSelection({ keys: new Set([index]), anchor: index });
    const target = targetIndexes(index).map((i) => rows[i]);
    const items = trackMenu(
      target.map((r) => r.track),
      { playlists, navigate, liked, artistId: (name) => artistIndex.get(name.toLowerCase()) ?? null },
    );
    showMenu(x, y, menuExtra ? [...items, ...menuExtra(target)] : items, returnFocus);
  }

  function onDragStart(e: DragEvent, index: number) {
    const indexes = targetIndexes(index);
    activeDrag = { ids: indexes.map((i) => rows[i].track.id), from: { list: listId, indexes } };
    writeDrag(e.dataTransfer, activeDrag);
  }

  const reorderingHere = () => !!onReorder && activeDrag?.from?.list === listId && activeDrag.from.indexes.length === 1;

  function onDragOver(e: DragEvent, index: number) {
    if (!carriesTracks(e.dataTransfer) || !reorderingHere()) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    setDropIndex(index);
  }

  function onDrop(e: DragEvent, index: number) {
    setDropIndex(null);
    const payload = readDrag(e.dataTransfer);
    if (!onReorder || payload?.from?.list !== listId || payload.from.indexes.length !== 1) return;
    e.preventDefault();
    const from = payload.from.indexes[0];
    if (from !== index) onReorder(from, index);
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
      data-art={showArt || undefined}
    >
      <div role="row" aria-rowindex={1} className={`${styles.row} ${styles.header}`}>
        <span role="columnheader" className={styles.number}>#</span>
        {showArt && <span role="columnheader" aria-label="Carátula" />}
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
              onContextMenu={(e) => {
                e.preventDefault();
                openMenu(item.index, e.clientX, e.clientY, e.currentTarget);
              }}
              draggable
              data-drop={dropIndex === item.index || undefined}
              onDragStart={(e) => onDragStart(e, item.index)}
              onDragEnd={() => {
                activeDrag = null;
                setDropIndex(null);
              }}
              onDragOver={(e) => onDragOver(e, item.index)}
              onDragLeave={() => setDropIndex((d) => (d === item.index ? null : d))}
              onDrop={(e) => onDrop(e, item.index)}
              onKeyDown={(e) => onKeyDown(e, item.index)}
            >
              <span role="gridcell" className={styles.number}>
                {isCurrent ? <Icon name="volume" size={16} /> : number}
              </span>
              {showArt && (
                <span role="gridcell" className={styles.art}>
                  <Cover albumId={track.albumId} size={200} seed={track.album ?? track.title} />
                </span>
              )}
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
                    void toggleLike(track.id, isLiked).catch(reportError);
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
