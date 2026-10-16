import { useState, type DragEvent } from "react";
import { commands, type Track } from "../bindings";
import { Icon } from "../components/Icon";
import { usePlayer } from "../core/player-store";
import { formatMs } from "../core/queries";
import { runCommand } from "../core/report";
import { carriesTracks, readDrag, writeDrag } from "../dnd/drag";
import { Cover } from "../library/Cover";
import styles from "./QueuePanel.module.css";

const QUEUE_LIST = "queue";

function Item({ track, children }: { track: Track; children?: React.ReactNode }) {
  return (
    <>
      <div className={styles.thumb}>
        <Cover albumId={track.albumId} size={200} seed={track.album ?? track.title} />
      </div>
      <div className={styles.text}>
        <span className={styles.title}>{track.title}</span>
        {track.artists.length > 0 && <span className={styles.artists}>{track.artists.join(", ")}</span>}
      </div>
      {children ?? <span className={styles.duration}>{formatMs(track.durationMs)}</span>}
    </>
  );
}

/** Right-hand panel (spec §7): what plays now, the user's queue (editable) and what follows from the context. */
export function QueuePanel() {
  const view = usePlayer((s) => s.view);
  const [dropAt, setDropAt] = useState<number | null>(null);
  const [panelDrop, setPanelDrop] = useState(false);
  const userQueue = view?.userQueue ?? [];
  const upNext = view?.upNext ?? [];

  function onPanelDragOver(e: DragEvent) {
    if (!carriesTracks(e.dataTransfer)) return;
    e.preventDefault();
    setPanelDrop(true);
  }

  function onPanelDrop(e: DragEvent) {
    setPanelDrop(false);
    setDropAt(null);
    const payload = readDrag(e.dataTransfer);
    if (!payload) return;
    e.preventDefault();
    if (payload.from?.list === QUEUE_LIST) return; // moving inside the queue is handled by the items
    runCommand(commands.queueAdd(payload.ids));
  }

  function onItemDrop(e: DragEvent, index: number) {
    const payload = readDrag(e.dataTransfer);
    if (payload?.from?.list !== QUEUE_LIST) return; // tracks from elsewhere: the panel adds them
    e.preventDefault();
    e.stopPropagation();
    setDropAt(null);
    setPanelDrop(false);
    const from = payload.from.indexes[0];
    if (from !== index) runCommand(commands.queueMove(from, index));
  }

  return (
    <aside
      aria-label="Cola"
      className={styles.panel}
      data-dropping={panelDrop || undefined}
      onDragOver={onPanelDragOver}
      onDragLeave={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setPanelDrop(false);
      }}
      onDrop={onPanelDrop}
    >
      <h2 className={styles.heading}>Cola</h2>

      {view?.track && (
        <section aria-labelledby="q-now" className={styles.section}>
          <h3 id="q-now" className={styles.label}>
            Sonando
          </h3>
          <div className={styles.row} data-current>
            <Item track={view.track} />
          </div>
        </section>
      )}

      <section aria-labelledby="q-user" className={styles.section}>
        <h3 id="q-user" className={styles.label}>
          En cola
        </h3>
        {userQueue.length === 0 ? (
          <p className={styles.empty}>Arrastra canciones aquí o usa «Añadir a la cola» en su menú.</p>
        ) : (
          <ol className={styles.list}>
            {userQueue.map((track, index) => (
              <li
                key={`${index}-${track.id}`}
                className={styles.row}
                draggable
                tabIndex={0}
                aria-label={`${track.title}, posición ${index + 1} de ${userQueue.length}. Alt+flechas para moverla`}
                onKeyDown={(e) => {
                  if (!e.altKey || (e.key !== "ArrowDown" && e.key !== "ArrowUp")) return;
                  e.preventDefault();
                  const to = index + (e.key === "ArrowDown" ? 1 : -1);
                  if (to >= 0 && to < userQueue.length) runCommand(commands.queueMove(index, to));
                }}
                data-drop={dropAt === index || undefined}
                onDragStart={(e) => writeDrag(e.dataTransfer, { ids: [track.id], from: { list: QUEUE_LIST, indexes: [index] } })}
                onDragOver={(e) => {
                  if (!carriesTracks(e.dataTransfer)) return;
                  e.preventDefault();
                  setDropAt(index);
                }}
                onDrop={(e) => onItemDrop(e, index)}
              >
                <Item track={track}>
                  <button
                    type="button"
                    className={styles.remove}
                    aria-label={`Quitar ${track.title}`}
                    onClick={() => runCommand(commands.queueRemove(index))}
                  >
                    <Icon name="close" size={16} />
                  </button>
                </Item>
              </li>
            ))}
          </ol>
        )}
      </section>

      {upNext.length > 0 && (
        <section aria-labelledby="q-next" className={styles.section}>
          <h3 id="q-next" className={styles.label}>
            A continuación
          </h3>
          <ol className={styles.list}>
            {upNext.map((track, index) => (
              <li key={`${index}-${track.id}`} className={styles.row}>
                <Item track={track} />
              </li>
            ))}
          </ol>
        </section>
      )}
    </aside>
  );
}
