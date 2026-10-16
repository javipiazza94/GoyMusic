import { expect, test } from "vitest";
import { emptySelection, nextSelection } from "./selection";

const keys = (s: { keys: Set<number> }) => [...s.keys].sort((a, b) => a - b);
const plain = { ctrl: false, shift: false };

test("a plain click selects only that row", () => {
  let s = nextSelection(emptySelection, 2, plain);
  s = nextSelection(s, 5, plain);
  expect(keys(s)).toEqual([5]);
  expect(s.anchor).toBe(5);
});

test("ctrl toggles and shift selects a range from the anchor", () => {
  let s = nextSelection(emptySelection, 2, plain);
  s = nextSelection(s, 4, { ctrl: true, shift: false });
  expect(keys(s)).toEqual([2, 4]);
  s = nextSelection(s, 2, { ctrl: true, shift: false });
  expect(keys(s)).toEqual([4]);
  // the anchor is the last row clicked, even when that click deselected it
  s = nextSelection(s, 5, { ctrl: false, shift: true });
  expect(keys(s)).toEqual([2, 3, 4, 5]);
  s = nextSelection(s, 0, { ctrl: false, shift: true });
  expect(keys(s)).toEqual([0, 1, 2]);
});
