<script lang="ts">
  import { listObjects, TYPES, type Archived, type Filter, type Obj } from '../lib/objects';
  import CreateForm from './CreateForm.svelte';
  import TagTree from './TagTree.svelte';
  import { objectHref, ownsKeys, trashHref } from './route';

  const PAGE = 50;

  let type = $state('');
  let pinned = $state(false);
  let archived = $state<Archived>('false');
  let tag = $state('');

  let objects = $state<Obj[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let error = $state('');
  let sel = $state(0);
  let rowsEl: HTMLElement;
  // Bumped per request so a slow, stale response never overwrites a newer one.
  let seq = 0;

  const filter = $derived<Filter>({
    type,
    tag,
    pinned: pinned || undefined,
    archived,
    limit: PAGE,
  });

  $effect(() => {
    void load(filter, false);
  });

  async function load(f: Filter, append: boolean) {
    const mine = ++seq;
    loading = true;
    error = '';
    try {
      const page = await listObjects({ ...f, offset: append ? objects.length : 0 });
      if (mine !== seq) return;
      objects = append ? [...objects, ...page.objects] : page.objects;
      total = page.total;
      if (!append) sel = 0;
    } catch (e) {
      if (mine === seq) error = `error: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      if (mine === seq) loading = false;
    }
  }

  function created(obj: Obj) {
    objects = [obj, ...objects];
    total += 1;
    sel = 0;
  }

  function move(by: number) {
    sel = Math.min(Math.max(sel + by, 0), objects.length - 1);
    rowsEl.querySelectorAll('a')[sel]?.focus();
  }

  /** Tag-folder buttons pass list keys through; other controls keep them. */
  function keepsKeys(target: EventTarget | null): boolean {
    return ownsKeys(target) && !(target instanceof HTMLElement && target.matches('[data-tag-folder]'));
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey || keepsKeys(e.target) || objects.length === 0) return;
    if (e.key === 'j') move(1);
    else if (e.key === 'k') move(-1);
    else if (e.key === 'Enter') location.hash = objectHref(objects[sel].id);
    else return;
    e.preventDefault();
  }
</script>

<svelte:window {onkeydown} />

<div class="objects">
  <TagTree selected={tag} onselect={(t) => (tag = t)} />

  <div class="main">
    <CreateForm oncreated={created} />

    <div class="filters">
      <select bind:value={type} aria-label="type filter">
        <option value="">any type</option>
        {#each TYPES as t (t)}<option value={t}>{t}</option>{/each}
      </select>
      <label><input type="checkbox" bind:checked={pinned} /> pinned only</label>
      <label>
        archived
        <select bind:value={archived} aria-label="archived filter">
          <option value="false">false</option>
          <option value="true">true</option>
          <option value="any">any</option>
        </select>
      </label>
      <a class="trashlink" href={trashHref}>trash</a>
    </div>

    {#if error}<p class="err" role="alert">{error}</p>{/if}

    <ul class="rows" bind:this={rowsEl} aria-label="objects">
      {#each objects as o, i (o.id)}
        <li>
          <a
            href={objectHref(o.id)}
            class:sel={i === sel}
            aria-current={i === sel ? 'true' : undefined}
            onfocus={() => (sel = i)}
          >
            <span class="pin" aria-label={o.pinned ? 'pinned' : undefined}>{o.pinned ? '*' : ' '}</span>
            <span class="title">{o.title}</span>
            <span class="type">{o.type}</span>
            {#if o.tags.length > 0}<span class="tagl">{o.tags.map((t) => `#${t}`).join(' ')}</span>{/if}
          </a>
        </li>
      {/each}
    </ul>

    {#if !loading && !error && objects.length === 0}<p class="dim">no objects</p>{/if}
    {#if objects.length < total}
      <button type="button" class="more" disabled={loading} onclick={() => load(filter, true)}>[ more ]</button>
    {/if}
  </div>
</div>

<style>
  .objects {
    display: grid;
    grid-template-columns: minmax(8rem, 14rem) minmax(0, 1fr);
    gap: 0.75rem;
  }
  @media (max-width: 40rem) {
    .objects {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-width: 0;
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1rem;
    color: var(--dim);
  }
  .filters label {
    display: flex;
    align-items: center;
    gap: 0.5ch;
    min-height: 44px;
  }
  .trashlink {
    display: inline-flex;
    align-items: center;
    min-height: 44px;
    margin-left: auto;
    color: var(--dim);
  }
  .trashlink:hover {
    color: var(--fg);
  }
  select,
  .more {
    min-height: 44px;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    cursor: pointer;
  }
  .more:hover {
    color: var(--fg);
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .rows a {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
    min-height: 44px;
    padding: 0 0.5ch;
    color: inherit;
    text-decoration: none;
  }
  .rows a.sel {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .pin {
    color: var(--accent);
    white-space: pre;
  }
  .title {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .type,
  .tagl {
    color: var(--dim);
    overflow-wrap: anywhere;
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
