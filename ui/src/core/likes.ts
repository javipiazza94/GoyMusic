import { useQuery } from "@tanstack/react-query";
import { useCallback } from "react";
import { commands } from "../bindings";
import { keys } from "./queries";
import { unwrap } from "./result";

export function useLikedIds(): Set<number> {
  const { data } = useQuery({
    queryKey: [...keys.likes, "ids"],
    queryFn: async () => new Set(unwrap(await commands.likedIds())),
  });
  return data ?? EMPTY;
}

const EMPTY = new Set<number>();

/** Likes or unlikes; the cache follows through the core's `likesChanged` event. */
export function useToggleLike(): (id: number, liked: boolean) => Promise<void> {
  return useCallback(async (id, liked) => {
    unwrap(await (liked ? commands.unlike(id) : commands.like(id)));
  }, []);
}
