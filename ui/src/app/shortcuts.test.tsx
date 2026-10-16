import { screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { commands } from "../bindings";
import { usePlayer } from "../core/player-store";
import { view } from "../test/fixtures";
import { renderAt } from "../test/render";

const playing = () => usePlayer.setState({ view: view({ state: "playing" }) });

test("space toggles playback outside of inputs", async () => {
  playing();
  const { user } = renderAt("/collection/tracks");
  await user.keyboard(" ");
  expect(commands.pause).toHaveBeenCalledTimes(1);
});

test("typing a space in the search box does not pause", async () => {
  playing();
  const { user } = renderAt("/search");
  await user.type(screen.getByRole("searchbox"), "a b");
  expect(commands.pause).not.toHaveBeenCalled();
});

test("ctrl+arrows skip tracks but not inside inputs", async () => {
  playing();
  const { user } = renderAt("/search");
  await user.click(screen.getByRole("searchbox"));
  await user.keyboard("{Control>}{ArrowRight}{/Control}");
  expect(commands.next).not.toHaveBeenCalled();
  (document.activeElement as HTMLElement).blur();
  await user.keyboard("{Control>}{ArrowRight}{/Control}");
  expect(commands.next).toHaveBeenCalledTimes(1);
  await user.keyboard("{Control>}{ArrowLeft}{/Control}");
  expect(commands.previous).toHaveBeenCalledTimes(1);
});

test("ctrl+f opens search and focuses the box", async () => {
  const { user, path } = renderAt("/");
  await user.keyboard("{Control>}f{/Control}");
  expect(path()).toBe("/search");
  expect(document.activeElement).toBe(await screen.findByRole("searchbox"));
});
