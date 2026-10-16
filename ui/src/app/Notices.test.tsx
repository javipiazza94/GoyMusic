import { act, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { usePlayer } from "../core/player-store";
import { reportError } from "../core/report";
import { CommandError } from "../core/result";
import { renderAt } from "../test/render";
import { Notices } from "./Notices";

beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
afterEach(() => vi.useRealTimers());

test("playback errors appear and go away on their own", () => {
  renderAt("/", <Notices />);
  act(() => usePlayer.getState().notify("playbackError", "No se pudo leer pista.flac"));
  expect(screen.getByText("No se pudo leer pista.flac")).toBeTruthy();
  act(() => vi.advanceTimersByTime(6000));
  expect(screen.queryByText("No se pudo leer pista.flac")).toBeNull();
});

test("device lost stays until dismissed", async () => {
  const { user } = renderAt("/", <Notices />);
  act(() => usePlayer.getState().notify("deviceLost", "Se perdió el dispositivo"));
  act(() => vi.advanceTimersByTime(20_000));
  expect(screen.getByRole("alert")).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "Cerrar aviso" }));
  expect(screen.queryByText("Se perdió el dispositivo")).toBeNull();
});

test("grave errors open the error panel", async () => {
  const { user } = renderAt("/", <Notices />);
  act(() => reportError(new CommandError({ code: "database", message: "disk I/O error" })));
  const panel = screen.getByRole("alertdialog", { name: "Algo ha fallado" });
  expect(panel.textContent).toContain("disk I/O error");
  expect(panel.textContent).toContain("~/.local/state/goymusic/logs/");
  await user.click(screen.getByRole("button", { name: "Cerrar" }));
  expect(screen.queryByRole("alertdialog")).toBeNull();
});

test("not found errors are a notice, not the panel", () => {
  renderAt("/", <Notices />);
  act(() => reportError(new CommandError({ code: "notFound", message: "La playlist no existe" })));
  expect(screen.queryByRole("alertdialog")).toBeNull();
  expect(screen.getByText("La playlist no existe")).toBeTruthy();
});

test("unknown failures are grave", () => {
  renderAt("/", <Notices />);
  act(() => reportError(new Error("boom")));
  expect(screen.getByRole("alertdialog").textContent).toContain("boom");
});

test("at most three notices are shown", () => {
  renderAt("/", <Notices />);
  act(() => ["a", "b", "c", "d"].forEach((m) => usePlayer.getState().notify("info", m)));
  expect(screen.queryByText("a")).toBeNull();
  expect(screen.getByText("d")).toBeTruthy();
});
