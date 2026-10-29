<!--
  The component "save status": what a form says of its last write, so that
  the user always sees that a change reached the program, also when the
  writing becomes automatic later. "Saving…" while a write runs, then
  "Saved at 14:32:05". The time is the local time of the page, only for
  information: it is kept nowhere (user, 2026-10-29). Presentation only; no
  command.
-->
<script lang="ts">
  let { busy, savedAt }: { busy: boolean; savedAt: Date | null } = $props();
</script>

<p class="save-status note" aria-live="polite">
  {#if busy}
    Saving…
  {:else if savedAt}
    Saved at {savedAt.toLocaleTimeString()}
  {/if}
</p>

<style>
  .save-status {
    min-height: 1.2rem;
    margin: 0.25rem 0;
  }
</style>
