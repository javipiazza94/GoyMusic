import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { Cover } from "./Cover";
import { GenArt } from "./GenArt";
import { huesFor } from "./hue";

test("generated art shows the initial on the item's colours", () => {
  render(<GenArt seed="carretera" label="Carretera" />);
  const art = screen.getByRole("img", { name: "Carretera" });
  expect(art.textContent).toBe("C");
  const [a, b] = huesFor("carretera");
  expect(art.style.getPropertyValue("--hue-a")).toBe(String(a));
  expect(art.style.getPropertyValue("--hue-b")).toBe(String(b));
});

test("a cover without album uses generated art instead of a grey box", () => {
  render(<Cover albumId={null} size={200} seed="Grabación 1" label="Grabación 1" />);
  expect(screen.getByRole("img", { name: "Grabación 1" }).textContent).toBe("G");
});
