import { useEffect } from "react";
import { Icon } from "../components/Icon";
import { usePlayer, type Notice } from "../core/player-store";
import styles from "./Notices.module.css";

const VISIBLE = 3;
const LINGER_MS = 6000;

function Toast({ notice }: { notice: Notice }) {
  const dismiss = usePlayer((s) => s.dismiss);
  const sticky = notice.kind === "deviceLost";
  useEffect(() => {
    if (sticky) return;
    const timer = setTimeout(() => dismiss(notice.id), LINGER_MS);
    return () => clearTimeout(timer);
  }, [notice.id, sticky, dismiss]);

  return (
    <div role={sticky ? "alert" : "status"} className={styles.toast} data-kind={notice.kind}>
      <p className={styles.message}>{notice.message}</p>
      <button type="button" className={styles.close} aria-label="Cerrar aviso" onClick={() => dismiss(notice.id)}>
        <Icon name="close" size={16} />
      </button>
    </div>
  );
}

/** Discreet notices for what the user can recover from, and a panel for serious failures (spec §9). */
export function Notices() {
  const notices = usePlayer((s) => s.notices);
  const grave = usePlayer((s) => s.grave);
  const clearGrave = usePlayer((s) => s.clearGrave);

  return (
    <>
      {grave && (
        <div role="alertdialog" aria-labelledby="grave-title" className={styles.grave}>
          <h2 id="grave-title" className={styles.graveTitle}>
            Algo ha fallado
          </h2>
          <p className={styles.message}>{grave}</p>
          <p className={styles.hint}>Los detalles están en ~/.local/state/goymusic/logs/</p>
          <button type="button" className={styles.graveClose} onClick={clearGrave}>
            Cerrar
          </button>
        </div>
      )}
      <div className={styles.stack}>
        {notices.slice(-VISIBLE).map((n) => (
          <Toast key={n.id} notice={n} />
        ))}
      </div>
    </>
  );
}
