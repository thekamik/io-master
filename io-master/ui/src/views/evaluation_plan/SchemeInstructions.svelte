<!--
  The instructions of the AI that proposes the scheme of a task (its
  commands task_scheme_instructions_get and _save in
  src/views/evaluation_plan/evaluation_plan.rs, step 51.3): a dialog opened
  from TaskScheme. Saving an empty text brings back the default ones.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";

  let { onclose }: { onclose: () => void } = $props();

  let text = $state("");
  let problem = $state("");
  let notice = $state("");

  onMount(async () => {
    try {
      text = await evaluationPlan.taskSchemeInstructionsGet();
    } catch (error) {
      problem = sentence(error);
    }
  });

  async function save(written: string) {
    try {
      text = await evaluationPlan.taskSchemeInstructionsSave(written);
      notice = "The instructions were saved.";
      problem = "";
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="overlay" role="presentation">
  <div class="dialog wide">
    <h2>Instructions to the AI: the scheme of a task</h2>
    <p class="note">Evaluation support, kept in your database, used for every task. Saving an empty text brings back the default ones.</p>
    {#if problem}<p class="problem">{problem}</p>{/if}
    {#if notice}<p class="done">{notice}</p>{/if}
    <textarea class="mono" rows="18" bind:value={text}></textarea>
    <div class="buttons">
      <button type="button" onclick={() => save(text)}>Save</button>
      <button type="button" onclick={() => save("")}>Default</button>
      <button type="button" onclick={onclose}>Close</button>
    </div>
  </div>
</div>
