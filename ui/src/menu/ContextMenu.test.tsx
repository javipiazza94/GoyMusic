import { fireEvent, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { commands, type Track } from "../bindings";
import { ok, track } from "../test/fixtures";
import { renderAt } from "../test/render";
import { TrackList, type TrackRow } from "../tracks/TrackList";
import { ContextMenu } from "./ContextMenu";

const rows = (tracks: Track[]): TrackRow[] => tracks.map((t) => ({ track: t, key: String(t.id) }));

function renderList(tracks: Track[]) {
  return renderAt(
    "/",
    <>
      <TrackList label="Canciones" rows={rows(tracks)} onPlay={() => {}} />
      <ContextMenu />
    </>,
  );
}

test("right click opens the track menu and adds to the queue", async () => {
  const { user } = renderList([track(5, "Levels")]);
  fireEvent.contextMenu(screen.getByText("Levels"), { clientX: 10, clientY: 10 });
  await user.click(screen.getByRole("menuitem", { name: "Añadir a la cola" }));
  expect(commands.queueAdd).toHaveBeenCalledWith([5]);
  expect(screen.queryByRole("menu")).toBeNull();
});

test("adds the selection to a playlist from the submenu", async () => {
  vi.mocked(commands.playlists).mockResolvedValue(ok([{ id: 3, name: "Mix", folderId: null, trackCount: 0 }]));
  const { user } = renderList([track(5, "Levels"), track(9, "Clarity")]);
  await user.click(screen.getByText("Levels"));
  await user.keyboard("{Shift>}");
  await user.click(screen.getByText("Clarity"));
  await user.keyboard("{/Shift}");
  fireEvent.contextMenu(screen.getByText("Clarity"));
  await user.click(screen.getByRole("menuitem", { name: "Añadir a playlist" }));
  await user.click(await screen.findByRole("menuitem", { name: "Mix" }));
  expect(commands.addToPlaylist).toHaveBeenCalledWith(3, [5, 9]);
});

test("go to album is disabled for tracks without album", () => {
  renderList([{ ...track(1, "Grabación"), album: null, albumId: null, artists: [] }]);
  fireEvent.contextMenu(screen.getByText("Grabación"));
  expect(screen.getByRole("menuitem", { name: "Ir al álbum" }).getAttribute("aria-disabled")).toBe("true");
  expect(screen.getByRole("menuitem", { name: "Ir al artista" }).getAttribute("aria-disabled")).toBe("true");
});

test("go to album navigates", async () => {
  const { user, path } = renderList([track(5, "Levels")]);
  fireEvent.contextMenu(screen.getByText("Levels"));
  await user.click(screen.getByRole("menuitem", { name: "Ir al álbum" }));
  expect(path()).toBe("/album/1");
});

test("escape closes the menu and returns focus to the row", async () => {
  const { user } = renderList([track(5, "Levels")]);
  const row = screen.getByText("Levels").closest('[role="row"]') as HTMLElement;
  row.focus();
  await user.keyboard("{Shift>}{F10}{/Shift}");
  expect(screen.getByRole("menu")).toBeTruthy();
  expect(document.activeElement?.getAttribute("role")).toBe("menuitem");
  await user.keyboard("{Escape}");
  expect(screen.queryByRole("menu")).toBeNull();
  expect(document.activeElement).toBe(row);
});

test("arrow keys move between menu items", async () => {
  const { user } = renderList([track(5, "Levels")]);
  fireEvent.contextMenu(screen.getByText("Levels"));
  expect(document.activeElement?.textContent).toBe("Añadir a la cola");
  await user.keyboard("{ArrowDown}");
  expect(document.activeElement?.textContent).toContain("Añadir a playlist");
});
