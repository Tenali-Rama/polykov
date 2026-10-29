<script lang="ts">
  import { onMount } from 'svelte';
  import { addLink, findByTitle, getLinks, removeLink, type Links, type Summary } from '../lib/links';
  import type { Obj } from '../lib/objects';
  import { objectHref } from './route';

  // Links and backlinks of object `id`. `[ link ]` takes an id, or a title to pick from matches.
  let { id }: { id: number } = $props();

  let links = $state<Links | null>(null);
  let error = $state('');
  let prompting = $state(false);
  let draft = $state('');
  let matches = $state<Obj[] | null>(null);

  onMount(() => act(reload));

  async function reload() {
    links = await getLinks(id);
  }

  async function act(run: () => Promise<void>) {
    error = '';
    try {
      await run();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function open() {
    draft = '';
    matches = null;
    error = '';
    prompting = true;
  }

  function close() {
    prompting = false;
    matches = null;
  }

  function link(to: number) {
    return act(async () => {
      await addLink(id, to);
      close();
      await reload();
    });
  }

  function unlink(to: number) {
    return act(async () => {
      await removeLink(id, to);
      await reload();
    });
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const query = draft.trim();
    if (query === '') return;
    if (/^\d+$/.test(query)) return link(Number(query));
    return act(async () => {
      matches = await findByTitle(query, id);
    });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }
</script>

{#snippet entry(s: Summary)}
  <a href={objectHref(s.id)}>{s.title}</a> <span class="dim">{s.type}</span>
{/snippet}

<section class="links" aria-label="links">
  <p class="label">links</p>
  {#if links}
    <ul>
      {#each links.links as s (s.id)}
        <li>
          {@render entry(s)}
          <button type="button" aria-label={`unlink ${s.title}`} onclick={() => unlink(s.id)}>[ unlink ]</button>
        </li>
      {:else}
        <li class="dim">no links</li>
      {/each}
    </ul>
  {/if}
  {#if !links}
    {#if !error}<p class="dim">loading …</p>{/if}
  {:else if prompting}
    <form class="prompt" onsubmit={submit}>
      <span class="sigil" aria-hidden="true">&gt;</span>
      <input
        bind:value={draft}
        {onkeydown}
        autocomplete="off"
        spellcheck="false"
        aria-label="link to id or title"
        placeholder="id or title"
        {@attach (node) => node.focus()}
      />
    </form>
    {#if matches}
      <ul aria-label="matches">
        {#each matches as m (m.id)}
          <li><button type="button" {onkeydown} onclick={() => link(m.id)}>{m.title}</button> <span class="dim">{m.type} #{m.id}</span></li>
        {:else}
          <li class="dim">no matches</li>
        {/each}
      </ul>
    {/if}
    <p class="dim">enter searches or links an id, esc cancels</p>
  {:else}
    <div><button type="button" onclick={open}>[ link ]</button></div>
  {/if}
  <p class="label">backlinks</p>
  {#if links}
    <ul>
      {#each links.backlinks as s (s.id)}
        <li>{@render entry(s)}</li>
      {:else}
        <li class="dim">no backlinks</li>
      {/each}
    </ul>
  {/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</section>

<style>
  .links {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }
  .label {
    color: var(--dim);
  }
  .label::before {
    content: '── ';
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
    min-height: 44px;
  }
  a {
    color: inherit;
  }
  button {
    min-height: 44px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  button:hover {
    color: var(--fg);
  }
  .prompt {
    display: flex;
    align-items: center;
    gap: 0.5ch;
  }
  .sigil {
    color: var(--accent);
  }
  input {
    flex: 1;
    min-width: 0;
    min-height: 44px;
    border: 0;
    border-bottom: 1px solid var(--line);
    background: transparent;
    color: inherit;
    font: inherit;
    caret-color: var(--fg);
    caret-shape: block;
  }
  input::placeholder {
    color: var(--dim);
  }
  input:focus {
    outline: none;
    border-bottom-color: var(--accent);
  }
  p {
    margin: 0;
  }
  .dim {
    color: var(--dim);
  }
  .err {
    color: var(--err);
  }
</style>
