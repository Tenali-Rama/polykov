<script lang="ts">
  import { onMount } from 'svelte';
  import { addComment, deleteComment, editComment, listComments, type Comment } from '../lib/comments';

  // Comment thread of object `id`, oldest first. Ctrl+Enter submits the prompt or an inline edit.
  let { id }: { id: number } = $props();

  let comments = $state<Comment[] | null>(null);
  let draft = $state('');
  let error = $state('');
  let editing = $state<number | null>(null);
  let editDraft = $state('');
  let editError = $state('');

  onMount(() =>
    act(async () => {
      comments = await listComments(id);
    }),
  );

  function message(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  async function act(run: () => Promise<void>) {
    error = '';
    try {
      await run();
    } catch (e) {
      error = message(e);
    }
  }

  function add() {
    return act(async () => {
      const c = await addComment(id, draft);
      comments = [...(comments ?? []), c];
      draft = '';
    });
  }

  function remove(cid: number) {
    return act(async () => {
      await deleteComment(cid);
      comments = (comments ?? []).filter((c) => c.id !== cid);
    });
  }

  function edit(c: Comment) {
    editing = c.id;
    editDraft = c.body;
    editError = '';
  }

  function cancel() {
    editing = null;
    editError = '';
  }

  async function save(cid: number) {
    editError = '';
    try {
      const c = await editComment(cid, editDraft);
      comments = (comments ?? []).map((old) => (old.id === cid ? c : old));
      editing = null;
    } catch (e) {
      editError = message(e);
    }
  }

  function promptKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && e.ctrlKey) {
      e.preventDefault();
      add();
    } else if (e.key === 'Escape' && draft !== '') {
      // Like TagsEditor: a non-empty prompt claims Esc and clears, so Detail stays open.
      e.preventDefault();
      draft = '';
    }
  }

  function editKey(e: KeyboardEvent, cid: number) {
    if (e.key === 'Escape') {
      e.preventDefault();
      cancel();
    } else if (e.key === 'Enter' && e.ctrlKey) {
      e.preventDefault();
      save(cid);
    }
  }
</script>

<section class="comments" aria-label="comments">
  <p class="label">comments</p>
  {#if comments}
    <ul>
      {#each comments as c (c.id)}
        <li>
          <p class="meta">
            <span class="dim">{c.created}</span>
            {#if editing !== c.id}
              <button type="button" aria-label={`edit comment ${c.id}`} onclick={() => edit(c)}>[ edit ]</button>
              <button type="button" aria-label={`delete comment ${c.id}`} onclick={() => remove(c.id)}>[ delete ]</button>
            {/if}
          </p>
          {#if editing === c.id}
            <textarea
              bind:value={editDraft}
              onkeydown={(e) => editKey(e, c.id)}
              rows={Math.max(2, editDraft.split('\n').length)}
              aria-label={`edit comment ${c.id}`}
              {@attach (node) => node.focus()}
            ></textarea>
            <p class="dim">ctrl+enter saves, esc cancels</p>
            {#if editError}<p class="err" role="alert">{editError}</p>{/if}
          {:else}
            <p class="body">{c.body}</p>
          {/if}
        </li>
      {:else}
        <li class="dim">no comments</li>
      {/each}
    </ul>
  {/if}
  <div class="prompt">
    <span class="sigil" aria-hidden="true">&gt;</span>
    <textarea bind:value={draft} onkeydown={promptKey} rows="2" aria-label="add comment" placeholder="add comment"></textarea>
  </div>
  <p class="actions">
    <button type="button" onclick={add}>[ add ]</button>
    <span class="dim">ctrl+enter adds</span>
  </p>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</section>

<style>
  .comments {
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
    padding-bottom: 0.25rem;
  }
  .meta,
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
  }
  .body {
    white-space: pre-wrap;
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
  .prompt {
    display: flex;
    align-items: flex-start;
    gap: 0.5ch;
  }
  .sigil {
    color: var(--accent);
  }
  textarea {
    flex: 1;
    width: 100%;
    min-width: 0;
    min-height: 44px;
    box-sizing: border-box;
    border: 1px solid var(--line);
    background: transparent;
    color: inherit;
    font: inherit;
    caret-color: var(--fg);
    caret-shape: block;
    resize: vertical;
  }
  textarea::placeholder {
    color: var(--dim);
  }
  textarea:focus {
    outline: none;
    border-color: var(--accent);
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
