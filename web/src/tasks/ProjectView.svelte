<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { ApiError } from '../lib/api';
  import { getObject, listObjects, type Obj } from '../lib/objects';
  import {
    PROJECT_STATUSES,
    createSection,
    deleteSection,
    getProject,
    listProjects,
    listSections,
    updateProject,
    updateSection,
    type Project,
    type ProjectChanges,
    type ProjectStatus,
    type Section,
  } from '../lib/projects';
  import { listTasks, localToday, type Row } from '../lib/tasks';
  import { projectHref } from './route';
  import TaskList, { TASKS_CHANGED } from './TaskList.svelte';
  import TaskRow from './TaskRow.svelte';

  // `kind: 'area'` shows the area's projects and `TaskList name=all filter={area}`. `id` is read once;
  // Shell re-creates the view per id.
  let { id, kind }: { id: number; kind: 'project' | 'area' } = $props();

  let project = $state<Project | null>(null);
  let area = $state<Obj | null>(null);
  let areas = $state<Obj[]>([]);
  let projects = $state<Project[]>([]);
  let sections = $state<Section[]>([]);
  let rows = $state<Row[]>([]);
  let loading = $state(true);
  let error = $state('');
  let today = $state(localToday());
  // The open prompt: a new section, or the rename of one.
  let adding = $state(false);
  let renaming = $state<number | null>(null);
  let draft = $state('');
  let confirming = $state<number | null>(null);
  let viewEl: HTMLElement;

  const groups = $derived([
    { section: null, rows: rows.filter((r) => !sections.some((s) => s.id === r.task.section_id)) },
    ...sections.map((s) => ({ section: s, rows: rows.filter((r) => r.task.section_id === s.id) })),
  ]);

  onMount(() => {
    void load();
    const reload = () => void loadTasks();
    if (kind === 'project') window.addEventListener(TASKS_CHANGED, reload);
    return () => window.removeEventListener(TASKS_CHANGED, reload);
  });

  function message(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  async function load() {
    try {
      if (kind === 'area') {
        const [obj, list] = await Promise.all([getObject(id), listProjects({ area: id })]);
        if (obj.type !== 'area') throw new ApiError(404, 'not an area');
        [area, projects] = [obj, list];
      } else {
        [project, sections, areas] = await Promise.all([
          getProject(id),
          listSections(id),
          listObjects({ type: 'area', limit: 1000 }).then((p) => p.objects),
        ]);
        await loadTasks();
      }
    } catch (e) {
      error = e instanceof ApiError && e.status === 404 ? `${kind} not found` : `error: ${message(e)}`;
    } finally {
      loading = false;
    }
  }

  async function loadTasks() {
    rows = (await listTasks('all', { project: id, limit: 1000 })).tasks;
    today = localToday();
  }

  /** Runs one change, shows its failure inline, and refocuses `focus` (a `data-focus` key) after. */
  async function act(run: () => Promise<void>, focus?: string) {
    error = '';
    try {
      await run();
    } catch (e) {
      error = `error: ${message(e)}`;
    }
    if (focus) {
      await tick();
      // The view may be gone by now (route change).
      viewEl?.querySelector<HTMLElement>(`[data-focus="${focus}"]`)?.focus();
    }
  }

  const save = (changes: ProjectChanges) =>
    act(async () => {
      project = await updateProject(id, changes);
    });

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const title = draft;
    const sid = renaming;
    void act(async () => {
      if (sid === null) {
        const s = await createSection(id, title);
        sections = [...sections, s];
      } else {
        await updateSection(sid, { title });
        sections = await listSections(id);
      }
      closePrompt();
    }, sid === null ? 'add' : `rename-${sid}`);
  }

  function openPrompt(sid: number | null, title = '') {
    adding = sid === null;
    renaming = sid;
    draft = title;
    confirming = null;
  }

  function closePrompt() {
    adding = false;
    renaming = null;
    draft = '';
  }

  /** Esc in the prompt cancels it (and is marked handled so the page stays). */
  function promptKey(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    const focus = renaming === null ? 'add' : `rename-${renaming}`;
    closePrompt();
    void act(async () => {}, focus);
  }

  /** Moves a section one place; focus stays on the same button unless it is now disabled. */
  function move(s: Section, by: number) {
    const to = s.position + by;
    const end = to === 0 || to === sections.length - 1;
    const button = by < 0 !== end ? 'up' : 'down';
    return act(async () => {
      await updateSection(s.id, { position: to });
      sections = await listSections(id);
    }, `${button}-${s.id}`);
  }

  const remove = (s: Section) =>
    act(async () => {
      await deleteSection(s.id);
      confirming = null;
      // The server renumbers the remaining positions.
      sections = await listSections(id);
      await loadTasks();
    }, 'add');

  function cancelDelete(s: Section) {
    confirming = null;
    void act(async () => {}, `delete-${s.id}`);
  }
</script>

{#snippet prompt(label: string)}
  <form class="prompt" onsubmit={submit}>
    <span class="sigil" aria-hidden="true">&gt;</span>
    <!-- svelte-ignore a11y_autofocus -->
    <input bind:value={draft} aria-label={label} autocomplete="off" autofocus onkeydown={promptKey} />
  </form>
{/snippet}

<section class="view" aria-label={kind} bind:this={viewEl}>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if loading}
    <p class="dim">loading …</p>
  {:else if kind === 'area' && area}
    <h1>{area.title}</h1>
    {#if projects.length > 0}
      <ul class="projects" aria-label="projects">
        {#each projects as p (p.id)}
          <li><a href={projectHref(p.id)}>{p.title}</a> <span class="dim">{p.project.open_tasks} open</span></li>
        {/each}
      </ul>
    {/if}
    <TaskList name="all" filter={{ area: id }} />
  {:else if project}
    <h1>{project.title}</h1>
    <div class="fields">
      <label>
        status
        <select value={project.project.status} onchange={(e) => save({ status: e.currentTarget.value as ProjectStatus })}>
          {#each PROJECT_STATUSES as s (s)}<option value={s}>{s}</option>{/each}
        </select>
      </label>
      <label>
        area
        <select
          value={project.project.area_id ?? ''}
          onchange={(e) => save({ area_id: e.currentTarget.value === '' ? null : Number(e.currentTarget.value) })}
        >
          <option value="">none</option>
          {#each areas as a (a.id)}<option value={a.id}>{a.title}</option>{/each}
        </select>
      </label>
      {#if project.project.start}<span class="dim">start {project.project.start}</span>{/if}
      {#if project.project.deadline}<span class="dim">deadline {project.project.deadline}</span>{/if}
    </div>

    {#each groups as g (g.section?.id ?? 0)}
      {@const s = g.section}
      {#if s}
        <div class="section">
          {#if renaming === s.id}
            {@render prompt(`rename ${s.title}`)}
          {:else}
            <h2>{s.title}</h2>
            {#if confirming === s.id}
              <span class="ask">delete {s.title}?</span>
              <button type="button" onclick={() => remove(s)}>[ yes ]</button>
              <button type="button" onclick={() => cancelDelete(s)} {@attach (node) => node.focus()}>[ no ]</button>
            {:else}
              <button type="button" data-focus="rename-{s.id}" onclick={() => openPrompt(s.id, s.title)}>[ rename ]</button>
              <button type="button" data-focus="up-{s.id}" disabled={s.position === 0} onclick={() => move(s, -1)}>[ up ]</button>
              <button
                type="button"
                data-focus="down-{s.id}"
                disabled={s.position === sections.length - 1}
                onclick={() => move(s, 1)}>[ down ]</button
              >
              <button type="button" data-focus="delete-{s.id}" onclick={() => (confirming = s.id)}>[ delete ]</button>
            {/if}
          {/if}
        </div>
      {/if}
      <ul class="rows" aria-label={s ? s.title : 'no section'}>
        {#each g.rows as row (row.id)}<TaskRow {row} {today} />{/each}
      </ul>
    {/each}

    {#if adding}
      {@render prompt('section title')}
    {:else}
      <button type="button" class="add" data-focus="add" onclick={() => openPrompt(null)}>[ add section ]</button>
    {/if}
  {/if}
</section>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }
  h1,
  h2 {
    margin: 0;
    font-size: inherit;
    font-weight: normal;
    overflow-wrap: anywhere;
  }
  h1 {
    color: var(--accent);
  }
  h2 {
    flex: 1 1 12ch;
    min-width: 0;
  }
  .fields,
  .section {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
  }
  .section {
    margin-top: 0.5rem;
    border-bottom: 1px solid var(--line);
  }
  label {
    display: flex;
    align-items: center;
    gap: 0.5ch;
    min-height: 44px;
    color: var(--dim);
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
  button {
    min-width: 44px;
    padding: 0 0.25ch;
  }
  button:hover:not(:disabled) {
    color: var(--fg);
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .add {
    align-self: flex-start;
  }
  .ask {
    color: var(--err);
  }
  .rows,
  .projects {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .projects a {
    display: inline-flex;
    align-items: center;
    min-height: 44px;
    color: inherit;
    overflow-wrap: anywhere;
  }
  .prompt {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 44px;
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
    caret-shape: block;
  }
  input:focus {
    outline: none;
    border-bottom-color: var(--accent);
  }
  .dim {
    margin: 0;
    color: var(--dim);
  }
  .err {
    margin: 0;
    color: var(--err);
  }
</style>
