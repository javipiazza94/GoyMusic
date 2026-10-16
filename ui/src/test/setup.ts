import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";
import { usePlayer } from "../core/player-store";
import { installMatchMedia, resetMatchMedia } from "./media";
import { resetCommandMocks } from "./mock-bindings";
import { useUi } from "../app/ui-store";

vi.mock("../bindings", () => import("./mock-bindings"));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));

class NoopResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= NoopResizeObserver as unknown as typeof ResizeObserver;
installMatchMedia();
// jsdom has no layout; virtualized lists measure their scroll element with these.
Object.defineProperty(HTMLElement.prototype, "offsetHeight", { configurable: true, get: () => 768 });
Object.defineProperty(HTMLElement.prototype, "offsetWidth", { configurable: true, get: () => 1024 });

const initialPlayer = usePlayer.getState();

beforeEach(() => {
  resetCommandMocks();
  resetMatchMedia();
  localStorage.clear();
  delete document.documentElement.dataset.theme;
  useUi.setState({ queueOpen: false });
  usePlayer.setState({ ...initialPlayer, view: null, positionMs: 0, scan: null, notices: [] }, true);
});

afterEach(cleanup);
