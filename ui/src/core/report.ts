import { usePlayer } from "./player-store";
import { CommandError, unwrap, type CommandResult } from "./result";

/** Codes the user can act on (or that just mean "already gone"): a passing notice is enough. */
const RECOVERABLE = new Set(["notFound", "invalidInput", "folderOverlap"]);

/** Shows a failure (spec §9): recoverable ones as a notice, the rest in the error panel. */
export function reportError(e: unknown): void {
  console.error(e);
  const store = usePlayer.getState();
  if (e instanceof CommandError && RECOVERABLE.has(e.error.code)) store.notify("error", e.error.message);
  else store.setGrave(e instanceof Error ? e.message : String(e));
}

/** Fires a command whose effect arrives through the core's events; failures are reported. */
export const runCommand = (call: Promise<CommandResult<unknown>>): void => void call.then(unwrap).catch(reportError);
