<script lang="ts">
  import { onMount } from 'svelte';
  import { hitHref } from '../lib/search';
  import { localToday, type Row, type Task } from '../lib/tasks';
  import { isTask, viewItems, type Items } from '../lib/views';
  import { ownsKeys } from '../objects/route';
  import { TASKS_CHANGED } from '../tasks/TaskList.svelte';
  import TaskRow from '../tasks/TaskRow.svelte';

  // `id`: the view, read once (ViewPage re-creates the layout after an edit).
  let { id }: { id: number } = $props();

  let data = $state<Items | null>(null);
  let error = $state('');
  let today = $state(localToday());
  // -1: nothing selected yet, so the first `j` selects the first item.
  let sel = $state(-1);
  const rows: (TaskRow | undefined)[] = $state([]);
  const links: (HTMLAnchorElement | undefined)[] = $state([]);
  // Bumped per request so a slow, stale response never overwrites a newer one.
  let seq = 0;

  /** The server's four quadrants in its order, each item with its index across them (the j/k order). */
  const quadrants = $derived.by(() => {
    let i = 0;
    return (data?.groups ?? []).map((g) => ({ ...g, items: g.items.map((item) => ({ item, i: i++ })) }));
  });
  const flat = $derived(quadrants.flatMap((q) => q.items.map((x) => x.item)));

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
      const next = await viewItems(id);
      if (mine !== seq) return;
      today = localToday();
      data = next;
      sel = -1;
    } catch (e) {
      if (mine === seq) error = `error: ${e instanceof Error ? e.message : String(e)}`;
    }
  }

  /** Items carry no sub-task counts, so their rows show none. */
  function row(item: Task): Row {
    return { ...item, subtasks: { open: 0, total: 0 } };
  }

  function focus(i: number) {
    if (rows[i]) rows[i].focus();
    else links[i]?.focus();
  }

  /** Completes (or reopens) the selected task, then reloads every list so it can change quadrant. */
  async function toggle(i: number) {
    await rows[i]?.toggle();
    window.dispatchEvent(new Event(TASKS_CHANGED));
  }

  function keepsKeys(target: EventTarget | null): boolean {
    return ownsKeys(target) || (target instanceof HTMLElement && target.closest('[data-owns-keys]') !== null);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey || keepsKeys(e.target) || flat.length === 0) return;
    if (e.key === 'j' || e.key === 'k') {
      sel = Math.min(Math.max(sel + (e.key === 'j' ? 1 : -1), 0), flat.length - 1);
      focus(sel);
    } else if (e.key === 'x' && sel >= 0 && rows[sel]) void toggle(sel);
    // A focused link follows itself on Enter (a selected item's link is focused).
    else if (e.key === 'Enter' && sel >= 0 && !(e.target instanceof HTMLAnchorElement))
      location.hash = hitHref(flat[sel]);
    else return;
    e.preventDefault();
  }
</script>

<svelte:window {onkeydown} />

<section class="results" aria-label="results">
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  <div class="matrix">
    {#each quadrants as q (q.key)}
      <section class="quadrant" aria-label={q.label}>
        <h2>{q.label} <span class="dim">{q.items.length}</span></h2>
        {#if q.items.length === 0}
          <p class="dim">empty</p>
        {:else}
          <ul class="rows">
            {#each q.items as { item, i } (item.id)}
              {#if isTask(item)}
                <TaskRow row={row(item)} {today} selected={i === sel} onfocus={() => (sel = i)} bind:this={rows[i]} />
              {:else}
                <li class="obj" class:sel={i === sel}>
                  <span class="dim">{item.type}</span>
                  <a href={hitHref(item)} bind:this={links[i]} onfocus={() => (sel = i)}>{item.title}</a>
                </li>
              {/if}
            {/each}
          </ul>
        {/if}
      </section>
    {/each}
  </div>
  {#if data?.truncated}<p class="dim" role="status">showing {flat.length} of {data.total}</p>{/if}
</section>

<style>
  .results {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }
  /* A quadrant stretches to its row and is at least 8rem tall, so an empty one keeps its size. */
  .matrix {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-auto-rows: minmax(8rem, auto);
    gap: 1ch;
  }
  .quadrant {
    min-width: 0;
    border: 1px solid var(--line);
    padding: 0 1ch 0.25rem;
  }
  h2 {
    margin: 0 0 0.25rem;
    font-size: inherit;
    font-weight: normal;
    border-bottom: 1px solid var(--line);
    overflow-wrap: anywhere;
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .obj {
    display: flex;
    align-items: center;
    gap: 1ch;
    padding-left: calc(44px + 1ch);
  }
  .obj.sel {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .obj a {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    min-height: 44px;
    color: inherit;
    text-decoration: none;
    overflow-wrap: anywhere;
  }
  .dim {
    margin: 0;
    color: var(--dim);
  }
  .err {
    margin: 0;
    color: var(--err);
  }
  @media (max-width: 599.98px) {
    .matrix {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
