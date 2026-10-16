import { useEffect, useRef } from "react";
import styles from "./views.module.css";

/** In-page confirmation for actions that cannot be undone; focus starts on the safe choice. */
export function ConfirmInline(p: { message: string; confirmLabel: string; onConfirm(): void; onCancel(): void }) {
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => cancel.current?.focus(), []);
  return (
    <div role="alertdialog" aria-label={p.message} className={styles.confirm} onKeyDown={(e) => e.key === "Escape" && p.onCancel()}>
      <p className={styles.confirmText}>{p.message}</p>
      <button type="button" className={styles.dangerButton} onClick={p.onConfirm}>
        {p.confirmLabel}
      </button>
      <button type="button" ref={cancel} className={styles.secondaryButton} onClick={p.onCancel}>
        Cancelar
      </button>
    </div>
  );
}
