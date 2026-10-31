<script lang="ts">
  // Global capture bar: Ctrl+Space from anywhere opens it; each keystroke re-parses the line
  // (cancelling the previous parse) to highlight tokens; Enter saves through smart add. A `>`
  // typed into the empty line hands over to the command palette.
  import { tick } from 'svelte';
  import { add, landing, localToday, parse, type Parsed, type Token } from '../lib/smartadd';
  import { CAPTURE_OPEN, PALETTE_OPEN } from '../palette/commands';
  import { chips, quote, segments } from './highlight';

  const NOTE_MS = 4000;

  let open = $state(false);
  let text = $state('');
  // The latest applied parse and the text it was for; highlighting needs offsets that match.
  let parsed = $state<Parsed | null>(null);
  let parsedText = $state('');
  // The text before the latest input, to tell a `>` typed into an empty line.
  let before = '';
  let error = $state('');
  let note = $state('');
  let saving = false;
  let input = $state<HTMLInputElement>();
  let mirror = $state<HTMLDivElement>();
  let returnTo: HTMLElement | null = null;
  let pending: AbortController | null = null;
  let noteTimer: ReturnType<typeof setTimeout> | undefined;

  // Refreshed on each open, so a tab left open past midnight labels days right.
  let today = $state(localToday());
  const shown = $derived(segments(text, parsed && parsedText === text ? parsed.tokens : []));
  const chipList = $derived(parsed ? chips(parsed, today) : []);

  $effect(() => {
    // Capture phase, so the shortcut works while another input or page handler has focus.
    const onkey = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.altKey && !e.metaKey && (e.code === 'Space' || e.key === ' ')) {
        e.preventDefault();
        void show();
      }
    };
    const onopen = () => void show();
    window.addEventListener('keydown', onkey, true);
    window.addEventListener(CAPTURE_OPEN, onopen);
    return () => {
      window.removeEventListener('keydown', onkey, true);
      window.removeEventListener(CAPTURE_OPEN, onopen);
    };
  });

  async function show() {
    if (!open) {
      returnTo = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      today = localToday();
      open = true;
    }
    await tick();
    input?.focus();
  }

  async function close() {
    pending?.abort();
    pending = null;
    open = false;
    text = '';
    before = '';
    parsed = null;
    parsedText = '';
    error = '';
    await tick();
    returnTo?.focus();
    returnTo = null;
  }

  function oninput() {
    const was = before;
    before = text;
    if (was === '' && text.startsWith('>')) void toPalette(text.slice(1));
    else void reparse();
  }

  /** Closes the bar and opens the palette with `rest`; the palette's Esc returns to where the bar was opened from. */
  async function toPalette(rest: string) {
    const back = returnTo;
    returnTo = null;
    back?.focus();
    window.dispatchEvent(new CustomEvent(PALETTE_OPEN, { detail: { text: rest } }));
    await close();
  }

  async function reparse() {
    pending?.abort();
    pending = null;
    error = '';
    syncScroll();
    if (text.trim() === '') {
      parsed = null;
      return;
    }
    const ctrl = new AbortController();
    pending = ctrl;
    const sent = text;
    try {
      const result = await parse(sent, ctrl.signal);
      if (pending !== ctrl) return;
      parsed = result;
      parsedText = sent;
    } catch (e) {
      if (pending !== ctrl || (e instanceof DOMException && e.name === 'AbortError')) return;
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (text.trim() === '' || saving) return;
    saving = true;
    pending?.abort();
    try {
      const task = await add(text);
      const list = landing(task.task, today);
      showNote(list === 'inbox' ? `added to inbox: ${task.title}` : `added: ${task.title} · ${list}`);
      window.dispatchEvent(new CustomEvent('polykov:tasks-changed'));
      await close();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      saving = false;
    }
  }

  function showNote(message: string) {
    clearTimeout(noteTimer);
    note = message;
    noteTimer = setTimeout(() => (note = ''), NOTE_MS);
  }

  async function unparse(token: Token) {
    if (parsedText !== text) return;
    text = quote(text, token);
    await tick();
    input?.focus();
    input?.setSelectionRange(text.length, text.length);
    void reparse();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Tab') return wrapFocus(e);
    if (e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    void close();
  }

  // Tab cycles within the open bar (input, then each chip's `[ x ]`) instead of reaching the page.
  function wrapFocus(e: KeyboardEvent) {
    const dialog = e.currentTarget as HTMLElement;
    const stops = [...dialog.querySelectorAll<HTMLElement>('input, button')];
    const first = stops[0];
    const last = stops[stops.length - 1];
    const target = e.shiftKey ? (document.activeElement === first ? last : null) : document.activeElement === last ? first : null;
    if (!target) return;
    e.preventDefault();
    target.focus();
  }

  // The mirror follows the input's own horizontal scroll, so long lines stay aligned.
  function syncScroll() {
    if (input && mirror) mirror.scrollLeft = input.scrollLeft;
  }
</script>

{#if note}<p class="note" role="status">{note}</p>{/if}

{#if open}
  <div class="capture" role="dialog" aria-label="capture" tabindex="-1" {onkeydown}>
    <form class="prompt" onsubmit={save}>
      <span class="sigil" aria-hidden="true">&gt;</span>
      <div class="field">
        <div class="mirror" aria-hidden="true" bind:this={mirror}>
          <span class="line"
            >{#each shown as seg, i (i)}{#if seg.kind}<span class="tok {seg.kind}" class:unresolved={!seg.resolved}
                  >{seg.text}</span
                >{:else}{seg.text}{/if}{/each}</span
          >
        </div>
        <input
          bind:this={input}
          bind:value={text}
          {oninput}
          onscroll={syncScroll}
          onkeyup={syncScroll}
          onclick={syncScroll}
          autocomplete="off"
          spellcheck="false"
          aria-label="capture"
          placeholder="add a task"
        />
      </div>
    </form>
    {#if chipList.length > 0}
      <ul class="chips" aria-label="parsed">
        {#each chipList as chip (chip.token.start)}
          <li class="chip {chip.token.kind}" class:unresolved={!chip.token.resolved}>
            {chip.label}<button type="button" aria-label={`unparse ${chip.token.text}`} onclick={() => unparse(chip.token)}
              >[ x ]</button
            >
          </li>
        {/each}
      </ul>
    {/if}
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </div>
{/if}

<style>
  .note {
    margin: 0;
    color: var(--ok);
    overflow-wrap: anywhere;
  }
  .capture {
    position: fixed;
    top: 0.5rem;
    left: 0.5rem;
    right: 0.5rem;
    z-index: 10;
    max-width: 72rem;
    margin: 0 auto;
    padding: 0 0.75rem;
    border: 1px solid var(--accent);
    background: var(--bg);
    box-shadow: 0 0 0 100vmax color-mix(in srgb, var(--bg) 60%, transparent);
  }
  .capture:focus {
    outline: none;
  }
  .prompt {
    display: flex;
    align-items: center;
    gap: 0.5rem;
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
  .line,
  input {
    font: inherit;
    letter-spacing: normal;
    white-space: pre;
  }
  /* A block, not flex: flex items would drop the whitespace-only runs between tokens. The line
     box is the input's height, so the text sits where the input's own text does. */
  .mirror {
    position: absolute;
    inset: 0;
    overflow: hidden;
    white-space: nowrap;
    line-height: 44px;
    pointer-events: none;
  }
  input {
    position: relative;
    width: 100%;
    height: 44px;
    padding: 0;
    border: 0;
    background: transparent;
    color: transparent;
    caret-color: var(--fg);
    caret-shape: block;
  }
  input::placeholder {
    color: var(--dim);
  }
  input:focus {
    outline: none;
  }
  .tok {
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .date,
  .time,
  .deadline,
  .recurrence {
    color: var(--accent);
  }
  .duration,
  .priority,
  .tag,
  .project,
  .area {
    color: var(--ok);
  }
  .unresolved {
    color: var(--err);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    margin: 0;
    padding: 0;
    list-style: none;
    border-top: 1px solid var(--line);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.5ch;
    overflow-wrap: anywhere;
  }
  .chip + .chip::before {
    content: '·';
    color: var(--dim);
    margin-right: 0.5ch;
  }
  .chip button {
    min-width: 44px;
    min-height: 44px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    cursor: pointer;
  }
  .chip button:hover {
    color: var(--fg);
  }
  .err {
    margin: 0 0 0.5rem;
    color: var(--err);
    overflow-wrap: anywhere;
  }
</style>
