import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";
import { usePlayer } from "../core/player-store";
import { installMatchMedia, resetMatchMedia } from "./media";
import { resetCommandMocks } from "./mock-bindings";

vi.mock("../bindings", () => import("./mock-bindings"));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));

class NoopResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= NoopResizeObserver as unknown as typeof ResizeObserver;
installMatchMedia();

const initialPlayer = usePlayer.getState();

beforeEach(() => {
  resetCommandMocks();
  resetMatchMedia();
  localStorage.clear();
  delete document.documentElement.dataset.theme;
  usePlayer.setState({ ...initialPlayer, view: null, positionMs: 0, scan: null, notices: [] }, true);
});

afterEach(cleanup);
