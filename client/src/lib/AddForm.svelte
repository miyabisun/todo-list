<script lang="ts">
  import { addTodo } from "./api";
  import { newTodoFields } from "./todo";

  let { onadded }: { onadded: () => Promise<void> } = $props();

  let title = $state("");
  let url = $state("");
  let due = $state("");
  let body = $state("");
  let sending = $state(false);
  let failure = $state("");

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!title.trim() || sending) {
      return;
    }
    sending = true;
    failure = "";
    try {
      await addTodo(newTodoFields({ title, url, due, body }));
      title = "";
      url = "";
      due = "";
      body = "";
      await onadded();
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      failure = `追加できませんでした（${reason}）。入力はそのまま残っています。`;
    } finally {
      sending = false;
    }
  }
</script>

<form class="add" onsubmit={submit}>
  <label for="new-title">新しい ToDo</label>
  <div class="row">
    <input
      id="new-title"
      type="text"
      autocomplete="off"
      required
      bind:value={title}
    />
    <button class="btn btn-primary" type="submit" disabled={sending}>
      追加
    </button>
  </div>
  <details class="more">
    <summary>リンク・期日・メモ</summary>
    <div class="fields">
      <label for="new-url">リンク</label>
      <input
        id="new-url"
        type="url"
        inputmode="url"
        placeholder="https://"
        pattern="[Hh][Tt][Tt][Pp][Ss]?://.+"
        title="http:// または https:// で始まるリンク"
        bind:value={url}
      />
      <label for="new-due">期日</label>
      <input id="new-due" type="date" bind:value={due} />
      <label for="new-body">メモ</label>
      <textarea id="new-body" rows="3" bind:value={body}></textarea>
    </div>
  </details>
  {#if failure}
    <p class="error-banner" role="alert">{failure}</p>
  {/if}
</form>

<style lang="sass">
  .add
    margin-bottom: var(--sp-4)

  label
    display: block
    margin-bottom: var(--sp-1)
    color: var(--c-muted)
    font-size: var(--fs-xs)
    line-height: 1.4

  .row
    display: flex
    gap: var(--sp-2)

  input, textarea
    width: 100%
    min-width: 0
    padding: var(--sp-2)
    border: 1px solid var(--c-border)
    border-radius: var(--radius-sm)
    background: var(--c-surface)
    color: var(--c-on-surface)
    font: inherit
    font-size: var(--fs-lg)
    line-height: 1.6

    &:focus
      border-color: var(--c-accent)

  textarea
    resize: vertical

  .btn-primary
    flex-shrink: 0

  .more
    margin-top: var(--sp-1)

    summary
      padding: var(--sp-2) 0
      color: var(--c-muted)
      font-size: var(--fs-sm)
      cursor: pointer

  .fields
    display: flex
    flex-direction: column

    input, textarea
      margin-bottom: var(--sp-3)
</style>
