<script lang="ts">
  import { createObject, TYPES, type Obj } from '../lib/objects';

  let { oncreated }: { oncreated: (obj: Obj) => void } = $props();

  let type = $state('note');
  let title = $state('');
  let error = $state('');

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    try {
      const obj = await createObject({ type, title });
      title = '';
      oncreated(obj);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }
</script>

<form class="create" onsubmit={submit}>
  <select bind:value={type} aria-label="new object type">
    {#each TYPES as t (t)}<option value={t}>{t}</option>{/each}
  </select>
  <span class="sigil" aria-hidden="true">&gt;</span>
  <div class="field">
    <input bind:value={title} autocomplete="off" spellcheck="false" aria-label="new object title" />
    {#if title === ''}
      <span class="placeholder" aria-hidden="true"><span class="cursor"></span>new {type}</span>
    {/if}
  </div>
  <button type="submit">[ add ]</button>
</form>
{#if error}<p class="err" role="alert">{error}</p>{/if}

<style>
  .create {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    column-gap: 0.5rem;
    border: 1px solid var(--line);
    padding: 0 0.25rem 0 0.5rem;
    min-height: 44px;
  }
  .create:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  select,
  button {
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
  .sigil {
    color: var(--accent);
  }
  .field {
    position: relative;
    flex: 1;
    min-width: 8ch;
  }
  input {
    width: 100%;
    min-height: 44px;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    caret-color: var(--fg);
    caret-shape: block;
  }
  input:focus {
    outline: none;
  }
  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    color: var(--dim);
    pointer-events: none;
  }
  .cursor {
    display: inline-block;
    width: 1ch;
    height: 1.2em;
    margin-right: 0.5ch;
    background: var(--fg);
  }
  .field:focus-within .cursor {
    display: none;
  }
  .err {
    margin: 0;
    color: var(--err);
  }
</style>
