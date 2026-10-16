import { fireEvent, screen, within } from "@testing-library/react";
import { expect, test } from "vitest";
import { commands } from "../bindings";
import { TRACKS_MIME } from "../dnd/drag";
import { usePlayer } from "../core/player-store";
import { fakeDataTransfer } from "../test/drag";
import { track, view } from "../test/fixtures";
import { renderAt } from "../test/render";
import { QueuePanel } from "./QueuePanel";

function renderQueue() {
  usePlayer.setState({
    view: view({ track: track(1, "Now"), userQueue: [track(5, "Levels"), track(6, "Fade")], upNext: [track(9, "Clarity")] }),
  });
  return renderAt("/", <QueuePanel />);
}

test("lists the user queue and up next", () => {
  renderQueue();
  expect(within(screen.getByRole("region", { name: "Sonando" })).getByText("Now")).toBeTruthy();
  expect(within(screen.getByRole("region", { name: "En cola" })).getByText("Levels")).toBeTruthy();
  expect(within(screen.getByRole("region", { name: "A continuación" })).getByText("Clarity")).toBeTruthy();
});

test("removes a queued track", async () => {
  const { user } = renderQueue();
  await user.click(screen.getByRole("button", { name: "Quitar Levels" }));
  expect(commands.queueRemove).toHaveBeenCalledWith(0);
});

test("dropping tracks on the panel queues them", () => {
  renderQueue();
  const panel = screen.getByRole("complementary", { name: "Cola" });
  const dt = fakeDataTransfer({ [TRACKS_MIME]: JSON.stringify({ ids: [7] }) });
  fireEvent.dragOver(panel, { dataTransfer: dt });
  fireEvent.drop(panel, { dataTransfer: dt });
  expect(commands.queueAdd).toHaveBeenCalledWith([7]);
});

test("reorders the user queue by dragging", () => {
  renderQueue();
  const dt = fakeDataTransfer();
  fireEvent.dragStart(screen.getByText("Levels"), { dataTransfer: dt });
  fireEvent.dragOver(screen.getByText("Fade"), { dataTransfer: dt });
  fireEvent.drop(screen.getByText("Fade"), { dataTransfer: dt });
  expect(commands.queueMove).toHaveBeenCalledWith(0, 1);
  expect(commands.queueAdd).not.toHaveBeenCalled();
});

test("an empty queue explains how to fill it", () => {
  usePlayer.setState({ view: view({ userQueue: [] }) });
  renderAt("/", <QueuePanel />);
  expect(screen.getByText(/Arrastra canciones aquí/)).toBeTruthy();
});

test("alt+arrows reorder the user queue from the keyboard", async () => {
  const { user } = renderQueue();
  const item = screen.getByText("Levels").closest("li") as HTMLElement;
  item.focus();
  await user.keyboard("{Alt>}{ArrowDown}{/Alt}");
  expect(commands.queueMove).toHaveBeenCalledWith(0, 1);
});
