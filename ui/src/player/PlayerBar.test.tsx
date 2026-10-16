import { act, fireEvent, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { usePlayer } from "../core/player-store";
import { ok, track, view } from "../test/fixtures";
import { renderAt } from "../test/render";
import { PlayerBar } from "./PlayerBar";

const renderBar = () => renderAt("/", <PlayerBar />);

test("shows the current track and toggles playback", async () => {
  usePlayer.setState({ view: view({ track: track(1, "Levels") }), positionMs: 61_000 });
  const { user } = renderBar();
  expect(screen.getByText("Levels")).toBeTruthy();
  expect(screen.getByText("1:01")).toBeTruthy();
  expect(screen.getByText("3:20")).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "Pausa" }));
  expect(commands.pause).toHaveBeenCalled();
});

test("seeks once when the drag ends", () => {
  usePlayer.setState({ view: view() });
  renderBar();
  const bar = screen.getByRole("slider", { name: "Posición" });
  fireEvent.change(bar, { target: { value: "1000" } });
  fireEvent.change(bar, { target: { value: "2000" } });
  fireEvent.change(bar, { target: { value: "3000" } });
  expect(commands.seek).not.toHaveBeenCalled();
  fireEvent.pointerUp(bar);
  expect(commands.seek).toHaveBeenCalledTimes(1);
  expect(commands.seek).toHaveBeenCalledWith(3000);
});

test("cycles repeat off → all → one → off", async () => {
  usePlayer.setState({ view: view({ repeat: "off" }) });
  const { user } = renderBar();
  await user.click(screen.getByRole("button", { name: "Repetir: no" }));
  expect(commands.setRepeat).toHaveBeenLastCalledWith("all");
  act(() => usePlayer.setState({ view: view({ repeat: "all" }) }));
  await user.click(screen.getByRole("button", { name: "Repetir: todo" }));
  expect(commands.setRepeat).toHaveBeenLastCalledWith("one");
  act(() => usePlayer.setState({ view: view({ repeat: "one" }) }));
  await user.click(screen.getByRole("button", { name: "Repetir: una" }));
  expect(commands.setRepeat).toHaveBeenLastCalledWith("off");
});

test("toggles shuffle", async () => {
  usePlayer.setState({ view: view({ shuffle: false }) });
  const { user } = renderBar();
  const button = screen.getByRole("button", { name: "Aleatorio" });
  expect(button.getAttribute("aria-pressed")).toBe("false");
  await user.click(button);
  expect(commands.setShuffle).toHaveBeenCalledWith(true);
});

test("likes the current track", async () => {
  usePlayer.setState({ view: view() });
  const { user } = renderBar();
  await user.click(await screen.findByRole("button", { name: "Me gusta" }));
  expect(commands.like).toHaveBeenCalledWith(1);
});

test("unlikes a liked track", async () => {
  vi.mocked(commands.likedIds).mockResolvedValue(ok([1]));
  usePlayer.setState({ view: view() });
  const { user } = renderBar();
  const heart = screen.getByRole("button", { name: "Me gusta" });
  await vi.waitFor(() => expect(heart.getAttribute("aria-pressed")).toBe("true"));
  await user.click(heart);
  expect(commands.unlike).toHaveBeenCalledWith(1);
});

test("links the album and a known artist", async () => {
  vi.mocked(commands.artists).mockResolvedValue(ok([{ id: 4, name: "Ana" }]));
  usePlayer.setState({ view: view() });
  renderBar();
  expect(screen.getByRole("link", { name: "Track 1" }).getAttribute("href")).toBe("/album/1");
  expect((await screen.findByRole("link", { name: "Ana" })).getAttribute("href")).toBe("/artist/4");
});

test("a track without album has no album link and shows a placeholder cover", () => {
  usePlayer.setState({ view: view({ track: { ...track(1, "Grabación"), album: null, albumId: null, artists: [] } }) });
  const { container } = renderBar();
  expect(screen.getByText("Grabación")).toBeTruthy();
  expect(screen.queryByRole("link", { name: "Grabación" })).toBeNull();
  expect(container.querySelector("img")).toBeNull();
});

test("changes and mutes the volume", async () => {
  usePlayer.setState({ view: view({ volume: 0.5 }) });
  const { user } = renderBar();
  fireEvent.change(screen.getByRole("slider", { name: "Volumen" }), { target: { value: "0.3" } });
  expect(commands.setVolume).toHaveBeenLastCalledWith(0.3);
  await user.click(screen.getByRole("button", { name: "Silenciar" }));
  expect(commands.setVolume).toHaveBeenLastCalledWith(0);
  await user.click(screen.getByRole("button", { name: "Activar sonido" }));
  expect(commands.setVolume).toHaveBeenLastCalledWith(0.3);
});

test("disables the controls when nothing is loaded", () => {
  usePlayer.setState({ view: view({ track: null, state: "idle" }) });
  renderBar();
  expect(screen.getByText("Elige algo que escuchar")).toBeTruthy();
  expect((screen.getByRole("button", { name: "Reproducir" }) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByRole("button", { name: "Siguiente" }) as HTMLButtonElement).disabled).toBe(true);
});
