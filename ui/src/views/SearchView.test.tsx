import { screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { commands } from "../bindings";
import { ok, track } from "../test/fixtures";
import { renderAt } from "../test/render";

const box = () => screen.getByRole("searchbox", { name: "Buscar en tu biblioteca" });

test("search updates the url and shows the top result", async () => {
  vi.mocked(commands.search).mockResolvedValue(
    ok({ top: { kind: "artist", item: { id: 2, name: "Radiohead" } }, tracks: [track(1, "Airbag")], albums: [], artists: [] }),
  );
  const { user, path } = renderAt("/search");
  await user.type(box(), "radioheaf");
  expect(path()).toBe("/search?q=radioheaf");
  const top = await screen.findByRole("region", { name: "Mejor resultado" });
  expect((await screen.findByRole("link", { name: /^Radiohead/ })).getAttribute("href")).toBe("/artist/2");
  expect(top).toBeTruthy();
  expect(commands.search).toHaveBeenLastCalledWith("radioheaf", 20);
});

test("search starts from the query in the url", async () => {
  renderAt("/search?q=airbag");
  expect((box() as HTMLInputElement).value).toBe("airbag");
  await vi.waitFor(() => expect(commands.search).toHaveBeenCalledWith("airbag", 20));
});

test("search with no results says so", async () => {
  renderAt("/search?q=zzz");
  expect(await screen.findByText("Sin resultados para «zzz»")).toBeTruthy();
});

test("an empty query does not search", () => {
  renderAt("/search?q=%20%20");
  expect(commands.search).not.toHaveBeenCalled();
});
