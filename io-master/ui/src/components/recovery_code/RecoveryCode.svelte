<!--
  The component "recovery code": the recovery code of a forgotten password,
  shown once, after the first start (and later after a change or a recovery
  of the password). The user confirms that it was written down by typing one
  of its groups; only then can they go on (onconfirmed), and the code is
  forgotten with the component. Presentation only; no command. It is the one
  secret the page ever shows (CLAUDE.md, "State lives in the core").
-->
<script lang="ts">
  import type { RecoveryCodeView } from "../../lib/api/program_start";

  let { view, onconfirmed }: { view: RecoveryCodeView; onconfirmed: () => void } = $props();

  const groups = $derived(view.code.split("-"));
  // Which group is asked for: chosen once, at random, so that it is not
  // always the first one that is checked.
  const asked = Math.floor(Math.random() * 7);
  let typed = $state("");

  const confirmed = $derived(typed.trim().toUpperCase() === (groups[asked] ?? "\u0000"));
</script>

<div class="recovery-code">
  <h3>Your recovery code</h3>
  <p class="note">
    With this code you can set a new password if you forget yours. Write it down or print it and keep it away from
    this computer, for example in the safe of the laboratory. It is shown only now: IO Master cannot show it again.
    Whoever has it can open your data.
  </p>
  <!-- A line breaks only between the groups, never inside one. -->
  <p class="code mono">
    {#each groups as group, index (index)}<span class="group">{group}</span>{#if index < groups.length - 1}-<wbr
        />{/if}{/each}
  </p>
  <label>
    To confirm that you wrote it down, type group {asked + 1} of 7
    <input class="mono" bind:value={typed} autocomplete="off" spellcheck="false" maxlength="5" />
  </label>
  <button type="button" disabled={!confirmed} onclick={onconfirmed}>I wrote it down, go on</button>
</div>
