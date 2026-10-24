<!--
  The element "recovery code form" of the view Users, on the page User (its
  command recovery_code_form_replace in src/views/users/users.rs): "Make a new
  recovery code", which needs the password (user, 2026-10-24); the new code is
  shown here once and the old one stops working (step 49 of docs/history.md).
-->
<script lang="ts">
  import RecoveryCode from "../../components/recovery_code/RecoveryCode.svelte";
  import type { RecoveryCodeView } from "../../lib/api/components/recovery_code";
  import { sentence } from "../../lib/api/errors";
  import { recoveryCodeFormReplace } from "../../lib/api/users";

  let password = $state("");
  let notice: { done: boolean; text: string } | null = $state(null);
  let code: RecoveryCodeView | null = $state(null);

  async function submit(event: SubmitEvent) {
    event.preventDefault();

    try {
      code = await recoveryCodeFormReplace(password);
      notice = { done: true, text: "A new recovery code was made. The old one no longer works." };
    } catch (error) {
      notice = { done: false, text: `No new code was made: ${sentence(error)}` };
    } finally {
      password = "";
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
      <button type="submit">Make a new recovery code</button>
    </form>
  {/if}
</div>
