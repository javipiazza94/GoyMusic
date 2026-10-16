import { useQuery } from "@tanstack/react-query";
import { commands, type Artist } from "../bindings";
import { keys } from "./queries";
import { unwrap } from "./result";

// Module-level so its reference is stable: Query re-runs an inline `select` on every render.
const toIndex = (artists: Artist[]) => new Map(artists.map((a) => [a.name.toLowerCase(), a.id]));

/** Name (lower case) → artist id. Tracks carry artist names only, so links resolve the id here. */
export function useArtistIndex(): Map<string, number> {
  const { data } = useQuery({
    queryKey: keys.artists,
    queryFn: async () => unwrap(await commands.artists()),
    select: toIndex,
  });
  return data ?? EMPTY;
}

const EMPTY = new Map<string, number>();

export function useArtistId(name: string | null): number | null {
  const index = useArtistIndex();
  return name ? (index.get(name.toLowerCase()) ?? null) : null;
}
