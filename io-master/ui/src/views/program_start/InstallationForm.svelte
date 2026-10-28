<!--
  The first start (its command installation_create in
  src/views/program_start/installation.rs): the one user of the
  installation, a name and a password given twice. The recovery code comes
  with it and is shown at once; when the user has confirmed it, the form
  tells the application (ondone), which reads the state of the program
  again.
-->
<script lang="ts">
  import RecoveryCode from "../../components/recovery_code/RecoveryCode.svelte";
  import { sentence } from "../../lib/api/errors";
  import { installationCreate, type RecoveryCodeView } from "../../lib/api/program_start";

  let { ondone }: { ondone: () => void } = $props();

  let name = $state("");
  let password = $state("");
  let passwordRepeated = $state("");
  let problem = $state("");
  let busy = $state(false);
  let code: RecoveryCodeView | null = $state(null);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      code = await installationCreate({ name, password, passwordRepeated });
    } catch (error) {
      problem = sentence(error);
    } finally {
      password = passwordRepeated = "";
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
        <input type="password" bind:value={passwordRepeated} autocomplete="new-password" minlength="12" required />
      </label>

      <button type="submit" disabled={busy}>{busy ? "Installing…" : "Install"}</button>
    </form>
  {/if}
</main>
