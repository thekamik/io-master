<!--
  The element "login form" of the view Users (its commands login_form_log_in
  and login_form_recover in src/views/users/users.rs). "I do not remember my
  password" switches it to the recovery: the recovery code and a new password;
  then the new code is shown once (step 49 of docs/history.md). When the
  evaluator is logged in it tells the application (ondone), which reads the
  state of the program again. Which of the two forms is shown is presentation.
-->
<script lang="ts">
  import RecoveryCode from "../../components/recovery_code/RecoveryCode.svelte";
  import type { RecoveryCodeView } from "../../lib/api/components/recovery_code";
  import { sentence } from "../../lib/api/errors";
  import { loginFormLogIn, loginFormRecover } from "../../lib/api/users";

  let { ondone }: { ondone: () => void } = $props();

  let recovering = $state(false);
  let name = $state("");
  let password = $state("");
  let passwordRepeat = $state("");
  let typedCode = $state("");
  let problem = $state("");
  let busy = $state(false);
  let code: RecoveryCodeView | null = $state(null);

  async function logIn(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      await loginFormLogIn(name, password);
      ondone();
    } catch (error) {
      problem = sentence(error);
    } finally {
      password = "";
      busy = false;
    }
  }

  async function recover(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      code = await loginFormRecover({ name, code: typedCode, password, passwordRepeat });
    } catch (error) {
      problem = sentence(error);
    } finally {
      typedCode = password = passwordRepeat = "";
      busy = false;
    }
  }

  function switchTo(recovery: boolean) {
    recovering = recovery;
    problem = "";
    typedCode = password = passwordRepeat = "";
  }
</script>

<main class="login" class:wide={recovering}>
  <div class="mark large">IO</div>
  {#if code}
    <RecoveryCode view={code} onconfirmed={ondone} />
  {:else if recovering}
    <h1>A new password</h1>
    <p class="note">Enter your recovery code and choose a new password. You get a new recovery code afterwards.</p>

    {#if problem}<p class="problem">{problem}</p>{/if}

    <form onsubmit={recover}>
      <label>Name <input bind:value={name} autocomplete="username" required /></label>
      <label>
        Recovery code
        <input class="mono" bind:value={typedCode} autocomplete="off" spellcheck="false" required />
      </label>
      <label>
        New password
        <input type="password" bind:value={password} autocomplete="new-password" minlength="12" required />
      </label>
      <label>
        New password again
        <input type="password" bind:value={passwordRepeat} autocomplete="new-password" minlength="12" required />
      </label>
      <button type="submit" disabled={busy}>{busy ? "Setting…" : "Set the new password"}</button>
    </form>
    <button type="button" class="link" onclick={() => switchTo(false)}>Back to the login</button>
  {:else}
    <h1>IO Master</h1>
    <p class="note">Log in to open your workstation.</p>

    {#if problem}<p class="problem">{problem}</p>{/if}

    <form onsubmit={logIn}>
      <label>Name <input bind:value={name} autocomplete="username" required /></label>
      <label>Password <input type="password" bind:value={password} autocomplete="current-password" required /></label>
      <button type="submit" disabled={busy}>{busy ? "Logging in…" : "Log in"}</button>
    </form>
    <button type="button" class="link" onclick={() => switchTo(true)}>I do not remember my password</button>
  {/if}
</main>
