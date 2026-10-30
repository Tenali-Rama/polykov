<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { listObjects, type Obj } from '../lib/objects';
  import { listProjects, type Project } from '../lib/projects';
  import { LISTS } from '../lib/tasks';
  import { listHref, trashHref } from '../objects/route';
  import { areaHref, projectHref, taskListHref } from './route';

  let projects = $state<Project[]>([]);
  let areas = $state<Obj[]>([]);
  let error = $state('');
  let hash = $state(current());
  // Phone only: the menu behind `[ lists ]` is open. Wider screens always show the links.
  let open = $state(false);
  let toggleEl: HTMLButtonElement;
  let menuEl: HTMLElement;

  const fixed = [
    ...LISTS.map((l) => ({ href: taskListHref(l), label: l })),
    { href: listHref, label: 'objects' },
    { href: trashHref, label: 'trash' },
  ];
  const links = $derived([
    ...fixed,
    ...projects.map((p) => ({ href: projectHref(p.id), label: p.title })),
    ...areas.map((a) => ({ href: areaHref(a.id), label: a.title })),
  ]);

  function current(): string {
    return location.hash === '' ? listHref : location.hash;
  }

  onMount(async () => {
    try {
      [projects, areas] = await Promise.all([
        listProjects(),
        listObjects({ type: 'area', limit: 1000 }).then((p) => p.objects),
      ]);
    } catch (e) {
      error = `nav: ${e instanceof Error ? e.message : String(e)}`;
    }
  });

  function onhashchange() {
    hash = current();
    open = false;
  }

  async function toggle() {
    open = !open;
    if (open) {
      await tick();
      menuEl.querySelector('a')?.focus();
    }
  }

  function close() {
    open = false;
    toggleEl.focus();
  }

  /** In the open menu: Esc closes it, arrows move between its links. */
  function onkeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === 'Escape') close();
    else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      const all = [...menuEl.querySelectorAll('a')];
      const at = all.indexOf(document.activeElement as HTMLAnchorElement);
      const to = e.key === 'ArrowDown' ? Math.min(at + 1, all.length - 1) : Math.max(at - 1, 0);
      all[to]?.focus();
    } else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<svelte:window {onhashchange} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<nav class="nav" aria-label="lists" {onkeydown}>
  <button
    type="button"
    class="toggle"
    aria-label="lists"
    aria-expanded={open}
    aria-controls="nav-menu"
    bind:this={toggleEl}
    onclick={toggle}>[ lists ]</button
  >
  <ul id="nav-menu" class="menu" class:open bind:this={menuEl}>
    {#each links as l, i (l.href)}
      <li class:gap={i === LISTS.length || i === fixed.length}>
        <a href={l.href} aria-current={l.href === hash ? 'page' : undefined}>{l.label}</a>
      </li>
    {/each}
  </ul>
  {#if error}<p class="err">{error}</p>{/if}
</nav>

<style>
  .nav {
    border: 1px solid var(--line);
    padding: 0 0.25rem;
  }
  .toggle {
    display: none;
    min-width: 44px;
    min-height: 44px;
    border: 0;
    background: transparent;
    color: var(--fg);
    font: inherit;
    cursor: pointer;
  }
  .menu {
    display: flex;
    flex-wrap: wrap;
    list-style: none;
    margin: 0;
    padding: 0;
    gap: 0 1.5ch;
  }
  .gap {
    margin-left: 1.5ch;
  }
  a {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 44px;
    min-height: 44px;
    padding: 0 0.5ch;
    color: var(--dim);
    text-decoration: none;
    overflow-wrap: anywhere;
  }
  a:hover {
    color: var(--fg);
  }
  a[aria-current='page'] {
    color: var(--accent);
    text-decoration: underline;
  }
  .err {
    margin: 0;
    color: var(--err);
  }
  @media (max-width: 599.98px) {
    .toggle {
      display: inline-block;
    }
    .menu {
      display: none;
      flex-direction: column;
    }
    .menu.open {
      display: flex;
    }
    .gap {
      margin: 0.5rem 0 0;
      border-top: 1px solid var(--line);
    }
    a {
      display: flex;
      justify-content: flex-start;
    }
  }
</style>
