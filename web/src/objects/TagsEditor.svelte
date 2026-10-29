<script lang="ts">
  // Tags of one object: `[ x ]` removes a tag, the prompt adds one. Each change saves the whole set;
  // the parent shows the server's normalized tags once `onsave` resolves.
  let { tags, onsave }: { tags: string[]; onsave: (tags: string[]) => Promise<void> } = $props();

  let draft = $state('');
  let error = $state('');

  async function save(next: string[]): Promise<boolean> {
    error = '';
    try {
      await onsave(next);
      return true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      return false;
    }
  }

  async function add(e: SubmitEvent) {
    e.preventDefault();
    const tag = draft.trim();
    if (tag === '') return;
    if (await save([...tags, tag])) draft = '';
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && draft !== '') {
      e.preventDefault();
      draft = '';
    }
  }
</script>

<div class="tags">
  {#each tags as tag (tag)}
    <span class="tag">#{tag}<button type="button" aria-label={`remove tag ${tag}`} onclick={() => save(tags.filter((t) => t !== tag))}>[ x ]</button></span>
  {:else}
    <span class="dim">no tags</span>
  {/each}
  <form class="add" onsubmit={add}>
    <span class="sigil" aria-hidden="true">#</span>
    <input bind:value={draft} {onkeydown} autocomplete="off" spellcheck="false" aria-label="add tag" placeholder="add tag" />
  </form>
</div>
{#if error}<p class="err" role="alert">{error}</p>{/if}

<style>
  .tags {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 0.5ch;
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
  .add {
    display: flex;
    align-items: center;
    gap: 0.5ch;
    min-width: 12ch;
    flex: 1;
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
  .dim {
    color: var(--dim);
  }
  .err {
    margin: 0;
    color: var(--err);
  }
</style>
