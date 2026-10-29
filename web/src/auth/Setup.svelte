<script lang="ts">
  import { ApiError, setupOwner } from '../lib/api';
  import AuthPanel from './AuthPanel.svelte';
  import PasswordPrompt from './PasswordPrompt.svelte';

  // Same rule as the server: at least 12 Unicode characters.
  const MIN_PASSWORD_CHARS = 12;

  // `onlogin` opens the login screen after the server says an owner already exists (409).
  let { ondone, onlogin }: { ondone: () => void; onlogin: () => void } = $props();

  let password = $state('');
  let confirm = $state('');
  let submitted = $state(false);
  let busy = $state(false);
  let serverError = $state('');
  let ownerExists = $state(false);

  const tooShort = $derived(
    (submitted || password !== '') && [...password].length < MIN_PASSWORD_CHARS,
  );
  const mismatch = $derived((submitted || confirm !== '') && confirm !== password);
  const errors = $derived([
    ...(tooShort ? [`password must be at least ${MIN_PASSWORD_CHARS} characters`] : []),
    ...(mismatch ? ['passwords do not match'] : []),
    ...(serverError ? [serverError] : []),
  ]);

  async function submit() {
    submitted = true;
    serverError = '';
    if (tooShort || mismatch) return;
    busy = true;
    try {
      await setupOwner(password);
      ondone();
    } catch (e) {
      ownerExists = e instanceof ApiError && e.status === 409;
      serverError = e instanceof Error ? e.message : 'setup failed';
    } finally {
      busy = false;
    }
  }
</script>

<!-- Once the owner exists the fields go and the action becomes `[ log in ]`. -->
<AuthPanel
  title="polykov setup — set the owner password"
  action={ownerExists ? 'log in' : 'create owner'}
  {busy}
  {errors}
  onsubmit={ownerExists ? onlogin : submit}
>
  {#if !ownerExists}
    <PasswordPrompt
      id="setup-password"
      label="password"
      bind:value={password}
      autocomplete="new-password"
      autofocus
      invalid={tooShort}
    />
    <PasswordPrompt
      id="setup-confirm"
      label="confirm"
      bind:value={confirm}
      autocomplete="new-password"
      invalid={mismatch}
    />
  {/if}
</AuthPanel>
