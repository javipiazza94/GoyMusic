/** Row selection in track lists: plain click selects one, Ctrl toggles, Shift selects a range from the anchor. */
export type Selection = { keys: Set<number>; anchor: number | null };

export const emptySelection: Selection = { keys: new Set(), anchor: null };

export function nextSelection(sel: Selection, index: number, mods: { ctrl: boolean; shift: boolean }): Selection {
  if (mods.shift && sel.anchor !== null) {
    const [from, to] = sel.anchor < index ? [sel.anchor, index] : [index, sel.anchor];
    return { keys: new Set(Array.from({ length: to - from + 1 }, (_, i) => from + i)), anchor: sel.anchor };
  }
  if (mods.ctrl) {
    const keys = new Set(sel.keys);
    if (keys.has(index)) keys.delete(index);
    else keys.add(index);
    return { keys, anchor: index };
  }
  return { keys: new Set([index]), anchor: index };
}
