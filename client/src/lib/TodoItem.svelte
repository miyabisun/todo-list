<script lang="ts">
  import Icon from "./Icon.svelte";
  import { dueLabel, safeUrl, type Todo } from "./todo";

  let {
    todo,
    today,
    failed = false,
    ontoggle,
  }: {
    todo: Todo;
    today: string;
    failed?: boolean;
    ontoggle: () => void;
  } = $props();

  let href = $derived(safeUrl(todo.url));
  let due = $derived(todo.due ? dueLabel(todo.due, today) : null);
</script>

<li class="todo" data-done={todo.done_at ? "true" : "false"}>
  <label class="check">
    <input
      type="checkbox"
      checked={Boolean(todo.done_at)}
      aria-label={`完了: ${todo.title}`}
      onchange={ontoggle}
    />
  </label>
  <div class="main">
    {#if href}
      <a class="title" {href} target="_blank" rel="noopener noreferrer">
        {todo.title}<span class="external"><Icon name="external-link" /></span>
      </a>
    {:else}
      <span class="title">{todo.title}</span>
    {/if}
    {#if due || todo.source}
      <p class="meta">
        {#if due}
          <span class:overdue={due.overdue && !todo.done_at}>{due.text}</span>
        {/if}
        {#if todo.source}
          <span>登録: {todo.source}</span>
        {/if}
      </p>
    {/if}
    {#if failed}
      <p class="error-banner" role="alert">
        更新できませんでした。通信を確認して、もう一度チェックしてください。
      </p>
    {/if}
    {#if todo.body.trim()}
      <details class="memo">
        <summary>メモ</summary>
        <p>{todo.body}</p>
      </details>
    {/if}
  </div>
</li>

<style lang="sass">
  .todo
    display: flex
    align-items: flex-start
    border: 1px solid var(--c-border)
    border-radius: var(--radius-md)
    background: var(--c-surface-raised)

  // 44px touch target around a native checkbox; the accent marks only its state.
  .check
    display: flex
    flex-shrink: 0
    align-items: center
    justify-content: center
    width: 44px
    height: 44px
    cursor: pointer

    input
      width: 20px
      height: 20px
      margin: 0
      accent-color: var(--c-accent)
      cursor: pointer

  .main
    flex: 1
    min-width: 0
    padding: 10px 10px 10px 0

  .title
    display: block
    overflow-wrap: anywhere
    font-size: var(--fs-md)
    font-weight: 500
    line-height: 24px
    color: var(--c-on-surface)

  a.title
    color: var(--c-link)
    text-decoration: none

    &:hover
      text-decoration: underline

  .external
    display: inline-block
    margin-left: var(--sp-1)
    font-size: var(--fs-xs)
    vertical-align: -1px

  .meta
    display: flex
    flex-wrap: wrap
    gap: 0 var(--sp-3)
    margin: 2px 0 0
    color: var(--c-muted)
    font-size: var(--fs-xs)
    line-height: 1.4

  .overdue
    color: var(--c-danger)
    font-weight: 500

  .memo
    margin-top: var(--sp-1)
    font-size: var(--fs-sm)

    summary
      color: var(--c-muted)
      font-size: var(--fs-xs)
      cursor: pointer

    p
      margin: var(--sp-1) 0 0
      white-space: pre-wrap
      overflow-wrap: anywhere
      line-height: 1.5

  .error-banner
    margin-bottom: 0

  .todo[data-done="true"] .title
    color: var(--c-muted)
    text-decoration: line-through
</style>
