import type { ServiceError } from "../bindings";

/** A command failed in the core; carries the typed `ServiceError` (code + message). */
export class CommandError extends Error {
  constructor(public error: ServiceError) {
    super(error.message);
    this.name = "CommandError";
  }
}

type CommandResult<T> = { status: "ok"; data: T } | { status: "error"; error: ServiceError };

export function unwrap<T>(result: CommandResult<T>): T {
  if (result.status === "error") throw new CommandError(result.error);
  return result.data;
}

export const isNotFound = (e: unknown): boolean => e instanceof CommandError && e.error.code === "notFound";

/** Fires a command whose effect arrives through the core's events; failures are logged. */
export const runCommand = (call: Promise<CommandResult<unknown>>): void =>
  void call.then(unwrap).catch(console.error);
