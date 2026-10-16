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
