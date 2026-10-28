<!--
  The login (its command login_log_in in src/views/program_start/login.rs):
  a name and a password. When the user is logged in, the form tells the
  application (ondone), which reads the state of the program again. The
  login with the recovery code comes later.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import { loginLogIn } from "../../lib/api/program_start";

  let { ondone }: { ondone: () => void } = $props();

  let name = $state("");
  let password = $state("");
  let problem = $state("");
  let busy = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      await loginLogIn(name, password);
      ondone();
    } catch (error) {
      problem = sentence(error);
    } finally {
      password = "";
      busy = false;
    }
  }
</script>

<main class="login">
  <div class="mark large">IO</div>
  <h1>IO Master</h1>
  <p class="note">Log in to open your workstation.</p>

  {#if problem}<p class="problem">{problem}</p>{/if}

  <form onsubmit={submit}>
    <label>Name <input bind:value={name} autocomplete="username" required /></label>
    <label>Password <input type="password" bind:value={password} autocomplete="current-password" required /></label>
    <button type="submit" disabled={busy}>{busy ? "Logging in…" : "Log in"}</button>
  </form>
</main>
