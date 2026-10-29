<script lang="ts">
  import { onMount } from 'svelte';
  import { ApiError } from '../lib/api';
  import {
    deleteAttachment,
    downloadUrl,
    humanSize,
    listAttachments,
    uploadAttachment,
    type Attachment,
  } from '../lib/attachments';

  let { id }: { id: number } = $props();

  let attachments = $state<Attachment[]>([]);
  let loading = $state(true);
  let error = $state('');
  let uploading = $state('');
  let confirming = $state<number | null>(null);
  let dragging = $state(false);
  let picker: HTMLInputElement;
  // The row whose `[ delete ]` takes focus back after a cancelled confirm.
  let refocus: number | null = null;

  onMount(async () => {
    try {
      attachments = await listAttachments(id);
    } catch (e) {
      error = message(e);
    } finally {
      loading = false;
    }
  });

  function message(e: unknown): string {
    if (e instanceof ApiError && e.status === 413) return 'file too large (max 25 MiB)';
    return e instanceof Error ? e.message : String(e);
  }

  /** One request per file, in order; the first failure stops the rest. */
  async function upload(files: File[]) {
    error = '';
    try {
      for (const file of files) {
        uploading = file.name;
        attachments = [...attachments, await uploadAttachment(id, file)];
      }
    } catch (e) {
      error = message(e);
    } finally {
      uploading = '';
    }
  }

  function picked() {
    const files = [...(picker.files ?? [])];
    picker.value = '';
    if (files.length > 0) void upload(files);
  }

  function ondragover(e: DragEvent) {
    e.preventDefault();
    dragging = true;
  }

  function ondrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    if (uploading !== '') return;
    const files = [...(e.dataTransfer?.files ?? [])];
    if (files.length > 0) void upload(files);
  }

  async function remove(a: Attachment) {
    error = '';
    try {
      await deleteAttachment(a.id);
      attachments = attachments.filter((x) => x.id !== a.id);
    } catch (e) {
      error = message(e);
    } finally {
      confirming = null;
    }
  }

  function cancel() {
    refocus = confirming;
    confirming = null;
  }

  /** Esc cancels a pending confirm; `preventDefault` keeps Detail from leaving the page. */
  function confirmKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      cancel();
    }
  }

  function focusIfRefocus(node: HTMLElement, aid: number) {
    if (refocus !== aid) return;
    refocus = null;
    node.focus();
  }
</script>

<section
  class="attachments"
  class:dragging
  aria-label="attachments"
  {ondragover}
  ondragleave={() => (dragging = false)}
  {ondrop}
>
  <p class="label">attachments</p>
  {#if loading}
    <p class="dim">loading …</p>
  {:else if attachments.length === 0}
    <p class="dim">no attachments · drop files here</p>
  {:else}
    <ul class="rows">
      {#each attachments as a (a.id)}
        <li>
          {#if a.mime.startsWith('image/')}
            <img class="thumb" src={downloadUrl(a.id, true)} alt={a.name} loading="lazy" />
          {/if}
          <span class="name">{a.name}</span>
          <span class="dim">{humanSize(a.size)}</span>
          <span class="dim">{a.mime}</span>
          {#if confirming === a.id}
            <span class="ask">delete {a.name}?</span>
            <button type="button" onclick={() => remove(a)} onkeydown={confirmKey}>[ yes ]</button>
            <button
              type="button"
              onclick={cancel}
              onkeydown={confirmKey}
              {@attach (node) => node.focus()}>[ no ]</button
            >
          {:else}
            <a href={downloadUrl(a.id)} download={a.name}>[ download ]</a>
            <button
              type="button"
              onclick={() => (confirming = a.id)}
              {@attach (node) => focusIfRefocus(node, a.id)}>[ delete ]</button
            >
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  <div class="add">
    <button type="button" onclick={() => picker.click()} disabled={uploading !== ''}>[ attach ]</button>
    <input type="file" multiple hidden bind:this={picker} onchange={picked} aria-label="files to attach" />
    {#if uploading}<span class="dim" role="status">uploading {uploading}…</span>{/if}
  </div>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</section>

<style>
  .attachments {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
    border: 1px dashed transparent;
  }
  .dragging {
    border-color: var(--accent);
  }
  .label {
    color: var(--dim);
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
  .add {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
  }
  .rows li {
    border-bottom: 1px solid var(--line);
  }
  /* A real basis: at phone width the size, mime and actions wrap under the name instead of squeezing it. */
  .name {
    flex: 1 1 14ch;
    min-width: 14ch;
    overflow-wrap: anywhere;
  }
  .thumb {
    width: 44px;
    height: 44px;
    object-fit: cover;
    border: 1px solid var(--line);
  }
  a,
  button {
    display: inline-flex;
    align-items: center;
    min-height: 44px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    text-decoration: none;
    cursor: pointer;
  }
  a:hover,
  button:hover {
    color: var(--fg);
  }
  button:disabled {
    cursor: default;
  }
  .ask,
  .err {
    color: var(--err);
  }
  .dim {
    color: var(--dim);
  }
</style>
