<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { getTask, updateTask, type ChecklistItem } from '../lib/tasks';

  // The checklist of task `id`. Each change sends the whole list in one `PATCH`; the server's list
  // replaces ours, and focus goes back to the item acted on.
  let { id }: { id: number } = $props();

  let items = $state<ChecklistItem[] | null>(null);
  let error = $state('');
  let editing = $state<number | null>(null);
  let editDraft = $state('');
  let addDraft = $state('');
  let root: HTMLElement;

  onMount(async () => {
    try {
      items = (await getTask(id)).task.checklist;
    } catch (e) {
      error = message(e);
    }
  });

  function message(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  /** Sends `list`; on success focuses `[data-focus=<focus>]` (the add input when missing). */
  async function save(list: ChecklistItem[], focus: string): Promise<boolean> {
    error = '';
    try {
      items = (await updateTask(id, { task: { checklist: list } })).task.checklist;
    } catch (e) {
      error = message(e);
      return false;
    }
    await tick();
    (root.querySelector<HTMLElement>(`[data-focus="${focus}"]`) ?? root.querySelector<HTMLElement>('[data-focus="add"]'))?.focus();
    return true;
  }

  const list = () => items ?? [];

  function toggle(i: number) {
    return save(list().map((it, j) => (j === i ? { ...it, done: !it.done } : it)), `item-${i}`);
  }

  function remove(i: number) {
    const rest = list().filter((_, j) => j !== i);
    return save(rest, `item-${Math.min(i, rest.length - 1)}`);
  }

  function move(i: number, delta: number, focus = 'item') {
    const j = i + delta;
    if (j < 0 || j >= list().length) return;
    const next = [...list()];
    [next[i], next[j]] = [next[j], next[i]];
    // A button disabled at the new position (up at the top, down at the bottom) can't keep focus.
    const end = (focus === 'up' && j === 0) || (focus === 'down' && j === next.length - 1);
    return save(next, `${end ? 'item' : focus}-${j}`);
  }

  function itemKey(e: KeyboardEvent, i: number) {
    if (!e.altKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
    e.preventDefault();
    void move(i, e.key === 'ArrowUp' ? -1 : 1);
  }

  function edit(i: number) {
    editDraft = list()[i].text;
    editing = i;
  }

  async function editKey(e: KeyboardEvent, i: number) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      editing = null;
      await tick();
      root.querySelector<HTMLElement>(`[data-focus="edit-${i}"]`)?.focus();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const text = editDraft;
      editing = null;
      if (!(await save(list().map((it, j) => (j === i ? { ...it, text } : it)), `edit-${i}`))) editing = i;
    }
  }

  async function add(e: SubmitEvent) {
    e.preventDefault();
    if (addDraft.trim() === '') return;
    if (await save([...list(), { text: addDraft, done: false }], 'add')) addDraft = '';
  }

  function addKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && addDraft !== '') {
      e.preventDefault();
      e.stopPropagation();
      addDraft = '';
    }
  }
</script>

<section class="checklist" aria-label="checklist" bind:this={root}>
  <p class="label">checklist</p>
  {#if items}
    <ul>
      {#each items as it, i (i)}
        <li>
          {#if editing === i}
            <input
              class="text"
              bind:value={editDraft}
              onkeydown={(e) => editKey(e, i)}
              autocomplete="off"
              spellcheck="false"
              aria-label="edit item {i + 1}"
              {@attach (node) => node.focus()}
            />
          {:else}
            <button
              type="button"
              class="item"
              class:done={it.done}
              data-focus="item-{i}"
              aria-pressed={it.done}
              onclick={() => toggle(i)}
              onkeydown={(e) => itemKey(e, i)}>{it.done ? '[x]' : '[ ]'} {it.text}</button
            >
            <span class="actions">
              <button type="button" data-focus="edit-{i}" aria-label="edit {it.text}" onclick={() => edit(i)}>[ edit ]</button>
              <button type="button" data-focus="up-{i}" aria-label="move up {it.text}" disabled={i === 0} onclick={() => move(i, -1, 'up')}
                >[ up ]</button
              >
              <button
                type="button"
                data-focus="down-{i}"
                aria-label="move down {it.text}"
                disabled={i === items.length - 1}
                onclick={() => move(i, 1, 'down')}>[ down ]</button
              >
              <button type="button" aria-label="delete {it.text}" onclick={() => remove(i)}>[ delete ]</button>
            </span>
          {/if}
        </li>
      {:else}
        <li class="dim">no items</li>
      {/each}
    </ul>
    <form class="prompt" onsubmit={add}>
      <span class="sigil" aria-hidden="true">&gt;</span>
      <input
        class="text"
        data-focus="add"
        bind:value={addDraft}
        onkeydown={addKey}
        autocomplete="off"
        spellcheck="false"
        aria-label="add item"
        placeholder="add item"
      />
    </form>
  {:else if !error}
    <p class="dim">loading …</p>
  {/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</section>

<style>
  .checklist {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .label {
    color: var(--dim);
  }
  .label::before {
    content: '── ';
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li,
  .actions,
  .prompt {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
    min-width: 0;
  }
  li {
    min-height: 44px;
  }
  .actions {
    flex-wrap: nowrap;
  }
  button {
    min-width: 44px;
    min-height: 44px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    color: var(--fg);
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .item {
    flex: 1 1 20ch;
    min-width: 0;
    color: inherit;
    overflow-wrap: anywhere;
  }
  .item.done {
    color: var(--dim);
    text-decoration: line-through;
  }
  .text {
    flex: 1;
    min-width: 0;
    min-height: 44px;
    border: 0;
    border-bottom: 1px solid var(--line);
    background: transparent;
    color: inherit;
    font: inherit;
    caret-shape: block;
  }
  .text:focus {
    outline: none;
    border-bottom-color: var(--accent);
  }
  .sigil {
    color: var(--accent);
  }
  p {
    margin: 0;
  }
  .dim {
    color: var(--dim);
  }
  .err {
    color: var(--err);
  }
</style>
