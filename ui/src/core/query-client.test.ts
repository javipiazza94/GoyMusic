import { expect, test, vi } from "vitest";
import { usePlayer } from "./player-store";
import { createQueryClient } from "./query-client";
import { CommandError } from "./result";

const fail = (code: "notFound" | "database") => vi.fn(async () => {
  throw new CommandError({ code, message: `${code} failure` });
});

test("not found is final: no retries and no error panel", async () => {
  const qc = createQueryClient({ retryDelay: 0 });
  const queryFn = fail("notFound");
  await expect(qc.fetchQuery({ queryKey: ["x"], queryFn })).rejects.toThrow();
  expect(queryFn).toHaveBeenCalledTimes(1);
  expect(usePlayer.getState().grave).toBeNull();
});

test("other failures are retried and then reported", async () => {
  const qc = createQueryClient({ retryDelay: 0 });
  const queryFn = fail("database");
  await expect(qc.fetchQuery({ queryKey: ["y"], queryFn, retry: qc.getDefaultOptions().queries!.retry })).rejects.toThrow();
  expect(queryFn.mock.calls.length).toBeGreaterThan(1);
  expect(usePlayer.getState().grave).toBe("database failure");
});

test("data stays until a core event invalidates it", () => {
  const options = createQueryClient().getDefaultOptions().queries!;
  expect(options.staleTime).toBe(Infinity);
  expect(options.refetchOnWindowFocus).toBe(false);
});
