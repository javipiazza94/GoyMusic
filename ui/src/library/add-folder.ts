import { open } from "@tauri-apps/plugin-dialog";
import { commands } from "../bindings";
import { unwrap } from "../core/result";

/** Asks for a directory and adds it to the library; the scan starts in the core. */
export async function addFolderFromDialog(): Promise<void> {
  const path = await open({ directory: true });
  if (typeof path === "string") unwrap(await commands.addFolder(path));
}
