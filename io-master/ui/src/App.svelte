<!--
  The application: shows the first start, the login, or the user logged in,
  from the state of the program (login_get in
  src/views/program_start/login.rs), which Rust keeps. It reads that state
  at the start and again whenever an element says it changed it (installed,
  logged in, logged out). The views of the work come back one by one
  (2026-10-28); until then the logged-in user sees their name.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "./lib/api/errors";
  import { loginGet, loginLogOut, type ProgramView } from "./lib/api/program_start";
  import InstallationForm from "./views/program_start/InstallationForm.svelte";
  import LoginForm from "./views/program_start/LoginForm.svelte";

  /** The state of the program; null until it is read. */
  let program: ProgramView | null = $state(null);
  let problem = $state("");

  async function reload() {
    problem = "";

    try {
      program = await loginGet();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function logOut() {
    try {
      await loginLogOut();
    } catch (error) {
      problem = sentence(error);
    }

    await reload();
  }

  onMount(reload);
</script>

{#if problem}
  <p class="problem centered">{problem}</p>
{:else if program === null}
  <p class="note centered">Starting…</p>
{:else if !program.installed}
  <InstallationForm ondone={reload} />
{:else if program.loggedIn === null}
  <LoginForm ondone={reload} />
{:else}
  <main class="login">
    <div class="mark large">IO</div>
    <h1>{program.loggedIn}</h1>
    <p class="note">You are logged in. IO Master is being rebuilt on its new data model; the views come back one by one.</p>
    <button type="button" onclick={logOut}>Log out</button>
  </main>
{/if}
