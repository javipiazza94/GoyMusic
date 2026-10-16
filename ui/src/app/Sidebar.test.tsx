import { screen, within } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { open } from "@tauri-apps/plugin-dialog";
import { commands } from "../bindings";
import { ok } from "../test/fixtures";
import { renderAt } from "../test/render";

const playlist = (id: number, name: string, folderId: number | null) => ({ id, name, folderId, trackCount: 0 });

test("marks the current section in the sidebar", async () => {
  renderAt("/collection/tracks");
  const nav = screen.getByRole("navigation", { name: "Principal" });
  expect(within(nav).getByRole("link", { name: "Me gusta" }).getAttribute("aria-current")).toBe("page");
  expect(within(nav).getByRole("link", { name: "Inicio" }).getAttribute("aria-current")).toBeNull();
});

test("groups playlists under their folders", async () => {
  vi.mocked(commands.playlistFolders).mockResolvedValue(ok([{ id: 1, name: "Ánimo", parentId: null }]));
  vi.mocked(commands.playlists).mockResolvedValue(ok([playlist(3, "Para escribir", 1), playlist(4, "Carretera", null)]));
  renderAt("/");
  const folder = await screen.findByRole("group", { name: "Ánimo" });
  expect(within(folder).getByRole("link", { name: "Para escribir" })).toBeTruthy();
  expect(within(folder).queryByRole("link", { name: "Carretera" })).toBeNull();
  expect(screen.getByRole("link", { name: "Carretera" })).toBeTruthy();
});

test("creates a playlist and opens it", async () => {
  vi.mocked(commands.createPlaylist).mockResolvedValue(ok(playlist(9, "Nueva playlist", null)));
  const { user, path } = renderAt("/");
  await user.click(screen.getByRole("button", { name: "Nueva playlist" }));
  expect(commands.createPlaylist).toHaveBeenCalledWith("Nueva playlist");
  await vi.waitFor(() => expect(path()).toBe("/playlist/9"));
});

test("creates a playlist folder", async () => {
  const { user } = renderAt("/");
  await user.click(screen.getByRole("button", { name: "Nueva carpeta" }));
  expect(commands.createPlaylistFolder).toHaveBeenCalledWith("Nueva carpeta", null);
});

test("imports an M3U8 file and opens the playlist", async () => {
  vi.mocked(open).mockResolvedValue("/tmp/lista.m3u8");
  vi.mocked(commands.importM3u).mockResolvedValue(ok({ playlist: playlist(5, "lista", null), matched: 2, missing: [] }));
  const { user, path } = renderAt("/");
  await user.click(screen.getByRole("button", { name: "Importar M3U8" }));
  expect(commands.importM3u).toHaveBeenCalledWith("/tmp/lista.m3u8");
  await vi.waitFor(() => expect(path()).toBe("/playlist/5"));
});
