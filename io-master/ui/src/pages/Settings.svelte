<!--
  Settings, in tabs:

  - User: the password of the evaluator (later the factors of the
    person, step 9 of docs/todo.md);
  - AI: the connections to AI services (AiConfig.svelte);
  - Program: what this installation is and how it is protected
    (later the TPM factor, the secret of the application and the
    removal of the installation, steps 8 and 13).

  Adding users is not here: one Linux account holds one installation
  with one user (docs/architecture.md, section 3).
-->
<script lang="ts">
  import { changePassword, type AppView } from "../lib/api/app";
import { sentence } from "../lib/api/errors";
  import AiConfig from "../components/AiConfig.svelte";

  let { app }: { app: AppView } = $props();

  /** A message after a form was sent. */
  type Notice = { done: boolean; text: string } | null;

  type Tab = "user" | "ai" | "program";

  const tabs: { id: Tab; title: string }[] = [
    { id: "user", title: "User" },
    { id: "ai", title: "AI" },
    { id: "program", title: "Program" },
  ];

  let tab: Tab = $state("user");

  let current = $state("");
  let password = $state("");
  let passwordRepeat = $state("");
  let passwordNotice: Notice = $state(null);

  async function submitPassword(event: SubmitEvent) {
    event.preventDefault();

    try {
      await changePassword(current, password, passwordRepeat);
      passwordNotice = { done: true, text: "Your password was changed." };
    } catch (error) {
      passwordNotice = { done: false, text: `The password was not changed: ${sentence(error)}` };
    } finally {
      current = password = passwordRepeat = "";
    }
  }
</script>

<div class="tabs">
  {#each tabs as item (item.id)}
    <button type="button" class:active={tab === item.id} onclick={() => (tab = item.id)}>{item.title}</button>
  {/each}
</div>

{#if tab === "user"}
  <form class="panel" onsubmit={submitPassword}>
    <h3>Your password</h3>
    <p class="note">
      Signed in as <strong>{app.user}</strong>. The password opens your encrypted database; at least 12 characters.
    </p>
    {#if passwordNotice}<p class={passwordNotice.done ? "done" : "problem"}>{passwordNotice.text}</p>{/if}
    <label>
      Current password <input type="password" bind:value={current} autocomplete="current-password" required />
    </label>
    <label>
      New password
      <input type="password" bind:value={password} autocomplete="new-password" minlength="12" required />
    </label>
    <label>
      New password again
      <input type="password" bind:value={passwordRepeat} autocomplete="new-password" minlength="12" required />
    </label>
    <button type="submit">Change password</button>
  </form>
{:else if tab === "ai"}
  <AiConfig />
{:else}
  <div class="card">
    <h3>This installation</h3>
    <dl class="facts">
      <dt>Version</dt>
      <dd>IO Master {app.version}</dd>
      <dt>Role</dt>
      <dd>{app.role === "master" ? "Master" : "Workstation"}</dd>
      <dt>User</dt>
      <dd>{app.user}</dd>
      <dt>Folder of the data</dt>
      <dd class="mono">{app.data_dir}</dd>
    </dl>
  </div>

  <div class="card">
    <h3>Protection</h3>
    {#if app.warnings.length === 0}
      <p><span class="badge success">Protected</span></p>
    {:else}
      <ul class="plain">
        {#each app.warnings as warning (warning.text)}
          <li><span class="badge {warning.level}">{warning.level === "danger" ? "Risk" : "Note"}</span> {warning.text}</li>
        {/each}
      </ul>
    {/if}
    <p class="note">
      Coming in later steps: the TPM as a further factor, the secret of the application in the keyring of your
      Linux account, the password reset through the master, and the removal of the installation.
    </p>
  </div>
{/if}
