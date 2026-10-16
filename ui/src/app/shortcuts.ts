import { useEffect } from "react";
import { useNavigate } from "react-router";
import { commands } from "../bindings";
import { usePlayer } from "../core/player-store";
import { reportError, runCommand } from "../core/report";

/** True when keys typed there belong to a text field, not to the app's shortcuts. */
export function isTypingTarget(t: EventTarget | null): boolean {
  if (!(t instanceof HTMLElement)) return false;
  return t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName);
}

/** Global shortcuts of spec §7: space play/pause, Ctrl+F search, Ctrl+←/→ previous/next. */
export function useShortcuts(): void {
  const navigate = useNavigate();
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.defaultPrevented || isTypingTarget(e.target)) return;
      const ctrl = e.ctrlKey || e.metaKey;
      if (e.key === " " && !ctrl && !e.altKey) {
        // a focused button already reacts to space itself
        if (e.repeat || (e.target instanceof HTMLElement && e.target.closest("button"))) return;
        e.preventDefault();
        if (usePlayer.getState().view?.track) void usePlayer.getState().togglePlay().catch(reportError);
      } else if (ctrl && e.key.toLowerCase() === "f") {
        e.preventDefault();
        navigate("/search");
        document.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
      } else if (ctrl && (e.key === "ArrowRight" || e.key === "ArrowLeft")) {
        e.preventDefault();
        runCommand(e.key === "ArrowRight" ? commands.next() : commands.previous());
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [navigate]);
}
