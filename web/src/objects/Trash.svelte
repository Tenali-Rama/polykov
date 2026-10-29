<script lang="ts">
  import { onMount } from 'svelte';
  import { emptyTrash, listTrash, purgeObject, restoreObject, type Trashed } from '../lib/trash';
  import { listHref } from './route';

  let objects = $state<Trashed[]>([]);
  let loading = $state(true);
  let error = $state('');
  let note = $state('');
  // What waits for a `[ yes ]`: one object's id, `'all'` for the whole trash, or nothing.
  let confirming = $state<number | 'all' | null>(null);

  onMount(async () => {
    try {
      objects = await listTrash();
    } catch (e) {
      error = `error: ${message(e)}`;
    } finally {
      loading = false;
    }
  });

  function message(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  async function act(run: () => Promise<string>) {
    error = '';
    note = '';
    try {
      note = await run();
    } catch (e) {
      error = `error: ${message(e)}`;
    } finally {
      confirming = null;
    }
  }

  function without(id: number) {
    objects = objects.filter((o) => o.id !== id);
  }

  const restore = (o: Trashed) =>
    act(async () => {
      await restoreObject(o.id);
      without(o.id);
      return `restored ${o.title}`;
    });

  const purge = (o: Trashed) =>
    act(async () => {
      await purgeObject(o.id);
      without(o.id);
      return `deleted ${o.title} forever`;
    });

  const empty = () =>
    act(async () => {
      const n = await emptyTrash();
      objects = [];
      return `deleted ${n} forever`;
    });

  /** RFC 3339 → `YYYY-MM-DD`. */
  function day(time: string): string {
    return time.slice(0, 10);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    if (confirming !== null) confirming = null;
    else location.hash = listHref;
  }
</script>

<svelte:window {onkeydown} />

<section class="trash" aria-label="trash">
  <a class="back" href={listHref}>[ esc ] back</a>
  <h1>trash</h1>
  <p class="dim">deleted objects are purged for good 30 days after deletion</p>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if note}<p class="note" role="status">{note}</p>{/if}

  {#if loading}
    <p class="dim">loading …</p>
  {:else if objects.length === 0}
    {#if !error}<p class="dim">trash is empty</p>{/if}
  {:else}
    <ul class="rows" aria-label="trashed objects">
      {#each objects as o (o.id)}
        <li>
          <span class="title">{o.title}</span>
          <span class="dim">{o.type} · deleted {day(o.deleted)} · purges {day(o.purge_at)}</span>
          {#if confirming === o.id}
            <span class="ask">delete forever?</span>
            <button type="button" onclick={() => purge(o)}>[ yes ]</button>
            <button type="button" onclick={() => (confirming = null)} {@attach (node) => node.focus()}>[ no ]</button>
          {:else}
            <button type="button" onclick={() => restore(o)}>[ restore ]</button>
            <button type="button" onclick={() => (confirming = o.id)}>[ delete forever ]</button>
          {/if}
        </li>
      {/each}
    </ul>
    <div class="all">
      {#if confirming === 'all'}
        <span class="ask">delete all {objects.length} forever?</span>
        <button type="button" onclick={empty}>[ yes ]</button>
        <button type="button" onclick={() => (confirming = null)} {@attach (node) => node.focus()}>[ no ]</button>
      {:else}
        <button type="button" onclick={() => (confirming = 'all')}>[ empty trash ]</button>
      {/if}
    </div>
  {/if}
</section>

<style>
  .trash {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .back {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    min-height: 44px;
    color: var(--dim);
    text-decoration: none;
  }
  .back:hover {
    color: var(--fg);
  }
  h1 {
    margin: 0;
    font-size: 1.2rem;
  }
  p {
    margin: 0;
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .rows li,
  .all {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
    border-bottom: 1px solid var(--line);
  }
  .title {
    flex: 1;
    min-width: 0;
  }
  button {
    min-height: 44px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    cursor: pointer;
  }
  button:hover {
    color: var(--fg);
  }
  .all {
    border-bottom: 0;
  }
  .ask,
  .err {
    color: var(--err);
  }
  .note {
    color: var(--ok);
  }
  .dim {
    color: var(--dim);
  }
</style>
