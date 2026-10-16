import { screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { usePlayer } from "../core/player-store";
import { ok } from "../test/fixtures";
import { renderAt } from "../test/render";

test("settings removes a folder and switches theme", async () => {
  vi.mocked(commands.folders).mockResolvedValue(ok([{ id: 1, path: "/home/javi/Música" }]));
  const { user } = renderAt("/settings");
  await user.click(await screen.findByRole("button", { name: "Quitar /home/javi/Música" }));
  expect(commands.removeFolder).toHaveBeenCalledWith(1);
  await user.click(screen.getByRole("radio", { name: "Claro" }));
  expect(document.documentElement.dataset.theme).toBe("light");
});

test("settings rescans and shows the progress", async () => {
  usePlayer.setState({ scan: { done: 3, total: 10 } });
  const { user } = renderAt("/settings");
  expect(screen.getByText("Escaneando… 3/10")).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "Reescanear" }));
  expect(commands.rescan).toHaveBeenCalled();
});

test("settings toggles ReplayGain", async () => {
  const { user } = renderAt("/settings");
  const toggle = await screen.findByRole("switch", { name: "ReplayGain" });
  expect(toggle.getAttribute("aria-checked")).toBe("false");
  await user.click(toggle);
  expect(commands.setReplaygain).toHaveBeenCalledWith(true);
});

test("settings shows when folders are only polled", async () => {
  vi.mocked(commands.settings).mockResolvedValue(ok({ replaygain: false, watch: { mode: "polling", reason: "inotify" } }));
  renderAt("/settings");
  expect(await screen.findByText(/cada 10 min/)).toBeTruthy();
  expect(screen.getByText(/inotify/)).toBeTruthy();
});
