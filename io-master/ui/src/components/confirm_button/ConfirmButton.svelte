<!--
  The component "confirm button" (a component of the page only; the audit of
  2026-10-25): a button that deletes or removes something that cannot be
  brought back asks first. The first click shows the same label again, to
  confirm, and "Keep"; only the second click calls onconfirm. Which state is
  shown is presentation, kept here.
-->
<script lang="ts">
  let {
    label = "Delete",
    small = true,
    disabled = false,
    onconfirm,
  }: { label?: string; small?: boolean; disabled?: boolean; onconfirm: () => void } = $props();

  let asking = $state(false);

  function confirm() {
    asking = false;
    onconfirm();
  }
</script>

{#if asking}
  <span class="confirm-question">Sure?</span>
  <button type="button" class="danger" class:small {disabled} onclick={confirm}>{label}</button>
  <button type="button" class:small onclick={() => (asking = false)}>Keep</button>
{:else}
  <button type="button" class="danger" class:small {disabled} onclick={() => (asking = true)}>{label}</button>
{/if}
