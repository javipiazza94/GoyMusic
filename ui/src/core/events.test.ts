import { QueryClient } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { usePlayer } from "./player-store";
import { handleCoreEvent } from "./events";

vi.mock("../bindings", () => ({ commands: {}, events: {} }));

let qc: QueryClient;
beforeEach(() => {
  qc = new QueryClient();
  vi.spyOn(qc, "invalidateQueries").mockResolvedValue();
  usePlayer.setState({ view: null, positionMs: 0, scan: null, notices: [], refresh: vi.fn().mockResolvedValue(undefined) });
});
afterEach(() => vi.useRealTimers());

const store = () => usePlayer.getState();

test("libraryUpdated invalidates every library query", () => {
  handleCoreEvent({ type: "libraryUpdated" }, qc, store());
  expect(qc.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["library"] });
});

test("playlist and like changes invalidate their queries", () => {
  handleCoreEvent({ type: "playlistsChanged" }, qc, store());
  handleCoreEvent({ type: "likesChanged" }, qc, store());
  expect(qc.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["playlists"] });
  expect(qc.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["likes"] });
});

test("playerStateChanged and queueChanged refresh the player view", () => {
  handleCoreEvent({ type: "playerStateChanged", state: "playing", track: 1 }, qc, store());
  handleCoreEvent({ type: "queueChanged" }, qc, store());
  expect(store().refresh).toHaveBeenCalledTimes(2);
});

test("playbackError adds a notice and refreshes", () => {
  handleCoreEvent({ type: "playbackError", track: 3, reason: "archivo ilegible" }, qc, store());
  expect(store().notices).toMatchObject([{ kind: "playbackError", message: "archivo ilegible" }]);
  expect(store().refresh).toHaveBeenCalledOnce();
});

test("audioDeviceLost adds a notice", () => {
  handleCoreEvent({ type: "audioDeviceLost", reason: "sin salida" }, qc, store());
  expect(store().notices).toMatchObject([{ kind: "deviceLost", message: "sin salida" }]);
});

test("scanProgress tracks progress and clears when done", () => {
  vi.useFakeTimers();
  handleCoreEvent({ type: "scanProgress", done: 10, total: 20 }, qc, store());
  expect(store().scan).toEqual({ done: 10, total: 20 });
  handleCoreEvent({ type: "scanProgress", done: 20, total: 20 }, qc, store());
  expect(store().scan).toEqual({ done: 20, total: 20 });
  vi.advanceTimersByTime(1_000);
  expect(store().scan).toBeNull();
});

test("positionChanged updates the position of the current track", () => {
  usePlayer.setState({ view: { track: { id: 4 } } as never });
  handleCoreEvent({ type: "positionChanged", track: 4, positionMs: 1_234 }, qc, store());
  expect(store().positionMs).toBe(1_234);
});
