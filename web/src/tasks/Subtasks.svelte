<script lang="ts">
  import { onMount } from 'svelte';
  import { createTask, listSubtasks, localToday, type Row } from '../lib/tasks';
  import TaskRow from './TaskRow.svelte';

  // The direct children of task `id`, with an `add sub-task` prompt.
  let { id }: { id: number } = $props();

  let rows = $state<Row[] | null>(null);
  let error = $state('');
  let draft = $state('');
  const today = localToday();

  onMount(() => act(reload));

  async function reload() {
    rows = await listSubtasks(id);
  }

  async function act(run: () => Promise<void>) {
    error = '';
    try {
      await run();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function add(e: SubmitEvent) {
    e.preventDefault();
    const title = draft.trim();
    if (title === '') return;
    return act(async () => {
      await createTask({ title, task: { parent_id: id } });
      draft = '';
      await reload();
    });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && draft !== '') {
      e.preventDefault();
      e.stopPropagation();
      draft = '';
    }
  }
</script>

<section class="subtasks" aria-label="sub-tasks">
  <p class="label">sub-tasks</p>
  {#if rows}
    <ul>
      {#each rows as row (row.id)}
        <TaskRow {row} {today} />
      {:else}
        <li class="dim">no sub-tasks</li>
      {/each}
    </ul>
    <form class="prompt" onsubmit={add}>
      <span class="sigil" aria-hidden="true">&gt;</span>
      <input
        bind:value={draft}
        {onkeydown}
        autocomplete="off"
        spellcheck="false"
        aria-label="add sub-task"
        placeholder="add sub-task"
      />
    </form>
  {:else if !error}
    <p class="dim">loading …</p>
  {/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</section>

<style>
  .subtasks {
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
  .prompt {
    display: flex;
    align-items: center;
    gap: 0.5ch;
  }
  .sigil {
    color: var(--accent);
  }
  input {
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
  input::placeholder {
    color: var(--dim);
  }
  input:focus {
    outline: none;
    border-bottom-color: var(--accent);
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
