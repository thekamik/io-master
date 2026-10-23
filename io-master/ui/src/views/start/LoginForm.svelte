<!--
  The element "login form" of the view Start (src/views/start/login_form/).
  When the evaluator is logged in it tells the application (ondone), which
  reads the state of the program again.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import { logIn } from "../../lib/api/start/login_form";

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
      await logIn(name, password);
      ondone();
    } catch (error) {
      problem = sentence(error);
      password = "";
    } finally {
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
