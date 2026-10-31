<script lang="ts">
  // Command palette: Ctrl+K from anywhere (or `>` in the capture bar) opens it. The input filters
  // the commands and, when not empty, runs a global search (each keystroke aborts the previous one).
  import { tick } from 'svelte';
  import { hitHref, isDone, pieces, search, type Hit } from '../lib/search';
  import { filterCommands, PALETTE_OPEN, type Command } from './commands';

  /** How many search results the palette shows. */
  const LIMIT = 10;

  type Item = { kind: 'command'; command: Command } | { kind: 'hit'; hit: Hit };

  let open = $state(false);
  let text = $state('');
  let hits = $state<Hit[]>([]);
  let error = $state('');
  let selected = $state(0);
  let input = $state<HTMLInputElement>();
  let list = $state<HTMLUListElement>();
  let returnTo: HTMLElement | null = null;
  let pending: AbortController | null = null;

  const items = $derived<Item[]>([
    ...filterCommands(text).map((command) => ({ kind: 'command' as const, command })),
    ...(text.trim() === '' ? [] : hits.map((hit) => ({ kind: 'hit' as const, hit }))),
  ]);

  $effect(() => {
    // Capture phase, so the shortcut works while an input or page handler has focus.
    const onkey = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.altKey && !e.metaKey && (e.code === 'KeyK' || e.key.toLowerCase() === 'k')) {
        e.preventDefault();
        void show(open ? text : '');
      }
    };
    const onopen = (e: Event) => void show((e as CustomEvent<{ text?: string }>).detail?.text ?? '');
    window.addEventListener('keydown', onkey, true);
    window.addEventListener(PALETTE_OPEN, onopen);
    return () => {
      window.removeEventListener('keydown', onkey, true);
      window.removeEventListener(PALETTE_OPEN, onopen);
    };
  });

  async function show(initial: string) {
    if (!open) {
      returnTo = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      open = true;
    }
    if (initial !== text) {
      text = initial;
      void query();
    }
    await tick();
    input?.focus();
  }

  async function close() {
    pending?.abort();
    pending = null;
    open = false;
    text = '';
    hits = [];
    error = '';
    selected = 0;
    await tick();
    returnTo?.focus();
    returnTo = null;
  }

  async function query() {
    pending?.abort();
    pending = null;
    error = '';
    selected = 0;
    const q = text.trim();
    if (q === '') {
      hits = [];
      return;
    }
    const ctrl = new AbortController();
    pending = ctrl;
    try {
      const page = await search(q, { signal: ctrl.signal, limit: LIMIT });
      if (pending !== ctrl) return;
      hits = page.results;
    } catch (e) {
      if (pending !== ctrl || (e instanceof DOMException && e.name === 'AbortError')) return;
      hits = [];
      error = `search: ${e instanceof Error ? e.message : String(e)}`;
    }
  }

  /** Closes (focus back where it was), then runs the item, so a command sees the page as it was. */
  async function run(item: Item | undefined) {
    if (!item) return;
    await close();
    if (item.kind === 'command') item.command.run();
    else location.hash = hitHref(item.hit.object);
  }

  async function move(by: number) {
    if (items.length === 0) return;
    selected = (selected + by + items.length) % items.length;
    await tick();
    list?.querySelector('[aria-selected="true"]')?.scrollIntoView?.({ block: 'nearest' });
  }

  function onkeydown(e: KeyboardEvent) {
    const keys: Record<string, () => unknown> = {
      ArrowDown: () => move(1),
      ArrowUp: () => move(-1),
      Enter: () => run(items[selected]),
      Escape: close,
      // The input is the overlay's only stop, so Tab keeps focus there.
      Tab: () => input?.focus(),
    };
    const act = keys[e.key];
    if (!act) return;
    e.preventDefault();
    e.stopPropagation();
    void act();
  }

  const optionId = (i: number) => `palette-option-${i}`;
</script>

{#if open}
  <div class="palette" role="dialog" aria-label="palette" tabindex="-1" {onkeydown}>
    <div class="prompt">
      <span class="sigil" aria-hidden="true">&gt;</span>
      <input
        bind:this={input}
        bind:value={text}
        oninput={query}
        autocomplete="off"
        spellcheck="false"
        maxlength="200"
        role="combobox"
        aria-label="palette"
        aria-expanded="true"
        aria-controls="palette-options"
        aria-activedescendant={items.length > 0 ? optionId(selected) : undefined}
        placeholder="command or search"
      />
    </div>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
    <ul id="palette-options" role="listbox" aria-label="results" bind:this={list}>
      {#each items as item, i (item.kind === 'command' ? `c ${item.command.label}` : `h ${item.hit.object.id}`)}
        <li
          id={optionId(i)}
          role="option"
          aria-selected={i === selected}
          tabindex="-1"
          onmousedown={(e) => {
            // Runs on press, without moving focus off the input (a tap fires it too).
            e.preventDefault();
            void run(item);
          }}
        >
          {#if item.kind === 'command'}
            <span class="label">{item.command.label}</span>
          {:else}
            {@const o = item.hit.object}
            <span class="head">
              <span class="type">{o.type}</span>
              <span class="label">{o.title}</span>
              {#if o.archived}<span class="mark">[archived]</span>{/if}
              {#if isDone(o)}<span class="mark">[done]</span>{/if}
            </span>
            <span class="snippet"
              >{#each pieces(item.hit.snippet) as p, j (j)}{#if p.hit}<mark>{p.text}</mark>{:else}{p.text}{/if}{/each}</span
            >
          {/if}
        </li>
      {:else}
        <li class="empty" role="presentation">no matches</li>
      {/each}
    </ul>
  </div>
{/if}

<style>
  .palette {
    position: fixed;
    top: 0.5rem;
    left: 0.5rem;
    right: 0.5rem;
    z-index: 10;
    display: flex;
    flex-direction: column;
    max-width: 72rem;
    max-height: calc(100dvh - 1rem);
    margin: 0 auto;
    border: 1px solid var(--accent);
    background: var(--bg);
    box-shadow: 0 0 0 100vmax color-mix(in srgb, var(--bg) 60%, transparent);
  }
  .palette:focus {
    outline: none;
  }
  .prompt {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 44px;
    padding: 0 0.75rem;
    border-bottom: 1px solid var(--line);
  }
  .sigil {
    color: var(--accent);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 44px;
    padding: 0;
    border: 0;
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
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    overflow-y: auto;
  }
  li {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-height: 44px;
    padding: 0.25rem 0.75rem;
    cursor: pointer;
    overflow-wrap: anywhere;
  }
  li[aria-selected='true'] {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  li.empty {
    color: var(--dim);
    cursor: default;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    gap: 0 1ch;
  }
  .type,
  .snippet {
    color: var(--dim);
  }
  .mark {
    color: var(--ok);
  }
  mark {
    background: transparent;
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .err {
    margin: 0;
    padding: 0 0.75rem;
    color: var(--err);
    overflow-wrap: anywhere;
  }
</style>
