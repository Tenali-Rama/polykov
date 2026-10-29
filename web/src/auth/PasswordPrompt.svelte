<script lang="ts">
  import { onMount } from 'svelte';

  type Props = {
    id: string;
    label: string;
    value: string;
    autocomplete?: 'current-password' | 'new-password';
    autofocus?: boolean;
    invalid?: boolean;
  };
  let {
    id,
    label,
    value = $bindable(),
    autocomplete = 'current-password',
    autofocus = false,
    invalid = false,
  }: Props = $props();

  let input: HTMLInputElement;

  onMount(() => {
    if (autofocus) input.focus();
  });
</script>

<div class="prompt">
  <span class="sigil" aria-hidden="true">&gt;</span>
  <label for={id}>{label}</label>
  <div class="field">
    <input
      {id}
      bind:this={input}
      bind:value
      type="password"
      {autocomplete}
      aria-invalid={invalid}
    />
    {#if value === ''}
      <span class="cursor" aria-hidden="true"></span>
    {/if}
  </div>
</div>

<style>
  .prompt {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    column-gap: 0.5rem;
    border: 1px solid var(--line);
    padding: 0 0.75rem;
    min-height: 44px;
  }
  .prompt:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  label {
    color: var(--dim);
    white-space: nowrap;
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
  .cursor {
    position: absolute;
    left: 0;
    top: 50%;
    width: 1ch;
    height: 1.2em;
    transform: translateY(-50%);
    background: var(--fg);
    pointer-events: none;
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
