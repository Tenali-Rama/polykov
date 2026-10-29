<script lang="ts">
  import { onMount } from 'svelte';
  import { authState, type AuthState } from './lib/api';
  import Login from './auth/Login.svelte';
  import Setup from './auth/Setup.svelte';
  import Shell from './Shell.svelte';

  // Routes by auth state: no owner → setup, no session → login, else the shell.
  let view = $state<AuthState | 'loading' | 'error'>('loading');
  let error = $state('');
  let loginNote = $state('');

  async function route() {
    view = 'loading';
    try {
      view = await authState();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      view = 'error';
    }
  }

  function setupDone() {
    loginNote = 'owner created — log in';
    view = 'login';
  }

  function showLogin() {
    loginNote = '';
    view = 'login';
  }

  // A fresh login hands focus to the command input; a reload with a session keeps the default.
  let loggedIn = $state(false);

  function loginDone() {
    loggedIn = true;
    view = 'shell';
  }

  onMount(route);
</script>

{#if view === 'setup'}
  <Setup ondone={setupDone} onlogin={showLogin} />
{:else if view === 'login'}
  <Login note={loginNote} ondone={loginDone} />
{:else if view === 'shell'}
  <Shell onlogout={showLogin} focusCommand={loggedIn} />
{:else}
  <main class="screen" aria-busy={view === 'loading'}>
    {#if view === 'loading'}
      <p class="dim">connecting…</p>
    {:else}
      <p class="err" role="alert">server unreachable: {error}</p>
      <button type="button" onclick={route}>[ retry ]</button>
    {/if}
  </main>
{/if}

<style>
  .screen {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.75rem;
    max-width: 40rem;
    margin: 10vh auto 0;
    padding: 0.5rem;
  }
  p {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .dim {
    color: var(--dim);
  }
  .err {
    color: var(--err);
  }
  button {
    min-height: 44px;
    padding: 0 0.75rem;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
  }
</style>
