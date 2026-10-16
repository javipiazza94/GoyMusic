import { fireEvent, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { ok, track } from "../test/fixtures";
import { renderAt } from "../test/render";

const album = { id: 7, title: "OK Computer", artist: "Radiohead", year: 1997, trackCount: 3 };

test("album view plays the album from the clicked track", async () => {
  vi.mocked(commands.album).mockResolvedValue(ok({ album, tracks: [track(1, "Airbag"), track(2, "Paranoid"), track(3, "Subterranean")] }));
  const { user } = renderAt("/album/7");
  expect(await screen.findByRole("heading", { level: 1, name: "OK Computer" })).toBeTruthy();
  expect(screen.getByText(/3 canciones/)).toBeTruthy();
  fireEvent.doubleClick(screen.getByText("Subterranean"));
  expect(commands.play).toHaveBeenCalledWith({ kind: "album", id: 7 }, 2);
  await user.click(screen.getByRole("button", { name: "Reproducir OK Computer" }));
  expect(commands.play).toHaveBeenLastCalledWith({ kind: "album", id: 7 }, 0);
});

test("an album that no longer exists shows not found", async () => {
  vi.mocked(commands.album).mockResolvedValue({ status: "error", error: { code: "notFound", message: "album 7" } });
  renderAt("/album/7");
  expect(await screen.findByRole("heading", { name: "No encontrado" })).toBeTruthy();
});
