import { Route, Routes } from "react-router";
import { AlbumView } from "../views/AlbumView";
import { ArtistView } from "../views/ArtistView";
import { Home } from "../views/Home";
import { NotFound } from "../views/NotFound";
import { SettingsView } from "../views/SettingsView";
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
      <Route path="/" element={<Home />} />
      <Route path="/album/:id" element={<AlbumView />} />
      <Route path="/artist/:id" element={<ArtistView />} />
      <Route path="/playlist/:id" element={<Placeholder title="Playlist" />} />
      <Route path="/collection/tracks" element={<Placeholder title="Me gusta" />} />
      <Route path="/search" element={<Placeholder title="Buscar" />} />
      <Route path="/settings" element={<SettingsView />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}
