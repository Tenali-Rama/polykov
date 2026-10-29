<script lang="ts">
  import { onMount } from 'svelte';
  import { ApiError } from '../lib/api';
  import { deleteObject, getObject, updateObject, type Obj, type Patch } from '../lib/objects';
  import Attachments from './Attachments.svelte';
  import Comments from './Comments.svelte';
  import Links from './Links.svelte';
  import PropsEditor from './PropsEditor.svelte';
  import TagsEditor from './TagsEditor.svelte';
  import { listHref } from './route';

  // Shell re-creates this component per id (`{#key}`), so `id` is read once on mount.
  let { id, ondeleted }: { id: number; ondeleted: () => void } = $props();

  let obj = $state<Obj | null>(null);
  let error = $state('');
  let editingTitle = $state(false);
  let titleDraft = $state('');
  let titleError = $state('');
  let actionError = $state('');

  onMount(async () => {
    try {
      obj = await getObject(id);
    } catch (e) {
      if (e instanceof ApiError && e.status === 404) error = 'object not found';
      else error = `error: ${message(e)}`;
    }
  });

  function message(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  /** One `PATCH`; the server's object (normalized tags, new `updated`) replaces ours. Throws `ApiError`. */
  async function save(patch: Patch) {
    obj = await updateObject(id, patch);
  }

  function editTitle(current: string) {
    titleDraft = current;
    titleError = '';
    editingTitle = true;
  }

  async function titleKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      editingTitle = false;
      titleError = '';
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (titleDraft === obj?.title) {
        editingTitle = false;
        return;
      }
      try {
        await save({ title: titleDraft });
        editingTitle = false;
        titleError = '';
      } catch (err) {
        titleError = message(err);
      }
    }
  }

  async function act(run: () => Promise<void>) {
    actionError = '';
    try {
      await run();
    } catch (e) {
      actionError = message(e);
    }
  }

  function remove() {
    return act(async () => {
      await deleteObject(id);
      ondeleted();
    });
  }

  function onkeydown(e: KeyboardEvent) {
    // Editors call `preventDefault` on an Esc they used to cancel.
    if (e.key === 'Escape' && !e.defaultPrevented) location.hash = listHref;
  }
</script>

<svelte:window {onkeydown} />

<article class="detail">
  <a class="back" href={listHref}>[ esc ] back</a>
  {#if error}
    <p class="err" role="alert">{error}</p>
  {:else if obj}
    {@const o = obj}
    {#if editingTitle}
      <input
        class="title"
        bind:value={titleDraft}
        onkeydown={titleKey}
        autocomplete="off"
        spellcheck="false"
        aria-label="title"
        {@attach (node) => node.focus()}
      />
      <p class="dim">enter saves, esc cancels</p>
    {:else}
      <h1>
        <button type="button" class="title" title="edit title" onclick={() => editTitle(o.title)}
          >{o.pinned ? '* ' : ''}{o.title}</button
        >
      </h1>
    {/if}
    {#if titleError}<p class="err" role="alert">{titleError}</p>{/if}

    <div class="actions">
      <button type="button" onclick={() => act(() => save({ pinned: !o.pinned }))}>[ {o.pinned ? 'unpin' : 'pin'} ]</button>
      <button type="button" onclick={() => act(() => save({ archived: !o.archived }))}
        >[ {o.archived ? 'unarchive' : 'archive'} ]</button
      >
      <button type="button" onclick={remove}>[ delete ]</button>
    </div>
    {#if actionError}<p class="err" role="alert">{actionError}</p>{/if}

    <dl>
      <dt>type</dt><dd>{o.type}</dd>
      <dt>id</dt><dd>{o.id}</dd>
      <dt>tags</dt><dd><TagsEditor tags={o.tags} onsave={(tags) => save({ tags })} /></dd>
      <dt>pinned</dt><dd>{o.pinned}</dd>
      <dt>archived</dt><dd>{o.archived}</dd>
      <dt>created</dt><dd>{o.created}</dd>
      <dt>updated</dt><dd>{o.updated}</dd>
    </dl>
    <PropsEditor properties={o.properties} onsave={(properties) => save({ properties })} />

    <!-- Panels: tasks 023 (links, comments) and 024 (attachments) each add one line here. -->
    <section class="panels" aria-label="panels">
      <Links {id} />
      <Comments {id} />
      <Attachments {id} />
    </section>
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
  button.title {
    color: inherit;
    text-align: left;
    overflow-wrap: anywhere;
  }
  input.title {
    min-height: 44px;
    border: 0;
    border-bottom: 1px solid var(--accent);
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 1.2rem;
    font-weight: bold;
    caret-color: var(--fg);
    caret-shape: block;
  }
  input.title:focus {
    outline: none;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0 1ch;
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
  p {
    margin: 0;
  }
  .panels:not(:empty) {
    display: flex;
    flex-direction: column;
    gap: 1rem;
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
