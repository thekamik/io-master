<!--
  The element "password form" of the view Settings, tab User
  (src/views/settings/password_form/). The passwords are cleared from the
  page as soon as they were sent.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import { change } from "../../lib/api/settings/password_form";

  let { user }: { user: string | null } = $props();

  let current = $state("");
  let password = $state("");
  let passwordRepeat = $state("");
  let notice: { done: boolean; text: string } | null = $state(null);

  async function submit(event: SubmitEvent) {
    event.preventDefault();

    try {
      await change(current, password, passwordRepeat);
      notice = { done: true, text: "Your password was changed." };
    } catch (error) {
      notice = { done: false, text: `The password was not changed: ${sentence(error)}` };
    } finally {
      current = password = passwordRepeat = "";
    }
  }
</script>

<form class="panel" onsubmit={submit}>
  <h3>Your password</h3>
  <p class="note">
    Signed in as <strong>{user}</strong>. The password opens your encrypted database; at least 12 characters.
  </p>
  {#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}
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
