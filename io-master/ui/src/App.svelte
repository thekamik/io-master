<!--
  The application: shows the setup, the login or the pages,
  depending on the state of the program, which Rust keeps
  (src/tauri_ui/state.rs). Every page reports a changed state
  (installed, logged in, logged out) through `onchange`.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { appState, sentence, type AppView } from "./lib/api";
  import Login from "./pages/Login.svelte";
  import Setup from "./pages/Setup.svelte";
  import Shell from "./pages/Shell.svelte";

  /** The state of the program; null until it is read. */
  let app: AppView | null = $state(null);
  let problem = $state("");

  onMount(async () => {
    try {
      app = await appState();
    } catch (error) {
      problem = sentence(error);
    }
  });

  function changed(next: AppView) {
    app = next;
  }
</script>

{#if problem}
  <p class="problem centered">{problem}</p>
{:else if app === null}
  <p class="note centered">Starting…</p>
{:else if !app.installed}
  <Setup onchange={changed} />
{:else if app.user === null}
  <Login onchange={changed} />
{:else}
  <Shell {app} onchange={changed} />
{/if}
