<script lang="ts">
  import { onMount } from 'svelte';
  import { serverVersion } from './lib/api';

  let status = $state('server …');
  let ok = $state<boolean | null>(null);
  let command = $state('');

  onMount(async () => {
    const version = await serverVersion();
    ok = version !== null;
    status = ok ? `server ok ${version}` : 'server unreachable';
  });
</script>

<main class="shell">
  <header class="bar">
    <span>polykov</span>
    <span class="status" class:ok={ok === true} class:err={ok === false} role="status">{status}</span>
  </header>

  <section class="body">
    <p class="dim">nothing here yet</p>
  </section>

  <form class="prompt" onsubmit={(e) => e.preventDefault()}>
    <span class="sigil" aria-hidden="true">&gt;</span>
    <div class="field">
      <input id="command" bind:value={command} autocomplete="off" spellcheck="false" aria-label="command" />
      {#if command === ''}
        <span class="placeholder" aria-hidden="true"><span class="cursor"></span>type a command</span>
      {/if}
    </div>
  </form>
</main>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    min-height: 100dvh;
    max-width: 72rem;
    margin: 0 auto;
    padding: 0.5rem;
    gap: 0.5rem;
  }
  .bar,
  .body,
  .prompt {
    border: 1px solid var(--line);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.25rem 0.75rem;
  }
  .status {
    color: var(--dim);
    overflow-wrap: anywhere;
  }
  .status.ok {
    color: var(--ok);
  }
  .status.err {
    color: var(--err);
  }
  .body {
    flex: 1;
    padding: 0.75rem;
  }
  .dim {
    color: var(--dim);
    margin: 0;
  }
  .prompt {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.75rem;
    min-height: 44px;
  }
  .sigil {
    color: var(--accent);
  }
  .field {
    position: relative;
    flex: 1;
    min-width: 0;
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
  .prompt:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
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
    animation: blink 1s steps(1) infinite;
  }
  .field:focus-within .cursor {
    display: none;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .cursor {
      animation: none;
    }
  }
</style>
