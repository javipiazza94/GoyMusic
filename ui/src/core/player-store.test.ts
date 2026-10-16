import { beforeEach, expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { ok, view } from "../test/fixtures";
import { usePlayer } from "./player-store";


beforeEach(() => {
  vi.mocked(commands.pause).mockResolvedValue(ok(null));
  vi.mocked(commands.resume).mockResolvedValue(ok(null));
  vi.mocked(commands.setVolume).mockResolvedValue(ok(null));
  usePlayer.setState({ view: view(), positionMs: 0, scan: null, notices: [] });
});

test("setVolume updates the view before the command resolves", () => {
  vi.mocked(commands.setVolume).mockReturnValue(new Promise(() => {}));
  void usePlayer.getState().setVolume(0.3);
  expect(usePlayer.getState().view?.volume).toBe(0.3);
  expect(commands.setVolume).toHaveBeenCalledWith(0.3);
});

test("togglePlay pauses while playing and resumes otherwise", async () => {
  await usePlayer.getState().togglePlay();
  expect(commands.pause).toHaveBeenCalledOnce();
  usePlayer.setState({ view: view({ state: "paused" }) });
  await usePlayer.getState().togglePlay();
  expect(commands.resume).toHaveBeenCalledOnce();
});

test("positionChanged for another track is ignored", () => {
  usePlayer.getState().setPosition(1, 5_000);
  expect(usePlayer.getState().positionMs).toBe(5_000);
  usePlayer.getState().setPosition(2, 9_000);
  expect(usePlayer.getState().positionMs).toBe(5_000);
});

test("refresh loads the view and its position", async () => {
  vi.mocked(commands.playerView).mockResolvedValue(ok(view({ positionMs: 1_500, state: "paused" })));
  await usePlayer.getState().refresh();
  expect(usePlayer.getState().view?.state).toBe("paused");
  expect(usePlayer.getState().positionMs).toBe(1_500);
});

test("a stale playerView answer never overwrites a newer one", async () => {
  let answerOld!: (v: unknown) => void;
  vi.mocked(commands.playerView)
    .mockReturnValueOnce(new Promise((r) => (answerOld = r)) as never)
    .mockResolvedValueOnce(ok(view({ state: "playing", positionMs: 2_000 })));
  const older = usePlayer.getState().refresh();
  await usePlayer.getState().refresh();
  answerOld(ok(view({ state: "loading", positionMs: 0 })));
  await older;
  expect(usePlayer.getState().view?.state).toBe("playing");
  expect(usePlayer.getState().positionMs).toBe(2_000);
});
