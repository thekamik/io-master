<!--
  The element "user menu" of the view Users, shown in the frame of the window (its command
  user_menu_log_out in src/views/users/users.rs):
  the logged-in evaluator at the foot of the sidebar, the logout, and the
  logout that closes the program (user_menu_close). When the evaluator is
  logged out it tells the application (onloggedout), which reads the state
  of the program again; after "Log out and close" the program ends.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import { userMenuClose, userMenuLogOut } from "../../lib/api/users";

  let { user, onloggedout }: { user: string | null; onloggedout: () => void } = $props();

  let problem = $state("");

  async function leave() {
    problem = "";

    try {
      await userMenuLogOut();
      onloggedout();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function close() {
    problem = "";

    try {
      await userMenuClose();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="user">
  <span class="avatar">{user?.charAt(0) ?? "?"}</span>
  <span class="who">{user}</span>
  <button class="ghost" onclick={leave} title="Log out">
    <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 4h5v16h-5M10 8l-4 4 4 4M6 12h10" /></svg>
  </button>
  <button class="ghost" onclick={close} title="Log out and close">
    <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v8M7 6.5a7 7 0 1 0 10 0" /></svg>
  </button>
</div>
{#if problem}<p class="problem">{problem}</p>{/if}
