<script lang="ts">
  import { onMount } from 'svelte';
  import { listTags, type TagCount } from '../lib/objects';

  let { selected, onselect }: { selected: string; onselect: (tag: string) => void } = $props();

  let tags = $state<TagCount[]>([]);
  let error = $state('');

  onMount(async () => {
    try {
      tags = await listTags();
    } catch (e) {
      error = `tags unavailable: ${e instanceof Error ? e.message : String(e)}`;
    }
  });

  function depth(tag: string): number {
    return tag.split('/').length - 1;
  }

  function leaf(tag: string): string {
    return tag.slice(tag.lastIndexOf('/') + 1);
  }
</script>

<nav class="tags" aria-label="tag folders">
  <button type="button" class:sel={selected === ''} aria-pressed={selected === ''} onclick={() => onselect('')}>
    all
  </button>
  {#each tags as t (t.tag)}
    <button
      type="button"
      class:sel={selected === t.tag}
      aria-pressed={selected === t.tag}
      title={t.tag}
      style:padding-left="{depth(t.tag) * 2 + 0.5}ch"
      onclick={() => onselect(t.tag)}
    >
      <span class="name">{leaf(t.tag)}/</span><span class="count">{t.count}</span>
    </button>
  {/each}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</nav>

<style>
  .tags {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  button {
    display: flex;
    justify-content: space-between;
    gap: 1ch;
    min-height: 44px;
    padding: 0 0.5ch;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  button:hover,
  button.sel {
    color: var(--fg);
  }
  button.sel .name {
    color: var(--accent);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .err {
    margin: 0;
    color: var(--err);
  }
</style>
