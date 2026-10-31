<script lang="ts">
  import { onMount } from 'svelte';
  import { ApiError } from '../lib/api';
  import { getObject } from '../lib/objects';
  import { updateView, type View } from '../lib/views';
  import { viewSource } from './source';
  import { OBJECTS_CHANGED } from '../favourites/Favourites.svelte';
  import ListLayout from './ListLayout.svelte';
  import Eisenhower from './Eisenhower.svelte';
  import Kanban from './Kanban.svelte';
  import ViewEditor from './ViewEditor.svelte';

  // `id` is read once; Shell re-creates the page per id.
  let { id }: { id: number } = $props();

  let view = $state<View | null>(null);
  let error = $state('');
  let editing = $state(false);
  // Bumped after an edit, so the layout remounts and loads the new results.
  let revision = $state(0);
  let editButton = $state<HTMLButtonElement>();

  onMount(async () => {
    try {
      const obj = await getObject(id);
      if (obj.type !== 'view') throw new ApiError(404, 'not a view');
      view = obj as View;
    } catch (e) {
      error = e instanceof ApiError && e.status === 404 ? 'view not found' : `error: ${message(e)}`;
    }
  });

  function message(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  async function togglePin() {
    if (!view) return;
    error = '';
    try {
      view = await updateView(id, { pinned: !view.pinned });
      window.dispatchEvent(new Event(OBJECTS_CHANGED));
    } catch (e) {
      error = `error: ${message(e)}`;
    }
  }

  function saved(v: View) {
    view = v;
    editing = false;
    revision += 1;
    editButton?.focus();
  }

  function cancel() {
    editing = false;
    editButton?.focus();
  }
</script>

<section class="page" aria-label="view">
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if view}
    <div class="head">
      <h1>{view.title}</h1>
      <button type="button" onclick={togglePin}>[ {view.pinned ? 'unpin' : 'pin'} ]</button>
      <button type="button" aria-expanded={editing} bind:this={editButton} onclick={() => (editing = !editing)}
        >[ edit ]</button
      >
    </div>
    <p class="dim query">&gt; {view.properties.query}</p>
    {#if editing}<ViewEditor {view} onsaved={saved} oncancel={cancel} />{/if}
    {#key revision}
      <!-- One branch per layout (046 kanban, 047 eisenhower); the rest fall back to the list. -->
      {#if view.properties.layout === 'list'}
        <ListLayout source={viewSource(view)} />
      {:else if view.properties.layout === 'eisenhower'}
        <Eisenhower source={viewSource(view)} />
      {:else if view.properties.layout === 'kanban'}
        <Kanban source={viewSource(view)} />
      {:else}
        <ListLayout source={viewSource(view)} />
      {/if}
    {/key}
  {/if}
</section>

<style>
  .page {
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
    flex: 1 1 12ch;
    min-width: 0;
    margin: 0;
    font-size: inherit;
    font-weight: normal;
    color: var(--accent);
    overflow-wrap: anywhere;
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
  .dim {
    margin: 0;
    color: var(--dim);
  }
  .query {
    overflow-wrap: anywhere;
  }
  .err {
    margin: 0;
    color: var(--err);
  }
</style>
