import { act, renderHook } from "@testing-library/react";
import { expect, test } from "vitest";
import { setSystemDark } from "../test/media";
import { initTheme, useTheme } from "./theme";

const applied = () => document.documentElement.dataset.theme;

test("defaults to the system preference", () => {
  setSystemDark(true);
  initTheme();
  expect(applied()).toBe("dark");
});

test("an explicit choice is applied and remembered", () => {
  initTheme();
  const { result } = renderHook(() => useTheme());
  act(() => result.current.setPref("light"));
  expect(applied()).toBe("light");
  expect(localStorage.getItem("goymusic.theme")).toBe("light");
  expect(result.current.pref).toBe("light");
});

test("follows a system change while on system", () => {
  initTheme();
  act(() => setSystemDark(false));
  expect(applied()).toBe("light");
  act(() => setSystemDark(true));
  expect(applied()).toBe("dark");
});

test("ignores system changes when the choice is explicit", () => {
  localStorage.setItem("goymusic.theme", "dark");
  initTheme();
  act(() => setSystemDark(false));
  expect(applied()).toBe("dark");
});
