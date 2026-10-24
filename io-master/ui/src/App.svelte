<!--
  The application: shows the setup, the login or the frame of the
  window, from the state of the program (the element "program" of the
  view Shell, src/views/shell/program/), which Rust keeps. It reads that
  state at the start and again whenever an element says it changed it
  (installed, logged in, logged out).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "./lib/api/errors";
  import * as shell from "./lib/api/shell";
  import Shell from "./views/shell/Shell.svelte";
  import LoginForm from "./views/users/LoginForm.svelte";
  import SetupForm from "./views/users/SetupForm.svelte";

  /** The state of the program; null until it is read. */
  let app: shell.ProgramView | null = $state(null);
  let problem = $state("");

  async function reload() {
    try {
      app = await shell.programGet();
    } catch (error) {
      problem = sentence(error);
    }
  }

  onMount(reload);
</script>

{#if problem}
  <p class="problem centered">{problem}</p>
{:else if app === null}
  <p class="note centered">Starting…</p>
{:else if !app.installed}
  <SetupForm ondone={reload} />
{:else if app.user === null}
  <LoginForm ondone={reload} />
{:else}
  <Shell {app} onloggedout={reload} />
{/if}
