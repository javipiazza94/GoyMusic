import { Link } from "react-router";
import styles from "./views.module.css";

export function NotFound({ what }: { what?: string }) {
  return (
    <section className={styles.empty}>
      <h1 className={styles.emptyTitle}>No encontrado</h1>
      <p>{what ?? "Esta página no existe."}</p>
      <Link className={styles.emptyLink} to="/">
        Ir a Inicio
      </Link>
    </section>
  );
}

/** A view whose data could not be loaded; the error itself was already reported (error panel). */
export function LoadError() {
  return (
    <section className={styles.empty}>
      <h1 className={styles.emptyTitle}>No se pudo cargar esta vista.</h1>
      <p>Prueba de nuevo dentro de un momento.</p>
      <Link className={styles.emptyLink} to="/">
        Ir a Inicio
      </Link>
    </section>
  );
}
