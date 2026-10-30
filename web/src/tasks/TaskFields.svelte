<script lang="ts">
  import { onMount } from 'svelte';
  import { findByTitle } from '../lib/links';
  import { getObject, listObjects, type Obj } from '../lib/objects';
  import { listProjects, listSections, type Project, type Section } from '../lib/projects';
  import {
    completeTask,
    formatEstimate,
    getTask,
    joinDate,
    parseEstimate,
    reopenTask,
    splitDate,
    updateTask,
    type FieldChanges,
    type Task,
  } from '../lib/tasks';
  import { objectHref } from '../objects/route';
  import Recurrence from './Recurrence.svelte';

  // The task fields of task `id`, each edited in place; every change is one `PATCH /api/tasks/{id}`.
  let { id }: { id: number } = $props();

  type Field = keyof FieldChanges | 'status';
  let task = $state<Task | null>(null);
  let projects = $state<Project[]>([]);
  let areas = $state<Obj[]>([]);
  let sections = $state<Section[]>([]);
  let parent = $state<Obj | null>(null);
  let next = $state<Task | null>(null);
  let error = $state('');
  // Inline errors by field: the server's 400 text, or a local parse error.
  let errors = $state<Partial<Record<Field, string>>>({});
  let estimate = $state('');
  // The estimate text being sent, so an Enter then a blur send it once.
  let sendingEstimate: string | null = null;
  let choosing = $state(false);
  let draft = $state('');
  let matches = $state<Obj[] | null>(null);

  const t = $derived(task?.task);

  onMount(async () => {
    try {
      const [loaded, p, a] = await Promise.all([
        getTask(id),
        listProjects({ status: 'any' }),
        listObjects({ type: 'area', limit: 1000 }).then((r) => r.objects),
      ]);
      [projects, areas] = [p, a];
      await show(loaded);
    } catch (e) {
      error = `error: ${message(e)}`;
    }
  });

  function message(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  /** Takes the server's task, then loads what depends on it (the project's sections, the parent title). */
  async function show(loaded: Task) {
    const before = task?.task;
    task = loaded;
    const f = loaded.task;
    estimate = f.estimate === null ? '' : formatEstimate(f.estimate);
    if (f.project_id !== before?.project_id) sections = f.project_id === null ? [] : await listSections(f.project_id);
    if (f.parent_id !== before?.parent_id) parent = f.parent_id === null ? null : await getObject(f.parent_id);
  }

  /** Runs `call` and shows its task; `false` when it failed and `field` shows the error. */
  async function run(field: Field, call: () => Promise<Task>): Promise<boolean> {
    errors = { ...errors, [field]: '' };
    try {
      await show(await call());
      return true;
    } catch (e) {
      errors = { ...errors, [field]: message(e) };
      return false;
    }
  }

  function patch(field: keyof FieldChanges, changes: FieldChanges) {
    return run(field, () => updateTask(id, { task: changes }));
  }

  function toggleDone() {
    next = null;
    return run('status', async () => {
      if (t?.status === 'done') return reopenTask(id);
      const res = await completeTask(id);
      next = res.next;
      return res.task;
    });
  }

  /** Emptying the date clears the time too (`joinDate` gives `null`). */
  function setDate(field: 'scheduled' | 'deadline', date: string, time: string) {
    return patch(field, { [field]: joinDate(date, time) });
  }

  async function saveEstimate() {
    const text = estimate.trim();
    const current = t?.estimate ?? null;
    if (text === sendingEstimate || text === (current === null ? '' : formatEstimate(current))) return;
    const minutes = text === '' ? null : parseEstimate(text);
    if (text !== '' && minutes === null) {
      errors = { ...errors, estimate: 'estimate must look like 90, 90m or 1h30m' };
      return;
    }
    sendingEstimate = text;
    await patch('estimate', { estimate: minutes });
    sendingEstimate = null;
  }

  function estimateKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      void saveEstimate();
    }
  }

  /** `''` → null, else the id. */
  const idOf = (value: string) => (value === '' ? null : Number(value));

  function openParent() {
    draft = '';
    matches = null;
    choosing = true;
  }

  async function setParent(pid: number | null) {
    await patch('parent_id', { parent_id: pid });
    if (!errors.parent_id) choosing = false;
  }

  async function submitParent(e: SubmitEvent) {
    e.preventDefault();
    const query = draft.trim();
    if (query === '') return;
    if (/^\d+$/.test(query)) return setParent(Number(query));
    try {
      matches = (await findByTitle(query, id)).filter((m) => m.type === 'task');
    } catch (err) {
      errors = { ...errors, parent_id: message(err) };
    }
  }

  function parentKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      choosing = false;
    }
  }
</script>

{#snippet err(field: Field)}
  {#if errors[field]}<span class="err" role="alert">{errors[field]}</span>{/if}
{/snippet}

{#snippet when(field: 'scheduled' | 'deadline')}
  {@const v = splitDate(t ? t[field] : null)}
  <input type="date" aria-label={field} value={v.date} onchange={(e) => setDate(field, e.currentTarget.value, v.time)} />
  {#if v.date}
    <input
      type="time"
      aria-label="{field} time"
      value={v.time}
      onchange={(e) => setDate(field, v.date, e.currentTarget.value)}
    />
    <button type="button" aria-label="clear {field}" onclick={() => patch(field, { [field]: null })}>[ clear ]</button>
  {/if}
  {@render err(field)}
{/snippet}

<section class="fields" aria-label="task">
  {#if error}
    <p class="err" role="alert">{error}</p>
  {:else if t}
    <div class="line">
      <button type="button" onclick={toggleDone}>[ {t.status === 'done' ? 'reopen' : 'complete'} ]</button>
      {#if t.status === 'done'}<span class="ok">done</span>{/if}
      {#if next}
        <a href={objectHref(next.id)}>next {(next.task.scheduled ?? next.task.deadline ?? '').slice(0, 10)}</a>
      {/if}
      {@render err('status')}
    </div>
    <dl>
      <dt>priority</dt>
      <dd>
        <select aria-label="priority" value={t.priority} onchange={(e) => patch('priority', { priority: Number(e.currentTarget.value) })}>
          {#each [1, 2, 3, 4] as p (p)}<option value={p}>P{p}</option>{/each}
        </select>
        {@render err('priority')}
      </dd>
      <dt>scheduled</dt>
      <dd>{@render when('scheduled')}</dd>
      <dt>deadline</dt>
      <dd>{@render when('deadline')}</dd>
      <dt>estimate</dt>
      <dd>
        <input
          class="text"
          aria-label="estimate"
          placeholder="90m, 1h30m"
          autocomplete="off"
          spellcheck="false"
          bind:value={estimate}
          onkeydown={estimateKey}
          onblur={saveEstimate}
        />
        {@render err('estimate')}
      </dd>
      <dt>someday</dt>
      <dd>
        <button type="button" aria-label="someday" aria-pressed={t.someday} onclick={() => patch('someday', { someday: !t.someday })}
          >{t.someday ? '[x]' : '[ ]'}</button
        >
        {@render err('someday')}
      </dd>
      <dt>project</dt>
      <dd>
        <select aria-label="project" value={t.project_id ?? ''} onchange={(e) => patch('project_id', { project_id: idOf(e.currentTarget.value) })}>
          <option value="">none</option>
          {#each projects as p (p.id)}<option value={p.id}>{p.title}</option>{/each}
        </select>
        {#if t.project_id !== null}
          <select
            aria-label="section"
            value={t.section_id ?? ''}
            onchange={(e) => patch('section_id', { section_id: idOf(e.currentTarget.value) })}
          >
            <option value="">no section</option>
            {#each sections as s (s.id)}<option value={s.id}>{s.title}</option>{/each}
          </select>
        {/if}
        {@render err('project_id')}
        {@render err('section_id')}
      </dd>
      <dt>area</dt>
      <dd>
        <select aria-label="area" value={t.area_id ?? ''} onchange={(e) => patch('area_id', { area_id: idOf(e.currentTarget.value) })}>
          <option value="">none</option>
          {#each areas as a (a.id)}<option value={a.id}>{a.title}</option>{/each}
        </select>
        {@render err('area_id')}
      </dd>
      <dt>parent</dt>
      <dd>
        {#if parent}<a href={objectHref(parent.id)}>{parent.title}</a>{/if}
        {#if choosing}
          <form class="prompt" onsubmit={submitParent}>
            <span class="sigil" aria-hidden="true">&gt;</span>
            <input
              class="text"
              bind:value={draft}
              onkeydown={parentKey}
              autocomplete="off"
              spellcheck="false"
              aria-label="parent id or title"
              placeholder="id or title"
              {@attach (node) => node.focus()}
            />
          </form>
          {#if matches}
            <ul aria-label="parent matches">
              {#each matches as m (m.id)}
                <li><button type="button" onkeydown={parentKey} onclick={() => setParent(m.id)}>{m.title}</button></li>
              {:else}
                <li class="dim">no matches</li>
              {/each}
            </ul>
          {/if}
        {:else}
          <button type="button" aria-label="{parent ? 'change' : 'set'} parent" onclick={openParent}>[ {parent ? 'change' : 'set'} ]</button>
          {#if parent}<button type="button" aria-label="clear parent" onclick={() => setParent(null)}>[ clear ]</button>{/if}
        {/if}
        {@render err('parent_id')}
      </dd>
      <dt>repeat</dt>
      <dd>
        <Recurrence value={t.recurrence} onsave={(recurrence) => patch('recurrence', { recurrence })} />
        {@render err('recurrence')}
      </dd>
    </dl>
  {:else}
    <p class="dim">loading …</p>
  {/if}
</section>

<style>
  .fields {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .line,
  dd,
  .prompt {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
    min-width: 0;
  }
  dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    align-items: center;
    gap: 0 1ch;
    margin: 0;
  }
  dt {
    color: var(--dim);
  }
  dd {
    margin: 0;
  }
  select,
  input,
  button {
    min-height: 44px;
    max-width: 100%;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
  }
  select,
  input[type='date'],
  input[type='time'] {
    min-width: 44px;
  }
  .text {
    flex: 1 1 12ch;
    min-width: 0;
    border-bottom: 1px solid var(--line);
    caret-shape: block;
  }
  .text:focus {
    outline: none;
    border-bottom-color: var(--accent);
  }
  button {
    min-width: 44px;
    padding: 0;
    color: var(--dim);
    text-align: left;
    cursor: pointer;
  }
  button:hover {
    color: var(--fg);
  }
  a {
    display: inline-flex;
    align-items: center;
    min-height: 44px;
    color: inherit;
    overflow-wrap: anywhere;
  }
  ul {
    flex-basis: 100%;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .sigil {
    color: var(--accent);
  }
  p {
    margin: 0;
  }
  .ok {
    color: var(--ok);
  }
  .dim {
    color: var(--dim);
  }
  .err {
    color: var(--err);
  }
</style>
