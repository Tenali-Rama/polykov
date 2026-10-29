<script module lang="ts">
  export type Props = Record<string, unknown>;

  /** A value as shown on a `key: value` line: strings as-is, anything else as JSON. */
  export function show(value: unknown): string {
    return typeof value === 'string' ? value : JSON.stringify(value);
  }

  export function formatProps(props: Props): string {
    return Object.entries(props)
      .map(([key, value]) => `${key}: ${show(value)}`)
      .join('\n');
  }

  /** Like the CLI's `--prop` (018): an integer or float, then `true`/`false`, else a string. */
  export function parseValue(raw: string): unknown {
    const n = /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(raw) ? Number(raw) : NaN;
    if (Number.isFinite(n)) return n;
    if (raw === 'true') return true;
    if (raw === 'false') return false;
    return raw;
  }

  /**
   * Parses `key: value` lines (blank lines skipped). A line whose value text is unchanged from
   * `before` keeps its old value, so arrays, `null` and numeric-looking strings survive a round trip.
   */
  export function parseProps(text: string, before: Props): { props: Props } | { error: string } {
    const props: Props = {};
    const lines = text.split('\n');
    for (const [i, line] of lines.entries()) {
      if (line.trim() === '') continue;
      const at = line.indexOf(':');
      if (at < 0) return { error: `line ${i + 1}: expected key: value` };
      const key = line.slice(0, at).trim();
      const raw = line.slice(at + 1).trim();
      if (key === '') return { error: `line ${i + 1}: missing key` };
      if (Object.hasOwn(props, key)) return { error: `line ${i + 1}: duplicate key "${key}"` };
      props[key] = Object.hasOwn(before, key) && show(before[key]) === raw ? before[key] : parseValue(raw);
    }
    return { props };
  }
</script>

<script lang="ts">
  let { properties, onsave }: { properties: Props; onsave: (props: Props) => Promise<void> } = $props();

  let editing = $state(false);
  let text = $state('');
  let error = $state('');

  function edit() {
    text = formatProps(properties);
    error = '';
    editing = true;
  }

  function cancel() {
    editing = false;
    error = '';
  }

  async function save() {
    const parsed = parseProps(text, properties);
    if ('error' in parsed) {
      error = parsed.error;
      return;
    }
    error = '';
    try {
      await onsave(parsed.props);
      editing = false;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      cancel();
    } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      void save();
    }
  }
</script>

<section class="props" aria-label="properties">
  {#if editing}
    <textarea
      bind:value={text}
      {onkeydown}
      rows={Math.max(3, text.split('\n').length + 1)}
      spellcheck="false"
      aria-label="properties as key: value lines"
      {@attach (node) => node.focus()}
    ></textarea>
    <div class="actions">
      <button type="button" onclick={save}>[ save ]</button>
      <button type="button" onclick={cancel}>[ cancel ]</button>
      <span class="dim">ctrl+enter saves, esc cancels</span>
    </div>
  {:else}
    {#each Object.entries(properties) as [key, value] (key)}
      <p>{key}: {show(value)}</p>
    {:else}
      <p class="dim">no properties</p>
    {/each}
    <div class="actions"><button type="button" onclick={edit}>[ edit properties ]</button></div>
  {/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</section>

<style>
  .props {
    border-top: 1px solid var(--line);
    padding-top: 0.5rem;
  }
  textarea {
    width: 100%;
    border: 1px solid var(--line);
    background: transparent;
    color: inherit;
    font: inherit;
    caret-color: var(--fg);
    caret-shape: block;
    resize: vertical;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
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
