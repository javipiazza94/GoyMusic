import type { QueryClient } from "@tanstack/react-query";
import { events, type Event } from "../bindings";
import { usePlayer, type PlayerStore } from "./player-store";
import { keys } from "./queries";

const SCAN_DONE_LINGER_MS = 1_000;

/** Applies one core event to the query cache and the player store (spec §3, §7). */
export function handleCoreEvent(e: Event, qc: QueryClient, store: PlayerStore): void {
  const refresh = () => void store.refresh().catch(() => {});
  switch (e.type) {
    case "libraryUpdated":
      void qc.invalidateQueries({ queryKey: keys.library });
      break;
    case "scanProgress":
      store.setScan(e.done, e.total);
      if (e.done === e.total) setTimeout(() => store.clearScan(), SCAN_DONE_LINGER_MS);
      break;
    case "playlistsChanged":
      void qc.invalidateQueries({ queryKey: keys.playlists });
      break;
    case "likesChanged":
      void qc.invalidateQueries({ queryKey: keys.likes });
      break;
    case "queueChanged":
    case "playerStateChanged":
      refresh();
      break;
    case "positionChanged":
      store.setPosition(e.track, e.positionMs);
      break;
    case "playbackError":
      store.notify("playbackError", e.reason);
      refresh();
      break;
    case "audioDeviceLost":
      store.notify("deviceLost", e.reason);
      refresh();
      break;
  }
}

/** Subscribes to the core's events; returns the unsubscribe function. */
export async function connectCoreEvents(qc: QueryClient): Promise<() => void> {
  return events.coreEvent.listen((e) => handleCoreEvent(e.payload, qc, usePlayer.getState()));
}
