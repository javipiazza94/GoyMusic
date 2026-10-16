import { Route, Routes } from "react-router";
import { AlbumView } from "../views/AlbumView";
import { ArtistView } from "../views/ArtistView";
import { Home } from "../views/Home";
import { LikedView } from "../views/LikedView";
import { NotFound } from "../views/NotFound";
import { PlaylistView } from "../views/PlaylistView";
import { SearchView } from "../views/SearchView";
import { SettingsView } from "../views/SettingsView";

/** The routes of spec §7. */
export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Home />} />
      <Route path="/album/:id" element={<AlbumView />} />
      <Route path="/artist/:id" element={<ArtistView />} />
      <Route path="/playlist/:id" element={<PlaylistView />} />
      <Route path="/collection/tracks" element={<LikedView />} />
      <Route path="/search" element={<SearchView />} />
      <Route path="/settings" element={<SettingsView />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}
