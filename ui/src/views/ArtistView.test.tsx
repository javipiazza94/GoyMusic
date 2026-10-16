import { fireEvent, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { ok, track } from "../test/fixtures";
import { renderAt } from "../test/render";

test("artist view lists albums and songs", async () => {
  vi.mocked(commands.artist).mockResolvedValue(
    ok({
      artist: { id: 2, name: "Radiohead" },
      albums: [{ id: 7, title: "OK Computer", artist: "Radiohead", year: 1997, trackCount: 12 }],
      tracks: [track(1, "Airbag"), track(2, "Paranoid")],
    }),
  );
  const { user } = renderAt("/artist/2");
  expect(await screen.findByRole("heading", { level: 1, name: "Radiohead" })).toBeTruthy();
  expect(screen.getByRole("link", { name: /OK Computer/ }).getAttribute("href")).toBe("/album/7");
  fireEvent.doubleClick(screen.getByText("Paranoid"));
  expect(commands.play).toHaveBeenCalledWith({ kind: "artist", id: 2 }, 1);
  await user.click(screen.getByRole("button", { name: "Reproducir Radiohead" }));
  expect(commands.play).toHaveBeenLastCalledWith({ kind: "artist", id: 2 }, 0);
});
