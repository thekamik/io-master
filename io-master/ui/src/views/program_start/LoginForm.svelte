<!--
  The login (its commands login_log_in and login_recover in
  src/panels/views/program_start/login.rs): a name and a password. "I do not
  remember my password" switches it to the recovery: the name, the recovery
  code and a new password given twice; the new recovery code is then shown
  once. When the user is logged in, the form tells the application (ondone),
  which reads the state of the program again. Which of the two forms is
  shown is presentation.
-->
<script lang="ts">
  import RecoveryCode from "../../components/recovery_code/RecoveryCode.svelte";
  import { sentence } from "../../lib/api/errors";
  import { loginLogIn, loginRecover, type RecoveryCodeView } from "../../lib/api/program_start";

  let { ondone }: { ondone: () => void } = $props();

  let recovering = $state(false);
  let name = $state("");
  let password = $state("");
  let passwordRepeated = $state("");
  let typedCode = $state("");
  let problem = $state("");
  let busy = $state(false);
  let code: RecoveryCodeView | null = $state(null);

  async function logIn(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      await loginLogIn(name, password);
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
      code = await loginRecover({ name, code: typedCode, password, passwordRepeated });
    } catch (error) {
      problem = sentence(error);
    } finally {
      typedCode = password = passwordRepeated = "";
      busy = false;
    }
  }

  function switchTo(recovery: boolean) {
    recovering = recovery;
    problem = "";
    typedCode = password = passwordRepeated = "";
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
        <input class="mono" bind:value={typedCode} autocomplete="off" spellcheck="false" maxlength="128" required />
      </label>
      <label>
        New password
        <input type="password" bind:value={password} autocomplete="new-password" minlength="12" required />
      </label>
      <label>
        New password again
        <input type="password" bind:value={passwordRepeated} autocomplete="new-password" minlength="12" required />
      </label>
      <button type="submit" disabled={busy}>{busy ? "Setting it…" : "Set the new password"}</button>
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
