import { QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { ok } from "../test/fixtures";
import { useArtistIndex } from "./artist-index";
import { createQueryClient } from "./query-client";

test("the artist index is built once, not on every render", async () => {
  vi.mocked(commands.artists).mockResolvedValue(ok([{ id: 1, name: "Ana" }]));
  const qc = createQueryClient();
  const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
  const { result, rerender } = renderHook(() => useArtistIndex(), { wrapper });
  await waitFor(() => expect(result.current.get("ana")).toBe(1));
  const first = result.current;
  rerender();
  expect(result.current).toBe(first);
});
