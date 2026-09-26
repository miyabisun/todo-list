import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { Todo } from "../lib/todo";
import Home from "./Home.svelte";

function todo(id: string, fields: Partial<Todo> = {}): Todo {
  return {
    id,
    title: `架空の用事 ${id}`,
    url: null,
    due: null,
    source: null,
    created_at: "2026-09-01T00:00:00.000Z",
    updated_at: "2026-09-01T00:00:00.000Z",
    done_at: null,
    body: "",
    ...fields,
  };
}

const TODOS = [
  todo("linked", {
    url: "https://example.com/form",
    due: "2000-01-01",
    source: "example-agent",
    body: "持ち物: 架空の書類",
  }),
  todo("plain"),
  todo("unsafe", { url: "javascript:alert(1)" }),
  todo("finished", { done_at: "2026-09-02T00:00:00.000Z" }),
];

function json(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), { status });
}

type Handler = (method: string, url: string, body: unknown) => Response;

function serve(handler: Handler) {
  const mock = vi.fn<typeof fetch>((input, init) =>
    Promise.resolve(
      handler(
        init?.method ?? "GET",
        String(input),
        init?.body ? JSON.parse(String(init.body)) : undefined,
      ),
    ),
  );
  vi.stubGlobal("fetch", mock);
  return mock;
}

function listState(): string | undefined {
  return document.querySelector<HTMLElement>("[data-state]")?.dataset.state;
}

async function loaded(state = "success") {
  await waitFor(() => expect(listState()).toBe(state));
}

describe("Home", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("shows open items with safe links, due text, source and memo; done items are folded", async () => {
    serve(() => json(TODOS));
    render(Home);
    expect(listState()).toBe("loading");
    await loaded();

    const link = screen.getByRole("link", { name: /架空の用事 linked/ });
    expect(link.getAttribute("href")).toBe("https://example.com/form");
    expect(link.getAttribute("target")).toBe("_blank");
    expect(link.getAttribute("rel")).toBe("noopener noreferrer");
    expect(screen.getAllByRole("link")).toHaveLength(1);
    expect(screen.getByText("架空の用事 unsafe").closest("a")).toBeNull();
    expect(screen.getByText("期限切れ 2000/1/1")).toBeTruthy();
    expect(screen.getByText("登録: example-agent")).toBeTruthy();
    expect(
      screen.getByText("持ち物: 架空の書類").closest("details"),
    ).toBeTruthy();

    const folded = screen.getByText("完了済み 1件").closest("details")!;
    expect(folded.open).toBe(false);
    const done = within(folded).getByRole("checkbox", {
      name: "完了: 架空の用事 finished",
    }) as HTMLInputElement;
    expect(done.checked).toBe(true);
  });

  it("completes in place and reopens with a second tap", async () => {
    const mock = serve((method, url, body) => {
      if (method === "PATCH") {
        const done = (body as { done: boolean }).done;
        return json(
          todo("plain", { done_at: done ? "2026-09-26T00:00:00.000Z" : null }),
        );
      }
      return json(TODOS);
    });
    render(Home);
    await loaded();
    const box = screen.getByRole("checkbox", {
      name: "完了: 架空の用事 plain",
    }) as HTMLInputElement;

    await fireEvent.click(box);
    await waitFor(() => expect(box.closest("li")?.dataset.done).toBe("true"));
    expect(box.checked).toBe(true);
    expect(mock).toHaveBeenCalledWith(
      "/api/todos/plain",
      expect.objectContaining({ method: "PATCH", body: '{"done":true}' }),
    );
    expect(screen.getByText("完了済み 1件")).toBeTruthy();

    await fireEvent.click(box);
    await waitFor(() => expect(box.closest("li")?.dataset.done).toBe("false"));
    expect(box.checked).toBe(false);
  });

  it("reverts a failed completion and explains it on that row", async () => {
    serve((method) =>
      method === "PATCH" ? json({ error: "io: disk full" }, 500) : json(TODOS),
    );
    render(Home);
    await loaded();
    const box = screen.getByRole("checkbox", {
      name: "完了: 架空の用事 plain",
    }) as HTMLInputElement;
    await fireEvent.click(box);

    const alert = await screen.findByRole("alert");
    expect(alert.closest("li")).toBe(box.closest("li"));
    expect(alert.textContent).toContain("更新できませんでした");
    expect(box.checked).toBe(false);
  });

  it("adds an item with optional fields, then clears the form and reloads", async () => {
    let list = TODOS;
    const mock = serve((method, _url, body) => {
      if (method === "POST") {
        const created = todo("new", body as Partial<Todo>);
        list = [created, ...list];
        return json(created, 201);
      }
      return json(list);
    });
    render(Home);
    await loaded();

    const title = screen.getByRole("textbox", { name: "新しい ToDo" });
    await fireEvent.input(title, { target: { value: " 架空の予約 " } });
    await fireEvent.input(screen.getByLabelText("リンク"), {
      target: { value: "https://example.com/reserve" },
    });
    await fireEvent.input(screen.getByLabelText("期日"), {
      target: { value: "2026-10-01" },
    });
    await fireEvent.submit(title.closest("form")!);

    await waitFor(() =>
      expect(screen.getByRole("link", { name: /架空の予約/ })).toBeTruthy(),
    );
    const post = mock.mock.calls.find(([, init]) => init?.method === "POST")!;
    expect(JSON.parse(String(post[1]!.body))).toEqual({
      title: "架空の予約",
      url: "https://example.com/reserve",
      due: "2026-10-01",
    });
    expect((title as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("リンク") as HTMLInputElement).value).toBe(
      "",
    );
  });

  it("keeps the typed text and shows the reason when adding fails", async () => {
    serve((method) =>
      method === "POST"
        ? json({ error: "url must be an http(s) URL or null" }, 400)
        : json([]),
    );
    render(Home);
    await loaded("empty");
    const title = screen.getByRole("textbox", { name: "新しい ToDo" });
    await fireEvent.input(title, { target: { value: "架空の用事" } });
    await fireEvent.submit(title.closest("form")!);

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("url must be an http(s) URL");
    expect((title as HTMLInputElement).value).toBe("架空の用事");
  });

  it("shows empty and error states and recovers with retry", async () => {
    const mock = vi
      .fn<typeof fetch>()
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce(json([]));
    vi.stubGlobal("fetch", mock);
    render(Home);
    await loaded("error");
    await fireEvent.click(screen.getByRole("button", { name: "再試行" }));
    await loaded("empty");
    expect(screen.getByText("やることはありません")).toBeTruthy();
  });

  it("reloads when the tab becomes visible again", async () => {
    const mock = serve(() => json(TODOS));
    render(Home);
    await loaded();
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "visible",
    });
    document.dispatchEvent(new Event("visibilitychange"));
    await waitFor(() => expect(mock).toHaveBeenCalledTimes(2));
  });
});
