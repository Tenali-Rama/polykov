<script lang="ts">
  import { untrack } from 'svelte';
  import { ApiError } from '../lib/api';
  import {
    GROUPS,
    LAYOUTS,
    SORTS,
    createView,
    updateView,
    type GroupBy,
    type Layout,
    type Sort,
    type View,
  } from '../lib/views';

  // `view` given: edit it (`PATCH`); else create a new one (`POST`). `oncancel`: Esc or `[ cancel ]`.
  let { view, onsaved, oncancel }: { view?: View; onsaved: (v: View) => void; oncancel: () => void } = $props();

  // The draft starts from `view` once; a save replaces the parent's copy, not this draft.
  const start = untrack(() => view);
  let title = $state(start?.title ?? '');
  let query = $state(start?.properties.query ?? '');
  let layout = $state<Layout>(start?.properties.layout ?? 'list');
  let group = $state<GroupBy>(start?.properties.group ?? 'none');
  let sort = $state<Sort>(start?.properties.sort ?? 'smart');
  let error = $state('');
  // Character offset of a query parse error, marked with `^` under the query.
  let at = $state<number | null>(null);
  let busy = $state(false);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    busy = true;
    error = '';
    at = null;
    try {
      const settings = { title, query, layout, group, sort };
      onsaved(start ? await updateView(start.id, settings) : await createView(settings));
    } catch (err) {
      error = `error: ${err instanceof Error ? err.message : String(err)}`;
      if (err instanceof ApiError && err.at !== undefined) at = err.at;
    } finally {
      busy = false;
    }
  }

  /** Esc cancels, marked handled so the page does not also act on it. */
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    oncancel();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form class="editor" aria-label={start ? 'edit view' : 'new view'} onsubmit={submit} {onkeydown}>
  <label class="line">
    <span class="dim">title</span>
    <input bind:value={title} autocomplete="off" {@attach (node) => node.focus()} />
  </label>
  <div class="query">
    <span class="sigil" aria-hidden="true">&gt;</span>
    <div class="field">
      <input
        bind:value={query}
        aria-label="query"
        aria-invalid={at !== null || undefined}
        aria-describedby={error ? 'view-editor-error' : undefined}
        autocomplete="off"
        spellcheck="false"
        placeholder="type:task -status:done"
      />
      {#if at !== null}<span class="at" aria-hidden="true">{' '.repeat(at)}^</span>{/if}
    </div>
  </div>
  <div class="line">
    <label>layout <select bind:value={layout}>{#each LAYOUTS as l (l)}<option value={l}>{l}</option>{/each}</select></label>
    <label>group <select bind:value={group}>{#each GROUPS as g (g)}<option value={g}>{g}</option>{/each}</select></label>
    <label>sort <select bind:value={sort}>{#each SORTS as s (s)}<option value={s}>{s}</option>{/each}</select></label>
  </div>
  {#if error}<p class="err" id="view-editor-error" role="alert">{error}</p>{/if}
  <div class="line">
    <button type="submit" disabled={busy}>[ save ]</button>
    <button type="button" onclick={oncancel}>[ cancel ]</button>
  </div>
</form>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.5rem;
    border: 1px solid var(--line);
    min-width: 0;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1.5ch;
    min-width: 0;
  }
  label {
    display: flex;
    align-items: center;
    gap: 0.5ch;
    min-height: 44px;
    color: var(--dim);
  }
  label.line {
    gap: 1ch;
  }
  .query {
    display: flex;
    align-items: flex-start;
    gap: 1ch;
    border: 1px solid var(--line);
    padding: 0 0.5ch;
  }
  .query:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .sigil {
    line-height: 44px;
    color: var(--accent);
  }
  .field {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  input {
    flex: 1;
    min-width: 0;
    min-height: 44px;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    caret-shape: block;
  }
  label input {
    border-bottom: 1px solid var(--line);
  }
  .query input:focus {
    outline: none;
  }
  input::placeholder {
    color: var(--dim);
  }
  .at {
    overflow: hidden;
    white-space: pre;
    color: var(--err);
  }
  select,
  button {
    min-height: 44px;
    border: 0;
    background: transparent;
    color: var(--fg);
    font: inherit;
    cursor: pointer;
  }
  button {
    min-width: 44px;
    color: var(--dim);
  }
  button:hover:not(:disabled) {
    color: var(--fg);
  }
  .err {
    margin: 0;
    color: var(--err);
    overflow-wrap: anywhere;
  }
  .dim {
    color: var(--dim);
  }
</style>
