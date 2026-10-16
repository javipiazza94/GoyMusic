import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { connectCoreEvents } from "./core/events";
import { usePlayer } from "./core/player-store";

const queryClient = new QueryClient();
void connectCoreEvents(queryClient);
void usePlayer.getState().refresh().catch(() => {});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
);
