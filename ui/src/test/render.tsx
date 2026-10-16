import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, type RenderResult } from "@testing-library/react";
import userEvent, { type UserEvent } from "@testing-library/user-event";
import type { ReactNode } from "react";
import { MemoryRouter, useLocation } from "react-router";
import { AppShell } from "../app/AppShell";

let currentPath = "";

function LocationProbe() {
  const location = useLocation();
  currentPath = location.pathname + location.search;
  return null;
}

/** Renders `ui` (the whole app by default) at `path` with fresh providers. */
export function renderAt(
  path: string,
  ui: ReactNode = <AppShell />,
): RenderResult & { user: UserEvent; qc: QueryClient; path(): string } {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const user = userEvent.setup();
  const result = render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[path]}>
        {ui}
        <LocationProbe />
      </MemoryRouter>
    </QueryClientProvider>,
  );
  return { ...result, user, qc, path: () => currentPath };
}
