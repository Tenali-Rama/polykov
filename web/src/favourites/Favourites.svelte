<script lang="ts" module>
  /** Window event that object changes (pin/unpin) dispatch so the favourites reload. */
  export const OBJECTS_CHANGED = 'polykov:objects-changed';
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { listObjects, type Obj } from '../lib/objects';
  import { objectHref } from '../objects/route';
  import { TASKS_CHANGED } from '../tasks/TaskList.svelte';
  import { areaHref, projectHref } from '../tasks/route';

  // Rendered by Nav inside its `<ul>`, before the list links, so it is one `<li>`.
  let pinned = $state<Obj[]>([]);
  let error = $state('');
  let hash = $state(location.hash);
  // Only the latest load applies, so a slow earlier response cannot overwrite a newer one.
  let seq = 0;

  /** Where a favourite leads: views (ADR 0016), projects and areas get their own pages. */
  function href(o: Obj): string {
    if (o.type === 'view') return `#/v/${o.id}`;
    if (o.type === 'project') return projectHref(o.id);
    if (o.type === 'area') return areaHref(o.id);
    return objectHref(o.id);
  }

  async function load() {
    const mine = ++seq;
    try {
      const { objects } = await listObjects({ pinned: true, limit: 100 });
      if (mine !== seq) return;
      pinned = objects;
      error = '';
    } catch (e) {
      if (mine === seq) error = `favourites: ${e instanceof Error ? e.message : String(e)}`;
    }
  }

  onMount(() => {
    load();
    window.addEventListener(TASKS_CHANGED, load);
    window.addEventListener(OBJECTS_CHANGED, load);
    return () => {
      window.removeEventListener(TASKS_CHANGED, load);
      window.removeEventListener(OBJECTS_CHANGED, load);
    };
  });

  function onhashchange() {
    hash = location.hash;
    load();
  }
</script>

<svelte:window {onhashchange} />

{#if pinned.length > 0 || error}
  <li class="favs">
    <span class="head" id="favourites-head">favourites</span>
    {#if error}<span class="err">{error}</span>{/if}
    <ul aria-labelledby="favourites-head">
      {#each pinned as o (o.id)}
        <li>
          <a href={href(o)} aria-current={href(o) === hash ? 'page' : undefined}
            ><span class="type">{o.type}</span> {o.title}</a
          >
        </li>
      {/each}
    </ul>
  </li>
{/if}

<style>
  .favs {
    flex-basis: 100%;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1.5ch;
    border-bottom: 1px solid var(--line);
  }
  .head {
    color: var(--dim);
  }
  ul {
    display: flex;
    flex-wrap: wrap;
    list-style: none;
    margin: 0;
    padding: 0;
    gap: 0 1.5ch;
  }
  a {
    display: inline-flex;
    align-items: center;
    min-width: 44px;
    min-height: 44px;
    padding: 0 0.5ch;
    color: var(--fg);
    text-decoration: none;
    overflow-wrap: anywhere;
  }
  .type {
    color: var(--dim);
    margin-right: 1ch;
  }
  a[aria-current='page'] {
    color: var(--accent);
    text-decoration: underline;
  }
  .err {
    color: var(--err);
  }
  @media (max-width: 599.98px) {
    .favs,
    ul {
      flex-direction: column;
      align-items: stretch;
    }
    .head {
      padding: 0.5rem 0.5ch 0;
    }
    a {
      display: flex;
    }
  }
</style>
