import { fireEvent, screen, within } from "@testing-library/react";
import { open } from "@tauri-apps/plugin-dialog";
import { expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { ok, track } from "../test/fixtures";
import { renderAt } from "../test/render";

const folder = { id: 1, path: "/home/javi/Música" };
const album = { id: 7, title: "OK Computer", artist: "Radiohead", year: 1997, trackCount: 12 };

test("home shows albums and plays one from its page", async () => {
  vi.mocked(commands.folders).mockResolvedValue(ok([folder]));
  vi.mocked(commands.albums).mockResolvedValue(ok([album]));
  const { user } = renderAt("/");
  expect((await screen.findByRole("tab", { name: "Álbumes" })).getAttribute("aria-selected")).toBe("true");
  expect(screen.getByRole("link", { name: /OK Computer/ }).getAttribute("href")).toBe("/album/7");
  await user.click(screen.getByRole("button", { name: "Reproducir OK Computer" }));
  expect(commands.play).toHaveBeenCalledWith({ kind: "album", id: 7 }, 0);
});

test("home falls back to songs when there are no albums", async () => {
  vi.mocked(commands.folders).mockResolvedValue(ok([folder]));
  vi.mocked(commands.tracks).mockResolvedValue(ok([track(5, "Levels"), track(9, "Clarity")]));
  renderAt("/");
  expect((await screen.findByRole("tab", { name: "Canciones" })).getAttribute("aria-selected")).toBe("true");
  fireEvent.doubleClick(await screen.findByText("Clarity"));
  // the whole library in the shown order, not only the rows loaded so far
  expect(commands.play).toHaveBeenCalledWith({ kind: "library", id: "title" }, 1);
});

test("home keeps the chosen tab in the url and remembers it", async () => {
  vi.mocked(commands.folders).mockResolvedValue(ok([folder]));
  vi.mocked(commands.albums).mockResolvedValue(ok([album]));
  const { user, path } = renderAt("/");
  await user.click(await screen.findByRole("tab", { name: "Artistas" }));
  expect(path()).toBe("/?vista=artistas");
  expect(localStorage.getItem("goymusic.homeTab")).toBe("artists");
});

test("home opens the tab and order named in the url", async () => {
  vi.mocked(commands.folders).mockResolvedValue(ok([folder]));
  vi.mocked(commands.albums).mockResolvedValue(ok([album]));
  renderAt("/?vista=canciones&orden=recentlyAdded");
  expect((await screen.findByRole("tab", { name: "Canciones" })).getAttribute("aria-selected")).toBe("true");
  await vi.waitFor(() => expect(commands.tracks).toHaveBeenLastCalledWith("recentlyAdded", 0, 500));
});

test("songs can be sorted", async () => {
  vi.mocked(commands.folders).mockResolvedValue(ok([folder]));
  const { user } = renderAt("/");
  await user.selectOptions(await screen.findByRole("combobox", { name: "Ordenar por" }), "recentlyAdded");
  expect(commands.tracks).toHaveBeenLastCalledWith("recentlyAdded", 0, 500);
});

test("home invites to add a folder when the library is empty", async () => {
  vi.mocked(open).mockResolvedValue("/home/javi/Música");
  const { user } = renderAt("/");
  expect(await screen.findByText("Tu biblioteca está vacía")).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "Añadir carpeta" }));
  expect(commands.addFolder).toHaveBeenCalledWith("/home/javi/Música");
});

test("home shows your playlists as colour tiles", async () => {
  vi.mocked(commands.folders).mockResolvedValue(ok([folder]));
  vi.mocked(commands.playlists).mockResolvedValue(ok([{ id: 3, name: "Carretera", folderId: null, trackCount: 7 }]));
  renderAt("/");
  const section = await screen.findByRole("region", { name: "Tus playlists" });
  expect(within(section).getByRole("link", { name: /Carretera/ }).getAttribute("href")).toBe("/playlist/3");
  expect(within(section).getByRole("link", { name: /Me gusta/ }).getAttribute("href")).toBe("/collection/tracks");
});
