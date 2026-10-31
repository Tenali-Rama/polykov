<script lang="ts">
  import { onMount } from 'svelte';
  import { listViews, type View } from '../lib/views';
  import { viewHref } from './route';
  import ViewEditor from './ViewEditor.svelte';

  let views = $state<View[]>([]);
  let loading = $state(true);
  let error = $state('');
  let creating = $state(false);
  let newButton = $state<HTMLButtonElement>();

  onMount(async () => {
    try {
      views = await listViews();
    } catch (e) {
      error = `error: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      loading = false;
    }
  });

  function cancel() {
    creating = false;
    newButton?.focus();
  }
</script>

<section class="index" aria-label="views">
  <div class="head">
    <h1>views</h1>
    <button type="button" aria-expanded={creating} bind:this={newButton} onclick={() => (creating = !creating)}
      >[ new view ]</button
    >
  </div>
  {#if creating}
    <ViewEditor onsaved={(v) => (location.hash = viewHref(v.id))} oncancel={cancel} />
  {/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if views.length > 0}
    <ul class="rows" aria-label="saved views">
      {#each views as v (v.id)}
        <li>
          <a href={viewHref(v.id)}>{v.pinned ? '* ' : ''}{v.title}</a>
          <span class="dim">{v.properties.layout} · {v.properties.query}</span>
        </li>
      {/each}
    </ul>
  {:else if !loading && !error}
    <p class="dim">no views yet</p>
  {/if}
</section>

<style>
  .index {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
  }
  h1 {
    flex: 1;
    margin: 0;
    font-size: inherit;
    font-weight: normal;
    color: var(--accent);
  }
  button {
    min-width: 44px;
    min-height: 44px;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    cursor: pointer;
  }
  button:hover {
    color: var(--fg);
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
  }
  a {
    display: inline-flex;
    align-items: center;
    min-height: 44px;
    color: inherit;
    text-decoration: none;
    overflow-wrap: anywhere;
  }
  .dim {
    margin: 0;
    min-width: 0;
    color: var(--dim);
    overflow-wrap: anywhere;
  }
  .err {
    margin: 0;
    color: var(--err);
  }
</style>
