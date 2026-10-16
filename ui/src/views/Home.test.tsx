import { fireEvent, screen } from "@testing-library/react";
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
  expect(commands.play).toHaveBeenCalledWith({ kind: "tracks", id: [5, 9] }, 1);
});

test("home remembers the chosen tab", async () => {
  vi.mocked(commands.folders).mockResolvedValue(ok([folder]));
  vi.mocked(commands.albums).mockResolvedValue(ok([album]));
  const { user } = renderAt("/");
  await user.click(await screen.findByRole("tab", { name: "Artistas" }));
  expect(localStorage.getItem("goymusic.homeTab")).toBe("artists");
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
