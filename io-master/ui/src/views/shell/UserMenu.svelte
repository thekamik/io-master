<!--
  The element "user menu" of the view Shell (src/views/shell/user_menu/):
  the logged-in evaluator at the foot of the sidebar, and the logout. When
  the evaluator is logged out it tells the application (onloggedout),
  which reads the state of the program again.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import { logOut } from "../../lib/api/shell/user_menu";

  let { user, onloggedout }: { user: string | null; onloggedout: () => void } = $props();

  let problem = $state("");

  async function leave() {
    problem = "";

    try {
      await logOut();
      onloggedout();
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
</div>
{#if problem}<p class="problem">{problem}</p>{/if}
