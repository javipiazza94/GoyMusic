import { AppRoutes } from "./routes";
import { Sidebar } from "./Sidebar";
import styles from "./AppShell.module.css";
import { useUi } from "./ui-store";

/** Layout of spec §7: sidebar, main view, queue panel and the player bar along the bottom. */
export function AppShell() {
  const queueOpen = useUi((s) => s.queueOpen);
  return (
    <div className={styles.shell} data-queue={queueOpen || undefined}>
      <Sidebar />
      <main className={styles.main}>
        <AppRoutes />
      </main>
      {queueOpen && (
        <aside className={styles.queue} aria-label="Cola">
          <h2 className={styles.queueTitle}>Cola</h2>
        </aside>
      )}
      <footer className={styles.player} />
    </div>
  );
}
