import { useQuery } from "@tanstack/react-query";
import { commands } from "../bindings";
import { keys } from "./queries";
import { unwrap } from "./result";

/** Name (lower case) → artist id. Tracks carry artist names only, so links resolve the id here. */
export function useArtistIndex(): Map<string, number> {
  const { data } = useQuery({
    queryKey: keys.artists,
    queryFn: async () => unwrap(await commands.artists()),
    select: (artists) => new Map(artists.map((a) => [a.name.toLowerCase(), a.id])),
  });
  return data ?? EMPTY;
}

const EMPTY = new Map<string, number>();

export function useArtistId(name: string | null): number | null {
  const index = useArtistIndex();
  return name ? (index.get(name.toLowerCase()) ?? null) : null;
}
