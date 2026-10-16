import { fireEvent, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { fakeDataTransfer } from "../test/drag";
import { ok, track } from "../test/fixtures";
import { renderAt } from "../test/render";
import { readDrag, TRACKS_MIME, writeDrag } from "./drag";

const mix = { id: 3, name: "Mix", folderId: null, trackCount: 0 };

test("drag data round-trips and foreign data is ignored", () => {
  const dt = fakeDataTransfer();
  writeDrag(dt as unknown as DataTransfer, { ids: [5, 9] });
  expect(readDrag(dt as unknown as DataTransfer)).toEqual({ ids: [5, 9] });
  expect(readDrag(fakeDataTransfer({ "text/plain": "hola" }) as unknown as DataTransfer)).toBeNull();
  expect(readDrag(fakeDataTransfer({ [TRACKS_MIME]: "{nope" }) as unknown as DataTransfer)).toBeNull();
});

test("dropping tracks on a sidebar playlist adds them", async () => {
  vi.mocked(commands.playlists).mockResolvedValue(ok([mix]));
  renderAt("/");
  const target = await screen.findByRole("link", { name: "Mix" });
  const dt = fakeDataTransfer({ [TRACKS_MIME]: JSON.stringify({ ids: [5, 9] }) });
  fireEvent.dragOver(target, { dataTransfer: dt });
  fireEvent.drop(target, { dataTransfer: dt });
  expect(commands.addToPlaylist).toHaveBeenCalledWith(3, [5, 9]);
});

test("dropping tracks on Me gusta likes them", () => {
  renderAt("/");
  const dt = fakeDataTransfer({ [TRACKS_MIME]: JSON.stringify({ ids: [5] }) });
  fireEvent.drop(screen.getByRole("link", { name: "Me gusta" }), { dataTransfer: dt });
  expect(commands.like).toHaveBeenCalledWith(5);
});

test("drops without our data are ignored", async () => {
  vi.mocked(commands.playlists).mockResolvedValue(ok([mix]));
  renderAt("/");
  const target = await screen.findByRole("link", { name: "Mix" });
  fireEvent.drop(target, { dataTransfer: fakeDataTransfer({ "text/plain": "x" }) });
  expect(commands.addToPlaylist).not.toHaveBeenCalled();
});

test("dragging a row carries its track, or the whole selection", async () => {
  vi.mocked(commands.playlist).mockResolvedValue(
    ok({ playlist: mix, items: [0, 1, 2].map((p) => ({ position: p, track: track(p + 1, `Song ${p + 1}`) })) }),
  );
  const { user } = renderAt("/playlist/3");
  const row = async (n: number) => (await screen.findByText(`Song ${n}`)).closest('[role="row"]') as HTMLElement;
  const single = fakeDataTransfer();
  fireEvent.dragStart(await row(2), { dataTransfer: single });
  expect(JSON.parse(single.data[TRACKS_MIME]).ids).toEqual([2]);
  await user.click(await row(1));
  await user.keyboard("{Shift>}");
  await user.click(await row(3));
  await user.keyboard("{/Shift}");
  const many = fakeDataTransfer();
  fireEvent.dragStart(await row(2), { dataTransfer: many });
  expect(JSON.parse(many.data[TRACKS_MIME]).ids).toEqual([1, 2, 3]);
});

test("reordering inside a playlist moves the item", async () => {
  vi.mocked(commands.playlist).mockResolvedValue(
    ok({ playlist: mix, items: [0, 1, 2].map((p) => ({ position: p, track: track(p + 1, `Song ${p + 1}`) })) }),
  );
  renderAt("/playlist/3");
  const row = async (n: number) => (await screen.findByText(`Song ${n}`)).closest('[role="row"]') as HTMLElement;
  const dt = fakeDataTransfer();
  fireEvent.dragStart(await row(1), { dataTransfer: dt });
  fireEvent.dragOver(await row(3), { dataTransfer: dt });
  fireEvent.drop(await row(3), { dataTransfer: dt });
  expect(commands.movePlaylistItem).toHaveBeenCalledWith(3, 0, 2);
});

test("the playlist menu removes the selected items", async () => {
  vi.mocked(commands.playlist).mockResolvedValue(
    ok({ playlist: mix, items: [0, 1].map((p) => ({ position: p, track: track(p + 1, `Song ${p + 1}`) })) }),
  );
  const { user } = renderAt("/playlist/3");
  fireEvent.contextMenu(await screen.findByText("Song 2"));
  await user.click(screen.getByRole("menuitem", { name: "Quitar de la playlist" }));
  expect(commands.removeFromPlaylist).toHaveBeenCalledWith(3, [1]);
});
