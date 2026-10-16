import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import App from "./App";
import { commands } from "./bindings";
import { formatMs } from "./core/queries";
import { usePlayer } from "./core/player-store";
import { ok } from "./test/fixtures";

vi.mock("./bindings", () => ({
  commands: { albums: vi.fn(), folders: vi.fn(), play: vi.fn(), addFolder: vi.fn(), previous: vi.fn(), next: vi.fn() },
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

beforeEach(() => {
  vi.mocked(commands.albums).mockResolvedValue(
    ok([{ id: 7, title: "OK Computer", artist: "Radiohead", year: 1997, trackCount: 12 }]),
  );
  vi.mocked(commands.folders).mockResolvedValue(ok([{ id: 1, path: "/home/javi/Música" }]));
  vi.mocked(commands.play).mockResolvedValue(ok(null));
  usePlayer.setState({ view: null, positionMs: 0, scan: null, notices: [] });
});

function renderApp() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <App />
    </QueryClientProvider>,
  );
}

test("shows albums and plays one on click", async () => {
  renderApp();
  expect(await screen.findByText("/home/javi/Música")).toBeTruthy();
  fireEvent.click(await screen.findByRole("button", { name: /OK Computer/ }));
  expect(commands.play).toHaveBeenCalledWith({ kind: "album", id: 7 }, 0);
});

test("shows scan progress while scanning", async () => {
  usePlayer.setState({ scan: { done: 3, total: 10 } });
  renderApp();
  expect(await screen.findByText("Escaneando… 3/10")).toBeTruthy();
});

test("formatMs formats minutes and seconds", () => {
  expect(formatMs(0)).toBe("0:00");
  expect(formatMs(61_000)).toBe("1:01");
  expect(formatMs(3_600_000)).toBe("60:00");
});
