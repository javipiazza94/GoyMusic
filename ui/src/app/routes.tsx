import { Route, Routes } from "react-router";
import { NotFound } from "../views/NotFound";
import styles from "../views/views.module.css";

/** Temporary view until its task replaces it. */
function Placeholder({ title }: { title: string }) {
  return (
    <section className={styles.page}>
      <h1 className={styles.title}>{title}</h1>
    </section>
  );
}

/** The routes of spec §7. */
export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Placeholder title="Biblioteca" />} />
      <Route path="/album/:id" element={<Placeholder title="Álbum" />} />
      <Route path="/artist/:id" element={<Placeholder title="Artista" />} />
      <Route path="/playlist/:id" element={<Placeholder title="Playlist" />} />
      <Route path="/collection/tracks" element={<Placeholder title="Me gusta" />} />
      <Route path="/search" element={<Placeholder title="Buscar" />} />
      <Route path="/settings" element={<Placeholder title="Ajustes" />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}
