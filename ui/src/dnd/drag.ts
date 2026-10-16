/** Tracks dragged inside the app (HTML5 drag and drop). `from` identifies the list and rows, for reordering. */
export type DragPayload = { ids: number[]; from?: { list: string; indexes: number[] } };

export const TRACKS_MIME = "application/x-goymusic-tracks";

export function writeDrag(dt: DataTransfer, payload: DragPayload): void {
  dt.setData(TRACKS_MIME, JSON.stringify(payload));
  dt.effectAllowed = "copyMove";
}

/** During `dragover` the data cannot be read yet; only its type. */
export const carriesTracks = (dt: DataTransfer | null): boolean => !!dt && Array.from(dt.types).includes(TRACKS_MIME);

export function readDrag(dt: DataTransfer | null): DragPayload | null {
  const raw = dt?.getData(TRACKS_MIME);
  if (!raw) return null;
  try {
    const value = JSON.parse(raw) as DragPayload;
    return Array.isArray(value.ids) && value.ids.every((id) => typeof id === "number") ? value : null;
  } catch {
    return null;
  }
}

/** Props for an element that accepts dropped tracks. */
export function dropTarget(onDrop: (payload: DragPayload) => void) {
  return {
    onDragOver(e: React.DragEvent) {
      if (!carriesTracks(e.dataTransfer)) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
      (e.currentTarget as HTMLElement).dataset.dropping = "true";
    },
    onDragLeave(e: React.DragEvent) {
      delete (e.currentTarget as HTMLElement).dataset.dropping;
    },
    onDrop(e: React.DragEvent) {
      delete (e.currentTarget as HTMLElement).dataset.dropping;
      const payload = readDrag(e.dataTransfer);
      if (!payload) return;
      e.preventDefault();
      onDrop(payload);
    },
  };
}
