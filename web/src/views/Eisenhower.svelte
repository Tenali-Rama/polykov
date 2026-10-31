<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { hitHref } from '../lib/search';
  import { localToday } from '../lib/tasks';
  import { isTask } from '../lib/views';
  import TaskRow from '../tasks/TaskRow.svelte';
  import { indexed, loader, rowKey, taskRow, type Loaded, type Source } from './source';

  // `source`: the items, read once (ViewPage re-creates the layout after an edit).
  let { source }: { source: Source } = $props();

  let data = $state<Loaded | null>(null);
  let error = $state('');
  let today = $state(localToday());
  // -1: nothing selected yet, so the first `j` selects the first item.
  let sel = $state(-1);
  const rows: (TaskRow | undefined)[] = $state([]);
  const links: (HTMLAnchorElement | undefined)[] = $state([]);

  /** The four quadrants in the server's order. */
  const quadrants = $derived(indexed(data?.groups ?? []));
  const flat = $derived(quadrants.flatMap((q) => q.items.map((x) => x.item)));

  const { watch } = loader(
    untrack(() => source),
    (next) => {
      today = localToday();
      data = next;
      sel = -1;
    },
    (e) => (error = e),
  );
  onMount(watch);

  function onkeydown(e: KeyboardEvent) {
    sel = rowKey(e, sel, flat, { rows, links }) ?? sel;
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
                <TaskRow row={taskRow(item)} {today} selected={i === sel} onfocus={() => (sel = i)} bind:this={rows[i]} />
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
