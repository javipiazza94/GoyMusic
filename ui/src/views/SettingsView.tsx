import { useQueryClient } from "@tanstack/react-query";
import { commands } from "../bindings";
import { usePlayer } from "../core/player-store";
import { keys, useFolders, useSettings } from "../core/queries";
import { unwrap } from "../core/result";
import { useTheme, type ThemePref } from "../core/theme";
import { addFolderFromDialog } from "../library/add-folder";
import styles from "./views.module.css";
import { reportError } from "../core/report";

const themes: Array<{ id: ThemePref; label: string }> = [
  { id: "system", label: "Sistema" },
  { id: "dark", label: "Oscuro" },
  { id: "light", label: "Claro" },
];

const fail = reportError;

export function SettingsView() {
  const folders = useFolders().data ?? [];
  const settings = useSettings().data;
  const scan = usePlayer((s) => s.scan);
  const { pref, setPref } = useTheme();
  const qc = useQueryClient();

  async function setReplaygain(on: boolean) {
    unwrap(await commands.setReplaygain(on));
    // no core event covers settings: refresh them here
    await qc.invalidateQueries({ queryKey: keys.settings });
  }

  return (
    <section className={styles.page}>
      <h1 className={styles.title}>Ajustes</h1>

      <section className={styles.block} aria-labelledby="set-folders">
        <h2 id="set-folders" className={styles.sectionTitle}>
          Carpetas de música
        </h2>
        <ul className={styles.folderList}>
          {folders.map((f) => (
            <li key={f.id} className={styles.folderRow}>
              <span className={styles.path}>{f.path}</span>
              <button
                type="button"
                className={styles.textButton}
                aria-label={`Quitar ${f.path}`}
                onClick={() => void commands.removeFolder(f.id).then(unwrap).catch(fail)}
              >
                Quitar
              </button>
            </li>
          ))}
        </ul>
        <div className={styles.actions}>
          <button type="button" className={styles.secondaryButton} onClick={() => void addFolderFromDialog().catch(fail)}>
            Añadir carpeta
          </button>
          <button type="button" className={styles.secondaryButton} onClick={() => void commands.rescan().then(unwrap).catch(fail)}>
            Reescanear
          </button>
          {scan && (
            <span className={styles.progress} role="status">
              Escaneando… {scan.done}/{scan.total}
            </span>
          )}
        </div>
        {settings && (
          <p className={styles.hint}>
            {settings.watch.mode === "live"
              ? "Los cambios en tus carpetas se detectan al momento."
              : `Solo se comprueban los cambios cada 10 min (${settings.watch.reason}).`}
          </p>
        )}
      </section>

      <section className={styles.block} aria-labelledby="set-playback">
        <h2 id="set-playback" className={styles.sectionTitle}>
          Reproducción
        </h2>
        <div className={styles.settingRow}>
          <span id="rg-label">ReplayGain</span>
          <button
            type="button"
            role="switch"
            aria-labelledby="rg-label"
            aria-checked={settings?.replaygain ?? false}
            className={styles.switch}
            disabled={!settings}
            onClick={() => void setReplaygain(!settings?.replaygain).catch(fail)}
          />
        </div>
        <p className={styles.hint}>Iguala el volumen entre canciones usando sus etiquetas.</p>
      </section>

      <section className={styles.block} aria-labelledby="set-theme">
        <h2 id="set-theme" className={styles.sectionTitle}>
          Apariencia
        </h2>
        <div role="radiogroup" aria-labelledby="set-theme" className={styles.segmented}>
          {themes.map((t) => (
            <label key={t.id} className={styles.segment}>
              <input type="radio" name="theme" value={t.id} checked={pref === t.id} onChange={() => setPref(t.id)} />
              <span>{t.label}</span>
            </label>
          ))}
        </div>
      </section>
    </section>
  );
}
