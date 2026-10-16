import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { huesFor } from "../library/hue";
import { ViewHeader } from "./ViewHeader";

test("a view header takes its item's colours and shows generated art without a cover", () => {
  const { container } = render(<ViewHeader kind="PLAYLIST" title="Carretera" meta={[]} seed="Carretera" cover={{ albumId: null }} />);
  const header = container.querySelector("header")!;
  expect(header.style.getPropertyValue("--hue-a")).toBe(String(huesFor("Carretera")[0]));
  expect(screen.getByRole("img", { name: "Carretera" }).textContent).toBe("C");
});
