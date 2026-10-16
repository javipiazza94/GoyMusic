import { act, fireEvent, screen, within } from "@testing-library/react";
import { useState } from "react";
import { expect, test, vi } from "vitest";
import type { Track } from "../bindings";
import { usePlayer } from "../core/player-store";
import { track, view } from "../test/fixtures";
import { renderAt } from "../test/render";
import { TrackList, type TrackRow } from "./TrackList";

const rowsOf = (n: number): TrackRow[] =>
  Array.from({ length: n }, (_, i) => ({ track: track(i + 1, `Song ${i + 1}`), key: String(i + 1) }));

const bodyRows = () => screen.getAllByRole("row").filter((r) => r.getAttribute("aria-rowindex") !== "1");

test("renders only the visible rows of a long list", () => {
  renderAt("/", <TrackList label="Canciones" rows={rowsOf(50_000)} onPlay={() => {}} />);
  const grid = screen.getByRole("grid", { name: "Canciones" });
  expect(grid.getAttribute("aria-rowcount")).toBe("50001");
  expect(bodyRows().length).toBeGreaterThan(5);
  expect(bodyRows().length).toBeLessThan(100);
});

test("double click plays from that row", () => {
  const onPlay = vi.fn();
  renderAt("/", <TrackList label="Canciones" rows={rowsOf(10)} onPlay={onPlay} />);
  fireEvent.doubleClick(screen.getByText("Song 4"));
  expect(onPlay).toHaveBeenCalledWith(3);
});

test("enter plays the focused row and arrows move focus", async () => {
  const onPlay = vi.fn();
  const { user } = renderAt("/", <TrackList label="Canciones" rows={rowsOf(10)} onPlay={onPlay} />);
  await user.click(screen.getByText("Song 1"));
  await user.keyboard("{ArrowDown}{ArrowDown}{ArrowUp}{Enter}");
  expect(onPlay).toHaveBeenCalledWith(1);
  expect(document.activeElement?.textContent).toContain("Song 2");
});

test("clicks select rows", async () => {
  const { user } = renderAt("/", <TrackList label="Canciones" rows={rowsOf(10)} onPlay={() => {}} />);
  await user.click(screen.getByText("Song 2"));
  await user.keyboard("{Shift>}");
  await user.click(screen.getByText("Song 4"));
  await user.keyboard("{/Shift}");
  const selected = bodyRows().filter((r) => r.getAttribute("aria-selected") === "true");
  expect(selected.map((r) => within(r).getByText(/Song/).textContent)).toEqual(["Song 2", "Song 3", "Song 4"]);
});

test("marks the track that is playing", () => {
  usePlayer.setState({ view: view({ track: track(2, "Song 2") }) });
  renderAt("/", <TrackList label="Canciones" rows={rowsOf(5)} onPlay={() => {}} />);
  const current = bodyRows().filter((r) => r.getAttribute("aria-current") === "true");
  expect(current).toHaveLength(1);
  expect(within(current[0]).getByText("Song 2")).toBeTruthy();
});

test("asks for more rows near the end", () => {
  const onEndReached = vi.fn();
  renderAt("/", <TrackList label="Canciones" rows={rowsOf(200)} onPlay={() => {}} onEndReached={onEndReached} />);
  expect(onEndReached).not.toHaveBeenCalled();
  const scroller = screen.getByRole("grid", { name: "Canciones" }).parentElement!;
  Object.defineProperty(scroller, "scrollTop", { configurable: true, value: 200 * 52 });
  fireEvent.scroll(scroller);
  expect(onEndReached).toHaveBeenCalled();
});

test("rows without album or artists still render", () => {
  const loose: Track = { ...track(1, "Grabación"), album: null, albumId: null, artists: [], trackNo: null };
  renderAt("/", <TrackList label="Canciones" rows={[{ track: loose, key: "1" }]} onPlay={() => {}} numbering="trackNo" />);
  expect(screen.getByText("Grabación")).toBeTruthy();
  expect(screen.queryByRole("link")).toBeNull();
});

test("links the album of a row", () => {
  renderAt("/", <TrackList label="Canciones" rows={rowsOf(1)} onPlay={() => {}} />);
  expect(screen.getByRole("link", { name: "Album" }).getAttribute("href")).toBe("/album/1");
});

test("end moves focus to the last row even when it was not rendered yet", async () => {
  const { user } = renderAt("/", <TrackList label="Canciones" rows={rowsOf(200)} onPlay={() => {}} />);
  await user.click(screen.getByText("Song 1"));
  await user.keyboard("{End}");
  const scroller = screen.getByRole("grid", { name: "Canciones" }).parentElement!;
  Object.defineProperty(scroller, "scrollTop", { configurable: true, value: 200 * 52 });
  fireEvent.scroll(scroller);
  await vi.waitFor(() => expect(document.activeElement?.textContent).toContain("Song 200"));
});

let setRows: (rows: TrackRow[]) => void = () => {};
function Harness({ initial }: { initial: TrackRow[] }) {
  const [rows, set] = useState(initial);
  setRows = set;
  return <TrackList label="Canciones" rows={rows} onPlay={() => {}} />;
}

test("the selection follows the rows when more of them load or they are reordered", async () => {
  const first = rowsOf(10);
  const { user } = renderAt("/", <Harness initial={first} />);
  await user.click(screen.getByText("Song 2"));
  await user.keyboard("{Shift>}");
  await user.click(screen.getByText("Song 3"));
  await user.keyboard("{/Shift}");
  const selected = () =>
    bodyRows()
      .filter((r) => r.getAttribute("aria-selected") === "true")
      .map((r) => within(r).getByText(/Song/).textContent)
      .sort();
  act(() => setRows(rowsOf(20)));
  expect(selected()).toEqual(["Song 2", "Song 3"]);
  act(() => setRows([...first].reverse()));
  expect(selected()).toEqual(["Song 2", "Song 3"]);
});

test("alt+arrows reorder a row from the keyboard", async () => {
  const onReorder = vi.fn();
  const { user } = renderAt("/", <TrackList label="Canciones" rows={rowsOf(5)} onPlay={() => {}} onReorder={onReorder} />);
  await user.click(screen.getByText("Song 2"));
  await user.keyboard("{Alt>}{ArrowDown}{/Alt}");
  expect(onReorder).toHaveBeenLastCalledWith(1, 2);
  await user.keyboard("{Alt>}{ArrowUp}{/Alt}");
  expect(onReorder).toHaveBeenLastCalledWith(2, 1);
});

test("lists that cannot be reordered ignore alt+arrows", async () => {
  const { user } = renderAt("/", <TrackList label="Canciones" rows={rowsOf(5)} onPlay={() => {}} />);
  await user.click(screen.getByText("Song 2"));
  await user.keyboard("{Alt>}{ArrowDown}{/Alt}");
  expect(document.activeElement?.textContent).toContain("Song 2");
});
