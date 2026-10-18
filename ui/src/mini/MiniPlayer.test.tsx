import { screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { usePlayer } from "../core/player-store";
import { PlayerBar } from "../player/PlayerBar";
import { track, view } from "../test/fixtures";
import { renderAt } from "../test/render";
import { MiniPlayer } from "./MiniPlayer";

test("the mini player shows the track and toggles playback", async () => {
  usePlayer.setState({ view: view({ track: track(1, "Faro") }) });
  const { user } = renderAt("/", <MiniPlayer />);
  expect(screen.getByText("Faro")).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "Pausa" }));
  expect(commands.pause).toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Siguiente" }));
  expect(commands.next).toHaveBeenCalled();
});

test("pin on top toggles and calls the core", async () => {
  usePlayer.setState({ view: view() });
  const { user } = renderAt("/", <MiniPlayer />);
  const pin = await screen.findByRole("button", { name: "Fijar encima" });
  expect(pin.getAttribute("aria-pressed")).toBe("false");
  await user.click(pin);
  expect(commands.setMiniOnTop).toHaveBeenLastCalledWith(true);
  expect(pin.getAttribute("aria-pressed")).toBe("true");
  await user.click(pin);
  expect(commands.setMiniOnTop).toHaveBeenLastCalledWith(false);
});

test("the pin is hidden when the window cannot stay on top", async () => {
  vi.mocked(commands.miniCanStayOnTop).mockResolvedValue(false);
  usePlayer.setState({ view: view() });
  renderAt("/", <MiniPlayer />);
  await screen.findByRole("button", { name: "Abrir GoyMusic" });
  await vi.waitFor(() => expect(commands.miniCanStayOnTop).toHaveBeenCalled());
  await vi.waitFor(() => expect(screen.queryByRole("button", { name: "Fijar encima" })).toBeNull());
});

test("open goes back to the main window and close hides the mini player", async () => {
  usePlayer.setState({ view: view() });
  const { user } = renderAt("/", <MiniPlayer />);
  await user.click(screen.getByRole("button", { name: "Abrir GoyMusic" }));
  expect(commands.showMainWindow).toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Cerrar mini reproductor" }));
  expect(commands.hideMiniPlayer).toHaveBeenCalled();
});

test("the player bar opens the mini player", async () => {
  usePlayer.setState({ view: view() });
  const { user } = renderAt("/", <PlayerBar />);
  await user.click(screen.getByRole("button", { name: "Mini reproductor" }));
  expect(commands.showMiniPlayer).toHaveBeenCalled();
});
