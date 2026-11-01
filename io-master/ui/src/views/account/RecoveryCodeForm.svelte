<!--
  The form "recovery code" of the panel Account (its command
  account_new_recovery_code in src/panels/views/account/credentials.rs): a
  new recovery code, made with the password, when the old one is lost or
  may have been seen. The new code is shown here once; the old one stops
  working.
-->
<script lang="ts">
  import RecoveryCode from "../../components/recovery_code/RecoveryCode.svelte";
  import { accountNewRecoveryCode } from "../../lib/api/account";
  import { sentence } from "../../lib/api/errors";
  import type { RecoveryCodeView } from "../../lib/api/program_start";

  let password = $state("");
  let notice: { done: boolean; text: string } | null = $state(null);
  let busy = $state(false);
  let code: RecoveryCodeView | null = $state(null);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    notice = null;
    busy = true;

    try {
      code = await accountNewRecoveryCode(password);
      notice = { done: true, text: "A new recovery code was made. The old one no longer works." };
    } catch (error) {
      notice = { done: false, text: `No new code was made. ${sentence(error)}` };
    } finally {
      password = "";
      busy = false;
    }
  }
</script>

<div class="panel">
  <h3>Your recovery code</h3>
  {#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}
  {#if code}
    <RecoveryCode view={code} onconfirmed={() => (code = null)} heading={false} />
  {:else}
    <form onsubmit={submit}>
      <p class="note">
        The recovery code sets a new password if you forget yours. Make a new one if the old one is lost or someone may
        have seen it; the old one then stops working.
      </p>
      <label>Password <input type="password" bind:value={password} autocomplete="current-password" required /></label>
      <button type="submit" disabled={busy}>{busy ? "Making it…" : "Make a new recovery code"}</button>
    </form>
  {/if}
</div>
