<!--
  The panel Account (its commands in src/panels/views/account/): the
  password of the user logged in and their recovery code, each changed with
  the password. The models User and UserKeys; nothing of them is shown but
  the new recovery code, once.
-->
<script lang="ts">
  import PasswordForm from "./PasswordForm.svelte";
  import RecoveryCodeForm from "./RecoveryCodeForm.svelte";
</script>

<div class="page-head">
  <h1>Account</h1>
  <p>Your password and your recovery code.</p>
</div>

<PasswordForm />
<RecoveryCodeForm />
