import type { Todo } from "./todo";

/** Throws with the server's `error` text when it sends one. */
async function requestJson<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, init);
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new Error(body?.error ?? `HTTP ${response.status}`);
  }
  return (await response.json()) as T;
}

function send<T>(method: string, url: string, fields: unknown): Promise<T> {
  return requestJson(url, {
    method,
    headers: { "content-type": "application/json" },
    body: JSON.stringify(fields),
  });
}

/** Open items first (by due date), then completed ones (newest first). */
export function fetchTodos(signal?: AbortSignal): Promise<Todo[]> {
  return requestJson("/api/todos", { signal });
}

export function addTodo(fields: Record<string, string>): Promise<Todo> {
  return send("POST", "/api/todos", fields);
}

export function setDone(id: string, done: boolean): Promise<Todo> {
  return send("PATCH", `/api/todos/${encodeURIComponent(id)}`, { done });
}
