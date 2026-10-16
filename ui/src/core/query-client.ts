import { QueryCache, QueryClient } from "@tanstack/react-query";
import { reportError } from "./report";
import { isNotFound } from "./result";

const MAX_RETRIES = 2;

/**
 * Query defaults for the whole app. Data stays fresh until a core event invalidates it (the UI
 * never polls, spec §7); "not found" is a final answer, not something to retry; any other
 * failure is reported (spec §9) instead of leaving a view blank.
 */
export function createQueryClient(options: { retryDelay?: number } = {}): QueryClient {
  return new QueryClient({
    queryCache: new QueryCache({
      onError: (error) => {
        if (!isNotFound(error)) reportError(error);
      },
    }),
    defaultOptions: {
      queries: {
        staleTime: Infinity,
        refetchOnWindowFocus: false,
        retry: (count, error) => !isNotFound(error) && count < MAX_RETRIES,
        ...(options.retryDelay !== undefined && { retryDelay: options.retryDelay }),
      },
    },
  });
}
