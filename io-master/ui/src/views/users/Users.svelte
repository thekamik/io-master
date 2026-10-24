<!--
  The view Users, its page User: the settings of the logged-in evaluator,
  those of the account and of the encryption of its database (step 47 of
  docs/history.md): the password and the recovery code (step 49); later the
  removal of the installation. The settings of the program are on the page
  Settings. The setup and the login (SetupForm, LoginForm) are shown by
  App.svelte before anybody is logged in, the user menu (UserMenu) by the
  frame of the window.
-->
<script lang="ts">
  import PasswordForm from "./PasswordForm.svelte";
  import RecoveryCodeForm from "./RecoveryCodeForm.svelte";

  let { user }: { user: string | null } = $props();
</script>

<PasswordForm {user} />
<RecoveryCodeForm />
