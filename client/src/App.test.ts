import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import App from "./App.svelte";

describe("App", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("keeps the header to the app name and the menu", () => {
    vi.stubGlobal(
      "fetch",
      vi.fn<typeof fetch>().mockResolvedValue(new Response("[]")),
    );
    render(App);
    const header = screen.getByRole("banner");
    expect(header.querySelector('a[href="/"]')?.textContent).toBe("todo-list");
    expect(screen.getByRole("button", { name: "メニュー" })).toBeTruthy();
    expect(header.querySelectorAll("a, button")).toHaveLength(2);
  });
});
