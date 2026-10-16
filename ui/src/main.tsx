import "./styles/base.css";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { initTheme } from "./core/theme";

const mock = import.meta.env.MODE === "mock";

async function start() {
  // The mock must replace Tauri's IPC before anything talks to the core.
  if (mock) await import("./dev/mock-ipc");
  const { connectCoreEvents } = await import("./core/events");
  const { usePlayer } = await import("./core/player-store");

  initTheme();
  const queryClient = new QueryClient();
  void connectCoreEvents(queryClient);
  void usePlayer.getState().refresh().catch(() => {});

  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <App />
        {mock && <MockBadge />}
      </QueryClientProvider>
    </StrictMode>,
  );
}

function MockBadge() {
  return (
    <div
      style={{
        position: "fixed", top: 8, right: 8, zIndex: 100, padding: "2px 8px", borderRadius: "var(--radius-pill)",
        font: "var(--text-xs) var(--font-mono)", background: "var(--color-paper-3)", color: "var(--color-ink-2)",
      }}
    >
      MODO MOCK · datos inventados
    </div>
  );
}

void start();
