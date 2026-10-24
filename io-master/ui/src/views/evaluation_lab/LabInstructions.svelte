<!--
  The instructions of a work of the AI of the Evaluation lab (its commands
  evaluation_lab_instructions_get and _save in
  src/views/evaluation_lab/evaluation_lab.rs, step 52): the chat of an
  input, the fragments, the assessment, the outputs. A dialog; saving an
  empty text brings back the default ones.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as evaluationLab from "../../lib/api/evaluation_lab";

  let { which, onclose }: { which: evaluationLab.LabWork; onclose: () => void } = $props();

  const titles = {
    input_chat: "the chat of an input",
    evidence_fragments: "the fragments of the evidence",
    assessment: "the agent of a requirement",
    output_draft: "the text of an output",
  };

  let text = $state("");
  let problem = $state("");
  let notice = $state("");

  onMount(async () => {
    try {
      text = await evaluationLab.evaluationLabInstructionsGet(which);
    } catch (error) {
      problem = sentence(error);
    }
  });

  async function save(written: string) {
    try {
      text = await evaluationLab.evaluationLabInstructionsSave(which, written);
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
    <p class="note">Evaluation support, kept in your database, used in every evaluation. Saving an empty text brings back the default ones.</p>
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
