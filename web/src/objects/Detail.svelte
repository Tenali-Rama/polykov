<script lang="ts">
  import { onMount } from 'svelte';
  import { ApiError } from '../lib/api';
  import { getObject, type Obj } from '../lib/objects';
  import { listHref } from './route';

  // Shell re-creates this component per id (`{#key}`), so `id` is read once on mount.
  let { id }: { id: number } = $props();

  let obj = $state<Obj | null>(null);
  let error = $state('');

  onMount(async () => {
    try {
      obj = await getObject(id);
    } catch (e) {
      if (e instanceof ApiError && e.status === 404) error = 'object not found';
      else error = `error: ${e instanceof Error ? e.message : String(e)}`;
    }
  });

  function show(value: unknown): string {
    return typeof value === 'string' ? value : JSON.stringify(value);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') location.hash = listHref;
  }
</script>

<svelte:window {onkeydown} />

<article class="detail">
  <a class="back" href={listHref}>[ esc ] back</a>
  {#if error}
    <p class="err" role="alert">{error}</p>
  {:else if obj}
    <h1>{obj.pinned ? '* ' : ''}{obj.title}</h1>
    <dl>
      <dt>type</dt><dd>{obj.type}</dd>
      <dt>id</dt><dd>{obj.id}</dd>
      <dt>tags</dt><dd>{obj.tags.length > 0 ? obj.tags.map((t) => `#${t}`).join(' ') : '-'}</dd>
      <dt>pinned</dt><dd>{obj.pinned}</dd>
      <dt>archived</dt><dd>{obj.archived}</dd>
      <dt>created</dt><dd>{obj.created}</dd>
      <dt>updated</dt><dd>{obj.updated}</dd>
    </dl>
    <section class="props" aria-label="properties">
      {#each Object.entries(obj.properties) as [key, value] (key)}
        <p>{key}: {show(value)}</p>
      {:else}
        <p class="dim">no properties</p>
      {/each}
    </section>

    <!-- Panels: tasks 023 (links, comments) and 024 (attachments) each add one line here. -->
    <section class="panels" aria-label="panels"></section>
  {:else}
    <p class="dim">loading …</p>
  {/if}
</article>

<style>
  .detail {
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
  dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 0 1ch;
    margin: 0;
  }
  dt {
    color: var(--dim);
  }
  dt::after {
    content: ':';
  }
  dd {
    margin: 0;
  }
  .props {
    border-top: 1px solid var(--line);
    padding-top: 0.5rem;
  }
  p {
    margin: 0;
  }
  .panels:not(:empty) {
    border-top: 1px solid var(--line);
    padding-top: 0.5rem;
  }
  .dim {
    color: var(--dim);
  }
  .err {
    color: var(--err);
  }
</style>
