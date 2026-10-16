import { act, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { renderAt } from "../test/render";
import { useUi } from "./ui-store";

test("renders the view for each route", async () => {
  renderAt("/settings");
  expect(await screen.findByRole("heading", { level: 1, name: "Ajustes" })).toBeTruthy();
});

test("unknown routes show not found with a way home", async () => {
  const { user, path } = renderAt("/nope");
  expect(await screen.findByRole("heading", { name: "No encontrado" })).toBeTruthy();
  await user.click(screen.getByRole("link", { name: "Ir a Inicio" }));
  expect(path()).toBe("/");
});

test("the queue column toggles and is remembered", () => {
  renderAt("/");
  expect(screen.queryByRole("complementary", { name: "Cola" })).toBeNull();
  act(() => useUi.getState().toggleQueue());
  expect(screen.getByRole("complementary", { name: "Cola" })).toBeTruthy();
  expect(localStorage.getItem("goymusic.queueOpen")).toBe("true");
});
