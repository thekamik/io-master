<!--
  The tab Program of the view Settings: what this installation is and how
  it is protected, from the state of the program the frame has read
  (src/views/shell/program/). No command of its own.
-->
<script lang="ts">
  import type { ProgramView } from "../../lib/api/shell";

  let { app }: { app: ProgramView } = $props();
</script>

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
    Coming in later steps: the TPM as a further factor, the secret of the application in the keyring of your Linux
    account, the password reset through the master, and the removal of the installation.
  </p>
</div>
