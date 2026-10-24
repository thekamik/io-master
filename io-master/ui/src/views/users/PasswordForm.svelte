<!--
  The element "password form" of the view Users, on the page User (its
  command password_form_change in src/views/users/users.rs). The passwords are
  cleared from the page as soon as they were sent. A change of the password
  makes a new recovery code, shown here once; the old one stops working (step
  49 of docs/history.md).
-->
<script lang="ts">
  import RecoveryCode from "../../components/recovery_code/RecoveryCode.svelte";
  import type { RecoveryCodeView } from "../../lib/api/components/recovery_code";
  import { sentence } from "../../lib/api/errors";
  import { passwordFormChange } from "../../lib/api/users";

  let { user }: { user: string | null } = $props();

  let current = $state("");
  let password = $state("");
  let passwordRepeat = $state("");
  let notice: { done: boolean; text: string } | null = $state(null);
  let code: RecoveryCodeView | null = $state(null);

  async function submit(event: SubmitEvent) {
    event.preventDefault();

    try {
      code = await passwordFormChange(current, password, passwordRepeat);
      notice = { done: true, text: "Your password was changed. Your old recovery code no longer works." };
    } catch (error) {
      notice = { done: false, text: `The password was not changed: ${sentence(error)}` };
    } finally {
      current = password = passwordRepeat = "";
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
        Signed in as <strong>{user}</strong>. The password opens your encrypted database; at least 12 characters. A new
        recovery code is made with it.
      </p>
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
  {/if}
</div>
