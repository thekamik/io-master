<!--
  The element "setup form" of the view Users (its command setup_form_install in src/views/users/users.rs):
  the first start: the one user of the installation. The first recovery code
  comes with the setup and is shown at once, as BitLocker shows its key (step
  49 of docs/history.md); when the evaluator has confirmed it, the form tells the
  application (ondone), which reads the state of the program again.

  Unlike the web interface there is no one-time setup code: it
  protected a network port, and the window has none.
-->
<script lang="ts">
  import RecoveryCode from "../../components/recovery_code/RecoveryCode.svelte";
  import type { RecoveryCodeView } from "../../lib/api/components/recovery_code";
  import { sentence } from "../../lib/api/errors";
  import { setupFormInstall } from "../../lib/api/users";

  let { ondone }: { ondone: () => void } = $props();

  let name = $state("");
  let password = $state("");
  let passwordRepeat = $state("");
  let problem = $state("");
  let busy = $state(false);
  let code: RecoveryCodeView | null = $state(null);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      code = await setupFormInstall({ name, password, passwordRepeat });
    } catch (error) {
      problem = sentence(error);
    } finally {
      password = passwordRepeat = "";
      busy = false;
    }
  }
</script>

<main class="login wide">
  <div class="mark large">IO</div>
  {#if code}
    <RecoveryCode view={code} onconfirmed={ondone} />
  {:else}
    <h1>Install IO Master</h1>
    <p class="note">This is the first start. The person who installs IO Master is its only user.</p>

    {#if problem}<p class="problem">{problem}</p>{/if}

    <form onsubmit={submit}>
      <label>Name <input bind:value={name} autocomplete="off" required /></label>
      <label>
        Password
        <input type="password" bind:value={password} autocomplete="new-password" minlength="12" required />
      </label>
      <label>
        Password again
        <input type="password" bind:value={passwordRepeat} autocomplete="new-password" minlength="12" required />
      </label>

      <button type="submit" disabled={busy}>{busy ? "Installing…" : "Install"}</button>
    </form>
  {/if}
</main>
