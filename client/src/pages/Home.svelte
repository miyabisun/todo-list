<script lang="ts">
  import AddForm from "../lib/AddForm.svelte";
  import TodoItem from "../lib/TodoItem.svelte";
  import { fetchTodos, setDone } from "../lib/api";
  import { localDate, type Todo } from "../lib/todo";

  type ListState = "loading" | "empty" | "error" | "success";

  // Toggled items stay where they are until the next load, so a mistaken tap
  // is undone by tapping the same box again.
  let open = $state<Todo[]>([]);
  let done = $state<Todo[]>([]);
  let listState = $state<ListState>("loading");
  // The failed row explains itself, so the reason is visible where the tap happened.
  let failedId = $state<string | null>(null);
  let today = $state(localDate(new Date()));

  let controller: AbortController | undefined;
  let loadedOnce = false;

  async function load() {
    controller?.abort();
    controller = new AbortController();
    if (!loadedOnce) {
      listState = "loading";
    }
    try {
      const todos = await fetchTodos(controller.signal);
      today = localDate(new Date());
      open = todos.filter((todo) => !todo.done_at);
      done = todos.filter((todo) => todo.done_at);
      listState = open.length === 0 ? "empty" : "success";
      loadedOnce = true;
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") {
        return;
      }
      listState = "error";
    }
  }

  async function toggle(todo: Todo) {
    const previous = todo.done_at;
    const finish = !previous;
    todo.done_at = finish ? new Date().toISOString() : null;
    failedId = null;
    try {
      Object.assign(todo, await setDone(todo.id, finish));
    } catch {
      todo.done_at = previous;
      failedId = todo.id;
    }
  }

  function onVisibilityChange() {
    if (document.visibilityState === "visible") {
      void load();
    }
  }

  $effect(() => {
    void load();
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      controller?.abort();
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  });
</script>

<section class="content" data-state={listState}>
  <AddForm onadded={load} />
  {#if listState === "loading"}
    <p class="state">
      <span class="spinner" aria-hidden="true"></span>読み込み中…
    </p>
  {:else if listState === "error"}
    <div class="state-wrap">
      <p class="state error">読み込みに失敗しました</p>
      <button class="btn" type="button" onclick={() => void load()}>
        再試行
      </button>
    </div>
  {:else}
    {#if listState === "empty"}
      <p class="state">やることはありません</p>
    {:else}
      <ul class="todos">
        {#each open as todo (todo.id)}
          <TodoItem
            {todo}
            {today}
            failed={failedId === todo.id}
            ontoggle={() => void toggle(todo)}
          />
        {/each}
      </ul>
    {/if}
    {#if done.length > 0}
      <details class="done">
        <summary>完了済み {done.length}件</summary>
        <ul class="todos">
          {#each done as todo (todo.id)}
            <TodoItem
              {todo}
              {today}
              failed={failedId === todo.id}
              ontoggle={() => void toggle(todo)}
            />
          {/each}
        </ul>
      </details>
    {/if}
  {/if}
</section>

<style lang="sass">
  .todos
    display: flex
    flex-direction: column
    gap: var(--sp-2)
    margin: 0
    padding: 0
    list-style: none

  .done
    margin-top: var(--sp-5)

    summary
      padding: var(--sp-3) 0
      color: var(--c-muted)
      font-size: var(--fs-sm)
      cursor: pointer

    .todos
      margin-top: var(--sp-2)
</style>
