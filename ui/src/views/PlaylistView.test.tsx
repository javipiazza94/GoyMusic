import { fireEvent, screen } from "@testing-library/react";
import { save } from "@tauri-apps/plugin-dialog";
import { beforeEach, expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { ok, track } from "../test/fixtures";
import { renderAt } from "../test/render";

const playlist = { id: 3, name: "Mix", folderId: null, trackCount: 2 };

beforeEach(() => {
  vi.mocked(commands.playlist).mockResolvedValue(
    ok({ playlist, items: [{ position: 0, track: track(1, "Airbag") }, { position: 1, track: track(2, "Paranoid") }] }),
  );
});

const heading = () => screen.findByRole("heading", { level: 1, name: "Mix" });

test("plays the playlist from the clicked track", async () => {
  await (renderAt("/playlist/3"), heading());
  fireEvent.doubleClick(screen.getByText("Paranoid"));
  expect(commands.play).toHaveBeenCalledWith({ kind: "playlist", id: 3 }, 1);
});

test("renames on enter and cancels on escape", async () => {
  const { user } = renderAt("/playlist/3");
  await heading();
  await user.click(screen.getByRole("button", { name: "Renombrar" }));
  const input = screen.getByRole("textbox", { name: "Nombre de la playlist" });
  await user.clear(input);
  await user.type(input, "Mix 2{Enter}");
  expect(commands.renamePlaylist).toHaveBeenCalledWith(3, "Mix 2");
  await user.click(screen.getByRole("button", { name: "Renombrar" }));
  await user.type(screen.getByRole("textbox", { name: "Nombre de la playlist" }), "xyz{Escape}");
  expect(commands.renamePlaylist).toHaveBeenCalledTimes(1);
  expect(screen.queryByRole("textbox", { name: "Nombre de la playlist" })).toBeNull();
});

test("deleting asks first and then goes home", async () => {
  const { user, path } = renderAt("/playlist/3");
  await heading();
  await user.click(screen.getByRole("button", { name: "Eliminar" }));
  expect(commands.deletePlaylist).not.toHaveBeenCalled();
  expect(screen.getByText(/¿Eliminar «Mix»\?/)).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "Eliminar playlist" }));
  expect(commands.deletePlaylist).toHaveBeenCalledWith(3);
  expect(path()).toBe("/");
});

test("cancelling the deletion keeps the playlist", async () => {
  const { user } = renderAt("/playlist/3");
  await heading();
  await user.click(screen.getByRole("button", { name: "Eliminar" }));
  await user.click(screen.getByRole("button", { name: "Cancelar" }));
  expect(commands.deletePlaylist).not.toHaveBeenCalled();
  expect(screen.queryByText(/¿Eliminar «Mix»\?/)).toBeNull();
});

test("exports to the chosen file", async () => {
  vi.mocked(save).mockResolvedValue("/tmp/x.m3u8");
  const { user } = renderAt("/playlist/3");
  await heading();
  await user.click(screen.getByRole("button", { name: "Exportar M3U8" }));
  expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "Mix.m3u8" }));
  expect(commands.exportM3u).toHaveBeenCalledWith(3, "/tmp/x.m3u8");
});

test("moves the playlist into a folder", async () => {
  vi.mocked(commands.playlistFolders).mockResolvedValue(ok([{ id: 1, name: "Ánimo", parentId: null }]));
  const { user } = renderAt("/playlist/3");
  await heading();
  await user.selectOptions(await screen.findByRole("combobox", { name: "Carpeta" }), "Ánimo");
  expect(commands.movePlaylistToFolder).toHaveBeenCalledWith(3, 1);
});

test("a deleted playlist shows not found", async () => {
  vi.mocked(commands.playlist).mockResolvedValue({ status: "error", error: { code: "notFound", message: "playlist 3" } });
  renderAt("/playlist/3");
  expect(await screen.findByRole("heading", { name: "No encontrado" })).toBeTruthy();
});
