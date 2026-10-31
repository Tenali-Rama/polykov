<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { hitHref } from '../lib/search';
  import { isTask, viewItems, type Group, type Item } from '../lib/views';
  import { TASKS_CHANGED } from '../tasks/TaskList.svelte';
  import { boardGroup, columns, movable, moveTask } from './move';

  // `id`: the view, read once (ViewPage re-creates the layout after an edit).
  let { id }: { id: number } = $props();

  let cols = $state<Group[]>([]);
  let group = $state<ReturnType<typeof boardGroup>>('status');
  let loaded = $state(false);
  let truncated = $state<{ shown: number; total: number } | null>(null);
  let error = $state('');
  let board = $state<HTMLElement>();
  // The card being dragged, kept here because jsdom and some browsers drop `dataTransfer` data.
  let dragging: number | null = null;
  // Bumped per request so a slow, stale response never overwrites a newer one.
  let seq = 0;

  const canMove = $derived(movable(group));
  const count = $derived(cols.reduce((n, c) => n + c.items.length, 0));

  onMount(() => {
    void load();
    const reload = () => void load();
    window.addEventListener(TASKS_CHANGED, reload);
    return () => window.removeEventListener(TASKS_CHANGED, reload);
  });

  async function load() {
    const mine = ++seq;
    error = '';
    try {
      const data = await viewItems(id);
      if (mine !== seq) return;
      group = boardGroup(data.view);
      cols = columns(group, data.groups);
      truncated = data.truncated ? { shown: data.groups.flatMap((g) => g.items).length, total: data.total } : null;
      loaded = true;
    } catch (e) {
      if (mine === seq) error = `error: ${message(e)}`;
    }
  }

  function message(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  /** The column and row of card `itemId`, or `null`. */
  function find(itemId: number): { c: number; r: number } | null {
    for (let c = 0; c < cols.length; c++) {
      const r = cols[c].items.findIndex((x) => x.id === itemId);
      if (r >= 0) return { c, r };
    }
    return null;
  }

  function focusCard(itemId: number) {
    board?.querySelector<HTMLElement>(`[data-card="${itemId}"]`)?.focus();
  }

  /** Moves a card to column `to` at once, then asks the server; a failure puts it back. */
  async function move(itemId: number, to: number) {
    const at = find(itemId);
    if (!at || at.c === to || to < 0 || to >= cols.length) return;
    const item = cols[at.c].items[at.r];
    error = '';
    if (!canMove) return void (error = 'read-only grouping');
    if (!isTask(item)) return void (error = 'only tasks move');
    const before = $state.snapshot(cols) as Group[];
    cols[at.c].items.splice(at.r, 1);
    cols[to].items.push(item);
    await tick();
    focusCard(itemId);
    try {
      const task = await moveTask(group, itemId, cols[to].key);
      const now = find(itemId);
      if (now) cols[now.c].items[now.r] = task;
    } catch (e) {
      cols = before;
      error = `error: ${message(e)}`;
      await tick();
      focusCard(itemId);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey || !(e.target instanceof HTMLElement)) return;
    const card = e.target.closest<HTMLElement>('[data-card]');
    const at = card ? find(Number(card.dataset.card)) : null;
    if (!at) return;
    const step = e.key === 'ArrowLeft' ? -1 : e.key === 'ArrowRight' ? 1 : 0;
    if (step !== 0 && e.shiftKey) void move(cols[at.c].items[at.r].id, at.c + step);
    else if (e.shiftKey) return;
    else if (step !== 0) {
      // The nearest column that way with a card in it, at the same row or its last one.
      let c = at.c + step;
      while (c >= 0 && c < cols.length && cols[c].items.length === 0) c += step;
      if (c >= 0 && c < cols.length) focusCard(cols[c].items[Math.min(at.r, cols[c].items.length - 1)].id);
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      const items = cols[at.c].items;
      const r = Math.min(Math.max(at.r + (e.key === 'ArrowDown' ? 1 : -1), 0), items.length - 1);
      focusCard(items[r].id);
    } else return;
    e.preventDefault();
  }

  function draggable(item: Item): boolean {
    return canMove && isTask(item);
  }

  function ondrop(e: DragEvent, to: number) {
    e.preventDefault();
    if (dragging !== null) void move(dragging, to);
    dragging = null;
  }
</script>

<section class="kanban" aria-label="results">
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if loaded && !canMove}<p class="dim" role="status">read-only grouping</p>{/if}
  <!-- Keys bubble up from the focused card; each card is a link, so Tab and Enter work on their own. -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="board" bind:this={board} {onkeydown}>
    {#each cols as col, c (col.key)}
      <section
        class="col"
        aria-label={col.label}
        ondragover={(e) => {
          if (dragging !== null) e.preventDefault();
        }}
        ondrop={(e) => ondrop(e, c)}
      >
        <h2>{col.label} <span class="dim">{col.items.length}</span></h2>
        <ul aria-label={col.label}>
          {#each col.items as item (item.id)}
            <li>
              <a
                class="card"
                href={hitHref(item)}
                data-card={item.id}
                draggable={draggable(item)}
                ondragstart={(e) => {
                  dragging = item.id;
                  e.dataTransfer?.setData('text/plain', String(item.id));
                }}
                ondragend={() => (dragging = null)}
              >
                <span class="title">
                  {#if isTask(item) && item.task.priority < 4}<span class="prio">p{item.task.priority}</span>{/if}
                  {item.title}
                </span>
                {#if isTask(item)}
                  {#if item.task.scheduled || item.task.deadline}
                    <span class="meta">
                      {#if item.task.scheduled}<span>{item.task.scheduled}</span>{/if}
                      {#if item.task.deadline}<span>due {item.task.deadline}</span>{/if}
                    </span>
                  {/if}
                {:else}
                  <span class="meta">{item.type}</span>
                {/if}
              </a>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </div>
  {#if loaded && count === 0}<p class="dim">nothing matches</p>{/if}
  {#if truncated}<p class="dim" role="status">showing {truncated.shown} of {truncated.total}</p>{/if}
</section>

<style>
  .kanban {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }
  /* The board scrolls inside its own box, one column per snap, so the page never scrolls sideways. */
  .board {
    display: flex;
    gap: 1ch;
    min-width: 0;
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    padding-bottom: 0.25rem;
  }
  .col {
    flex: 1 0 260px;
    min-width: 260px;
    scroll-snap-align: start;
    border: 1px solid var(--line);
    display: flex;
    flex-direction: column;
  }
  h2 {
    margin: 0;
    padding: 0 1ch;
    font-size: inherit;
    font-weight: normal;
    border-bottom: 1px solid var(--line);
    overflow-wrap: anywhere;
  }
  ul {
    flex: 1;
    list-style: none;
    margin: 0;
    padding: 0.25rem;
    min-height: 44px;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .card {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-height: 44px;
    padding: 0.25rem 1ch;
    border: 1px dashed var(--line);
    color: inherit;
    text-decoration: none;
    overflow-wrap: anywhere;
  }
  .card:focus-visible,
  .card:hover {
    border-style: solid;
    border-color: var(--accent);
  }
  .prio {
    color: var(--accent);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0 1ch;
    color: var(--dim);
  }
  .dim {
    margin: 0;
    color: var(--dim);
  }
  .err {
    margin: 0;
    color: var(--err);
  }
</style>
