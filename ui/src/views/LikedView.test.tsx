import { fireEvent, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { ok, track } from "../test/fixtures";
import { renderAt } from "../test/render";

test("liked view plays the liked context", async () => {
  vi.mocked(commands.likedTracks).mockResolvedValue(ok([track(1, "Airbag"), track(2, "Paranoid")]));
  renderAt("/collection/tracks");
  fireEvent.doubleClick(await screen.findByText("Airbag"));
  expect(commands.play).toHaveBeenCalledWith({ kind: "liked" }, 0);
});

test("liked view explains how to fill it when empty", async () => {
  renderAt("/collection/tracks");
  expect(await screen.findByText(/aparecen aquí/)).toBeTruthy();
});
