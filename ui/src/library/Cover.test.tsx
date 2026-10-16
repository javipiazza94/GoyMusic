import { render } from "@testing-library/react";
import { expect, test } from "vitest";
import { Cover } from "./Cover";

test("covers reserve their size before the image loads", () => {
  const { container } = render(<Cover albumId={3} size={200} />);
  const img = container.querySelector("img")!;
  expect(img.getAttribute("width")).toBe("200");
  expect(img.getAttribute("height")).toBe("200");
});
