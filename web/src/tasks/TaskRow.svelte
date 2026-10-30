<script lang="ts">
  import { completeTask, reopenTask, type Row } from '../lib/tasks';
  import { objectHref } from '../objects/route';

  // `today`: the local date, for the overdue mark. `selected`/`onfocus`: the list's j/k selection.
  let {
    row,
    today,
    selected = false,
    onfocus,
  }: { row: Row; today: string; selected?: boolean; onfocus?: () => void } = $props();

  // Follows `row` until a toggle here overrides it.
  let done = $derived(row.task.status === 'done');
  // `done · next <date>` after completing a recurring task.
  let next = $state('');
  let error = $state('');
  let busy = false;
  let link: HTMLAnchorElement;

  const t = $derived(row.task);
  const overdue = $derived(t.deadline !== null && t.deadline.slice(0, 10) < today);

  /** Completes an open task, reopens a done one. */
  export async function toggle() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      if (done) {
        await reopenTask(row.id);
        done = false;
        next = '';
      } else {
        const res = await completeTask(row.id);
        done = true;
        const n = res.next?.task;
        if (n) next = `done · next ${(n.scheduled ?? n.deadline ?? '').slice(0, 10)}`;
      }
    } catch (e) {
      error = `error: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      busy = false;
    }
  }

  export function focus() {
    link.focus();
  }
</script>

<li class="row" class:sel={selected} class:done>
  <button
    type="button"
    class="check"
    aria-label={done ? `reopen ${row.title}` : `complete ${row.title}`}
    onclick={toggle}>{done ? '[x]' : '[ ]'}</button
  >
  <div class="main">
    <a
      class="title"
      href={objectHref(row.id)}
      bind:this={link}
      aria-current={selected ? 'true' : undefined}
      {onfocus}
    >
      {#if t.priority < 4}<span class="prio">p{t.priority}</span>{/if}
      {row.title}
    </a>
    <span class="meta">
      {#if next}<span class="note" role="status">{next}</span>{/if}
      {#if t.scheduled}<span>{t.scheduled}</span>{/if}
      {#if t.deadline}<span class:err={overdue}>due {t.deadline}</span>{/if}
      {#if row.subtasks.total > 0}<span>[{row.subtasks.open}/{row.subtasks.total}]</span>{/if}
      {#if row.tags.length > 0}<span class="tags">{row.tags.map((tag) => `#${tag}`).join(' ')}</span>{/if}
    </span>
    {#if error}<span class="err" role="alert">{error}</span>{/if}
  </div>
</li>

<style>
  .row {
    display: flex;
    align-items: flex-start;
    gap: 0 1ch;
  }
  .row.sel {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .check {
    flex: none;
    min-width: 44px;
    min-height: 44px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    white-space: pre;
    cursor: pointer;
  }
  .check:hover,
  .done .check {
    color: var(--ok);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
  }
  .title {
    flex: 1 1 20ch;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 1ch;
    min-height: 44px;
    color: inherit;
    text-decoration: none;
    overflow-wrap: anywhere;
  }
  .done .title {
    color: var(--dim);
    text-decoration: line-through;
  }
  .prio {
    color: var(--accent);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0 1ch;
    min-width: 0;
    color: var(--dim);
  }
  .meta > span {
    overflow-wrap: anywhere;
    min-width: 0;
  }
  .note {
    color: var(--ok);
  }
  .err {
    color: var(--err);
  }
  /* Phone: the meta goes on its own line under the title. */
  @media (max-width: 40rem) {
    .meta {
      flex-basis: 100%;
      padding-bottom: 0.25rem;
    }
  }
</style>
