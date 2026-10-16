import { commands, type PlayContext } from "../bindings";
import { unwrap } from "./result";

/** Starts `context` at `index`; the player view follows through the core's events. */
export const playFrom = (context: PlayContext, index: number) =>
  void commands.play(context, index).then(unwrap).catch(console.error);
