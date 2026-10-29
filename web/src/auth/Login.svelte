<script lang="ts">
  import { login } from '../lib/api';
  import AuthPanel from './AuthPanel.svelte';
  import PasswordPrompt from './PasswordPrompt.svelte';

  let { note = '', ondone }: { note?: string; ondone: () => void } = $props();

  let password = $state('');
  let busy = $state(false);
  let error = $state('');

  async function submit() {
    error = '';
    if (password === '') {
      error = 'password required';
      return;
    }
    busy = true;
    try {
      if (await login(password)) ondone();
      else error = 'wrong password';
    } catch (e) {
      error = e instanceof Error ? e.message : 'login failed';
    } finally {
      busy = false;
    }
  }
</script>

<AuthPanel title="polykov login" action="log in" {busy} {note} errors={error ? [error] : []} onsubmit={submit}>
  <PasswordPrompt id="login-password" label="password" bind:value={password} autofocus invalid={error !== ''} />
</AuthPanel>
