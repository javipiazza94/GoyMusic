import { expect, test } from "vitest";
import { HUES, huesFor } from "./hue";

test("the same name always gets the same colours", () => {
  expect(huesFor("Carretera")).toEqual(huesFor("Carretera"));
});

test("colours come from the curated set and the pair differs", () => {
  for (const name of ["Carretera", "Ánimo", "Para escribir", "Lumbre Sur", "", "x"]) {
    const [a, b] = huesFor(name);
    expect(HUES).toContain(a);
    expect(HUES).toContain(b);
    expect(a).not.toBe(b);
  }
});

test("different names spread over several colours", () => {
  const names = Array.from({ length: 40 }, (_, i) => `Lista ${i}`);
  expect(new Set(names.map((n) => huesFor(n)[0])).size).toBeGreaterThanOrEqual(6);
});
