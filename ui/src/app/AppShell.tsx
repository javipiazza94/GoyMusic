import { ContextMenu } from "../menu/ContextMenu";
import { PlayerBar } from "../player/PlayerBar";
import { QueuePanel } from "../queue/QueuePanel";
import { useState } from "react";
import { Notices } from "./Notices";
import { AppRoutes } from "./routes";
import { ScrollParent } from "./scroll-parent";
import { useShortcuts } from "./shortcuts";
import { Sidebar } from "./Sidebar";
import styles from "./AppShell.module.css";
import { useUi } from "./ui-store";

/** Layout of spec §7: sidebar, main view, queue panel and the player bar along the bottom. */
export function AppShell() {
  const queueOpen = useUi((s) => s.queueOpen);
  const [main, setMain] = useState<HTMLElement | null>(null);
  useShortcuts();
  return (
    <div className={styles.shell} data-queue={queueOpen || undefined}>
      <button type="button" className={styles.skip} onClick={() => main?.focus()}>
        Saltar al contenido
      </button>
      <Sidebar />
      <main className={styles.main} ref={setMain} tabIndex={-1}>
        <ScrollParent.Provider value={main}>
          <AppRoutes />
        </ScrollParent.Provider>
      </main>
      {queueOpen && <QueuePanel />}
      <footer className={styles.player}>
        <PlayerBar />
      </footer>
      <ContextMenu />
      <Notices />
    </div>
  );
}
