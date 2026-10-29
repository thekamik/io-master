<!--
  The application: shows the first start, the login, or the user logged in,
  from the state of the program (login_get in
  src/panels/views/program_start/login.rs), which Rust keeps. It reads that state
  at the start and again whenever an element says it changed it (installed,
  logged in, logged out). The user logged in sees the frame: the panels in
  the side bar (the evaluation plan so far; the others come back one by
  one) and their name with "Log out".
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "./lib/api/errors";
  import { loginGet, loginLogOut, type ProgramView } from "./lib/api/program_start";
  import InstallationForm from "./views/program_start/InstallationForm.svelte";
  import EvaluationPlan from "./views/evaluation_plan/EvaluationPlan.svelte";
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
  <div class="layout">
    <nav class="sidebar">
      <div class="brand">
        <span class="mark">IO</span>
        <div>
          <div class="name">IO Master</div>
          <div class="tagline">evaluation with AI</div>
        </div>
      </div>

      <button type="button" class="nav-item active">Evaluation plan</button>

      <div class="user">
        <span class="who">{program.loggedIn}</span>
        <button type="button" onclick={logOut}>Log out</button>
      </div>
    </nav>

    <main>
      <EvaluationPlan />
    </main>
  </div>
{/if}
