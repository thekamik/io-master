<!--
  The element "setup form" of the view Users (its command setup_form_install in src/views/users/users.rs):
  the first start: the one user of the installation. When it is
  done it tells the application (ondone), which reads the state of the
  program again.

  Unlike the web interface there is no one-time setup code: it
  protected a network port, and the window has none.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import { setupFormInstall } from "../../lib/api/users";

  let { ondone }: { ondone: () => void } = $props();

  let name = $state("");
  let password = $state("");
  let passwordRepeat = $state("");
  let problem = $state("");
  let busy = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      await setupFormInstall({ name, password, passwordRepeat });
      ondone();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }
</script>

<main class="login wide">
  <div class="mark large">IO</div>
  <h1>Install IO Master</h1>
  <p class="note">
    This is the first start. The person who installs IO Master is its only user.
  </p>

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
</main>
