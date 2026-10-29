<script lang="ts">
  import type { Snippet } from 'svelte';

  // Box frame shared by the setup and login screens. `onsubmit` runs on Enter or the button.
  type Props = {
    title: string;
    action: string;
    busy: boolean;
    errors: string[];
    note?: string;
    onsubmit: () => void;
    children: Snippet;
  };
  let { title, action, busy, errors, note = '', onsubmit, children }: Props = $props();

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!busy) onsubmit();
  }
</script>

<main class="screen">
  <form class="panel" onsubmit={submit} novalidate>
    <h1>{title}</h1>
    {#if note}<p class="note">{note}</p>{/if}
    {@render children()}
    <div class="errors" aria-live="polite">
      {#each errors as error (error)}
        <p class="err">{error}</p>
      {/each}
    </div>
    <button type="submit" disabled={busy}>[ {busy ? '…' : action} ]</button>
  </form>
</main>

<style>
  .screen {
    display: flex;
    align-items: flex-start;
    justify-content: center;
    min-height: 100dvh;
    padding: 0.5rem;
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    width: 100%;
    max-width: 40rem;
    margin-top: 10vh;
    border: 1px solid var(--line);
    padding: 0.75rem;
  }
  h1 {
    margin: 0;
    font-size: inherit;
    font-weight: normal;
  }
  p {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .note {
    color: var(--ok);
  }
  .err {
    color: var(--err);
  }
  .err::before {
    content: '! ';
  }
  .errors:empty {
    display: none;
  }
  button {
    align-self: flex-start;
    min-height: 44px;
    padding: 0 0.75rem;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
  }
  button:disabled {
    color: var(--dim);
    cursor: default;
  }
</style>
