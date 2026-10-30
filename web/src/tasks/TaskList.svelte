<script lang="ts" module>
  /** Window event that makes every mounted task list reload (dispatched after a capture, task 036). */
  export const TASKS_CHANGED = 'polykov:tasks-changed';
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { listTasks, localToday, type ListFilter, type ListName, type Row } from '../lib/tasks';
  import { objectHref, ownsKeys } from '../objects/route';
  import TaskRow from './TaskRow.svelte';

  const PAGE = 100;

  // `filter` given: the list is embedded in a view (project, area) and shows no heading of its own.
  let { name, filter }: { name: ListName; filter?: Pick<ListFilter, 'project' | 'area'> } = $props();

  let rows = $state<Row[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let error = $state('');
  // -1: nothing selected yet, so the first `j` selects the first row.
  let sel = $state(-1);
  let today = $state(localToday());
  const refs: TaskRow[] = $state([]);
  // Bumped per request so a slow, stale response never overwrites a newer one.
  let seq = 0;

  /** Upcoming groups rows under their date (scheduled, else deadline); the other lists are one group. */
  const groups = $derived.by(() => {
    const out: { date: string; rows: { row: Row; i: number }[] }[] = [];
    rows.forEach((row, i) => {
      const date = name === 'upcoming' ? (row.task.scheduled ?? row.task.deadline ?? '').slice(0, 10) : '';
      if (out.at(-1)?.date !== date) out.push({ date, rows: [] });
      out[out.length - 1].rows.push({ row, i });
    });
    return out;
  });

  $effect(() => {
    void load(name, filter, false);
  });

  onMount(() => {
    const reload = () => void load(name, filter, false);
    window.addEventListener(TASKS_CHANGED, reload);
    return () => window.removeEventListener(TASKS_CHANGED, reload);
  });

  async function load(list: ListName, f: ListFilter | undefined, append: boolean) {
    const mine = ++seq;
    loading = true;
    error = '';
    try {
      const page = await listTasks(list, { ...f, limit: PAGE, offset: append ? rows.length : 0 });
      if (mine !== seq) return;
      today = localToday();
      rows = append ? [...rows, ...page.tasks] : page.tasks;
      total = page.total;
      if (!append) sel = -1;
    } catch (e) {
      if (mine === seq) error = `error: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      if (mine === seq) loading = false;
    }
  }

  function move(by: number) {
    sel = Math.min(Math.max(sel + by, 0), rows.length - 1);
    refs[sel]?.focus();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey || ownsKeys(e.target) || rows.length === 0) return;
    if (e.key === 'j') move(1);
    else if (e.key === 'k') move(-1);
    else if (e.key === 'x' && sel >= 0) void refs[sel]?.toggle();
    // A focused link follows itself on Enter (a selected row's link is focused).
    else if (e.key === 'Enter' && sel >= 0 && !(e.target instanceof HTMLAnchorElement))
      location.hash = objectHref(rows[sel].id);
    else return;
    e.preventDefault();
  }
</script>

<svelte:window {onkeydown} />

<section class="tasks" aria-label={name}>
  {#if !filter}<h1>{name}</h1>{/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}

  {#each groups as g (g.date)}
    {#if g.date}<h2>{g.date}</h2>{/if}
    <ul class="rows" aria-label={g.date || name}>
      {#each g.rows as { row, i } (row.id)}
        <TaskRow {row} {today} selected={i === sel} onfocus={() => (sel = i)} bind:this={refs[i]} />
      {/each}
    </ul>
  {/each}

  {#if !loading && !error && rows.length === 0}<p class="dim">nothing in {name}</p>{/if}
  {#if rows.length < total}
    <button type="button" class="more" disabled={loading} onclick={() => load(name, filter, true)}>[ more ]</button>
  {/if}
</section>

<style>
  .tasks {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }
  h1,
  h2 {
    margin: 0;
    font-size: inherit;
    font-weight: normal;
  }
  h1 {
    color: var(--accent);
  }
  h2 {
    margin-top: 0.5rem;
    color: var(--dim);
    border-bottom: 1px solid var(--line);
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .more {
    align-self: flex-start;
    min-height: 44px;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    cursor: pointer;
  }
  .more:hover {
    color: var(--fg);
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
