import type { ServiceError } from "../bindings";

/** A command failed in the core; carries the typed `ServiceError` (code + message). */
export class CommandError extends Error {
  constructor(public error: ServiceError) {
    super(error.message);
    this.name = "CommandError";
  }
}

export type CommandResult<T> = { status: "ok"; data: T } | { status: "error"; error: ServiceError };

export function unwrap<T>(result: CommandResult<T>): T {
  if (result.status === "error") throw new CommandError(result.error);
  return result.data;
}

export const isNotFound = (e: unknown): boolean => e instanceof CommandError && e.error.code === "notFound";
