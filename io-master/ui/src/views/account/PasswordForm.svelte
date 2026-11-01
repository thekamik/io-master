<!--
  The form "password" of the panel Account (its command
  account_change_password in src/panels/views/account/credentials.rs): the
  present password and the new one given twice. The passwords are cleared
  from the page as soon as they were sent. The change makes a new recovery
  code, shown here once; the old one stops working.
-->
<script lang="ts">
  import RecoveryCode from "../../components/recovery_code/RecoveryCode.svelte";
  import { accountChangePassword } from "../../lib/api/account";
  import { sentence } from "../../lib/api/errors";
  import type { RecoveryCodeView } from "../../lib/api/program_start";

  let current = $state("");
  let password = $state("");
  let passwordRepeated = $state("");
  let notice: { done: boolean; text: string } | null = $state(null);
  let busy = $state(false);
  let code: RecoveryCodeView | null = $state(null);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    notice = null;
    busy = true;

    try {
      code = await accountChangePassword({ current, password, passwordRepeated });
      notice = { done: true, text: "Your password was changed. Your old recovery code no longer works." };
    } catch (error) {
      notice = { done: false, text: `The password was not changed. ${sentence(error)}` };
    } finally {
      current = password = passwordRepeated = "";
      busy = false;
    }
  }
</script>

<div class="panel">
  <h3>Your password</h3>
  {#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}
  {#if code}
    <RecoveryCode view={code} onconfirmed={() => (code = null)} />
  {:else}
    <form onsubmit={submit}>
      <p class="note">
        The password opens your encrypted database; at least 12 characters. A new recovery code is made with it.
      </p>
      <label>
        Present password <input type="password" bind:value={current} autocomplete="current-password" required />
      </label>
      <label>
        New password
        <input type="password" bind:value={password} autocomplete="new-password" minlength="12" required />
      </label>
      <label>
        New password again
        <input type="password" bind:value={passwordRepeated} autocomplete="new-password" minlength="12" required />
      </label>
      <button type="submit" disabled={busy}>{busy ? "Changing it…" : "Change the password"}</button>
    </form>
  {/if}
</div>
