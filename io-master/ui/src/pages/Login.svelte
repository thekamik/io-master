<!-- The login. -->
<script lang="ts">
  import { logIn, type AppView } from "../lib/api/app";
import { sentence } from "../lib/api/errors";

  let { onchange }: { onchange: (app: AppView) => void } = $props();

  let name = $state("");
  let password = $state("");
  let problem = $state("");
  let busy = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      onchange(await logIn(name, password));
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
