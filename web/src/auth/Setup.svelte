<script lang="ts">
  import { setupOwner } from '../lib/api';
  import AuthPanel from './AuthPanel.svelte';
  import PasswordPrompt from './PasswordPrompt.svelte';

  // Same rule as the server: at least 12 Unicode characters.
  const MIN_PASSWORD_CHARS = 12;

  let { ondone }: { ondone: () => void } = $props();

  let password = $state('');
  let confirm = $state('');
  let submitted = $state(false);
  let busy = $state(false);
  let serverError = $state('');

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
      serverError = e instanceof Error ? e.message : 'setup failed';
    } finally {
      busy = false;
    }
  }
</script>

<AuthPanel title="polykov setup — set the owner password" action="create owner" {busy} {errors} onsubmit={submit}>
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
</AuthPanel>
