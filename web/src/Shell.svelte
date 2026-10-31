<script lang="ts">
  import { onMount } from 'svelte';
  import { logout, serverVersion } from './lib/api';
  import CaptureBar from './capture/CaptureBar.svelte';
  import Detail from './objects/Detail.svelte';
  import List from './objects/List.svelte';
  import Trash from './objects/Trash.svelte';
  import { listHref, parseHash } from './objects/route';
  import Palette from './palette/Palette.svelte';
  import Nav from './tasks/Nav.svelte';
  import ProjectView from './tasks/ProjectView.svelte';
  import { parseTaskHash } from './tasks/route';
  import TaskList from './tasks/TaskList.svelte';

  // `focusCommand`: focus the command input on mount (after a login).
  let { onlogout, focusCommand = false }: { onlogout: () => void; focusCommand?: boolean } = $props();

  let route = $state(parseHash(location.hash));
  // Task routes (`#/today`, `#/p/<id>`, ...) are checked before the object routes.
  let taskRoute = $state(parseTaskHash(location.hash));
  // One-line note above the list (e.g. after a delete); cleared once the route leaves the list.
  let note = $state('');

  function onhashchange() {
    route = parseHash(location.hash);
    taskRoute = parseTaskHash(location.hash);
    if (route.view !== 'list') note = '';
    if (taskRoute) note = '';
  }

  function deleted() {
    note = 'moved to trash';
    location.hash = listHref;
  }

  let status = $state('server …');
  let ok = $state<boolean | null>(null);
  let command = $state('');

  let logoutError = $state('');

  async function doLogout() {
    logoutError = '';
    try {
      await logout();
      onlogout();
    } catch (e) {
      logoutError = e instanceof Error ? `logout failed: ${e.message}` : 'logout failed';
    }
  }

  /** The skip link: focus the command input. Not a hash change (the hash is the route), and an
   * Enter stops here so the list keys on `window` don't also open the selected row. */
  function skipToCommand(e: Event) {
    e.preventDefault();
    e.stopPropagation();
    document.getElementById('command')?.focus();
  }

  onMount(async () => {
    const version = await serverVersion();
    ok = version !== null;
    status = ok ? `server ok ${version}` : 'server unreachable';
  });
</script>

<svelte:window {onhashchange} />

<main class="shell">
  <a class="skip" href="#command" onclick={skipToCommand} onkeydown={(e) => e.key === 'Enter' && skipToCommand(e)}
    >skip to command line</a
  >
  <header class="bar">
    <span>polykov</span>
    <span class="status" class:ok={ok === true} class:err={ok === false} role="status">{status}</span>
    <button type="button" class="logout" onclick={doLogout}>logout</button>
  </header>
  {#if logoutError}<p class="err" role="alert">{logoutError}</p>{/if}
  <Nav />
  <CaptureBar />
  <Palette />

  <section class="body">
    {#if taskRoute?.view === 'list'}
      <TaskList name={taskRoute.list} />
    {:else if taskRoute}
      {#key `${taskRoute.view}/${taskRoute.id}`}<ProjectView id={taskRoute.id} kind={taskRoute.view} />{/key}
    {:else}
    {#if route.view === 'detail'}
      {#key route.id}<Detail id={route.id} ondeleted={deleted} />{/key}
    {:else if route.view === 'trash'}
      <Trash />
    {:else}
      {#if note}<p class="note" role="status">{note}</p>{/if}
      <List />
    {/if}
    {/if}
  </section>

  <form class="prompt" onsubmit={(e) => e.preventDefault()}>
    <span class="sigil" aria-hidden="true">&gt;</span>
    <div class="field">
      <input
        id="command"
        bind:value={command}
        autocomplete="off"
        spellcheck="false"
        aria-label="command"
        {@attach (node) => {
          if (focusCommand) node.focus();
        }}
      />
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
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0 0.25rem 0 0.75rem;
  }
  .status {
    flex: 1;
    text-align: right;
    color: var(--dim);
    overflow-wrap: anywhere;
  }
  .status.ok {
    color: var(--ok);
  }
  .status.err {
    color: var(--err);
  }
  .logout {
    min-height: 44px;
    padding: 0 0.5rem;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    cursor: pointer;
  }
  .logout:hover {
    color: var(--fg);
  }
  p.err {
    margin: 0;
    color: var(--err);
  }
  p.note {
    margin: 0 0 0.5rem;
    color: var(--ok);
  }
  .body {
    flex: 1;
    padding: 0.75rem;
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
  /* Kept above the viewport (not clipped to 1 px) so it is still a 44 px target; shown on focus. */
  .skip {
    position: absolute;
    top: -100px;
    left: 0.5rem;
    z-index: 10;
    display: inline-flex;
    align-items: center;
    min-height: 44px;
    padding: 0 0.75rem;
    border: 1px solid var(--accent);
    background: var(--bg);
    color: var(--accent);
  }
  .skip:focus {
    top: 0.5rem;
  }
</style>
