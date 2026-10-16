<!--
  The first start: the user and the role of the installation.

  Unlike the web interface there is no one-time setup code: it
  protected a network port, and the window has none.
-->
<script lang="ts">
  import { sentence, setUp, type AppView } from "../lib/api";

  let { onchange }: { onchange: (app: AppView) => void } = $props();

  let name = $state("");
  let password = $state("");
  let passwordRepeat = $state("");
  let role = $state("");
  let roleConfirmed = $state(false);
  let problem = $state("");
  let busy = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      onchange(await setUp({ name, password, passwordRepeat, role, roleConfirmed }));
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }
</script>

<main class="login wide">
  <div class="mark large">IO</div>
  <h1>Install IO Master</h1>
  <p class="note">
    This is the first start. The person who installs IO Master is its user. The role decides what this installation
    is for, and it cannot be changed later.
  </p>

  {#if problem}<p class="problem">{problem}</p>{/if}

  <form onsubmit={submit}>
    <label>Name <input bind:value={name} autocomplete="off" required /></label>
    <label>
      Password
      <input type="password" bind:value={password} autocomplete="new-password" minlength="12" required />
    </label>
    <label>
      Password again
      <input type="password" bind:value={passwordRepeat} autocomplete="new-password" minlength="12" required />
    </label>

    <fieldset>
      <legend>Role of this installation</legend>
      <label class="choice">
        <input type="radio" bind:group={role} value="workstation" />
        <strong>Workstation</strong>: the computer of an evaluator. AI, chats, devices, evaluations.
      </label>
      <label class="choice">
        <input type="radio" bind:group={role} value="master" />
        <strong>Master</strong>: a separate machine that only serves password resets.
      </label>
    </fieldset>

    <label class="choice">
      <input type="checkbox" bind:checked={roleConfirmed} />
      I understand that the role cannot be changed later.
    </label>

    <button type="submit" disabled={busy}>{busy ? "Installing…" : "Install"}</button>
  </form>
</main>
