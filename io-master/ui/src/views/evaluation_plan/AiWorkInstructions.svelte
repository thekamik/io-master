<!--
  The instructions of a work of the AI of the view (its commands
  ai_work_instructions_get and _save in
  src/views/evaluation_plan/evaluation_plan.rs, steps 51.3 and 51.4): the
  scheme of a task (opened from TaskScheme), the choice of the tasks
  (TaskProposals), the related elements (TaskLinks). A dialog; saving an
  empty text brings back the default ones.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";

  let { which, onclose }: { which: evaluationPlan.AiWork; onclose: () => void } = $props();

  const titles = {
    task_scheme: "the scheme of a task",
    requirement_choice: "the choice of the tasks",
    related_elements: "the related elements of a task",
  };

  let text = $state("");
  let problem = $state("");
  let notice = $state("");

  onMount(async () => {
    try {
      text = await evaluationPlan.aiWorkInstructionsGet(which);
    } catch (error) {
      problem = sentence(error);
    }
  });

  async function save(written: string) {
    try {
      text = await evaluationPlan.aiWorkInstructionsSave(which, written);
      notice = "The instructions were saved.";
      problem = "";
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="overlay" role="presentation">
  <div class="dialog wide">
    <h2>Instructions to the AI: {titles[which]}</h2>
    <p class="note">Evaluation support, kept in your database, used in every project. Saving an empty text brings back the default ones.</p>
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
