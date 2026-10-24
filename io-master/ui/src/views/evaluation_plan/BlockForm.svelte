<!--
  The form of the block chosen in the scheme of a task (its commands
  task_scheme_save_block and task_scheme_remove_block in
  src/views/evaluation_plan/evaluation_plan.rs): an input (its name, the
  evidence expected, the instructions for its agent), the requirement (the
  instructions for its agent; its text is the standard's), an output (its
  name, format, target, condition, the instructions for its AI). After a
  change it gives the scheme back to the screen (onchanged).
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";
  import OutputFields from "./OutputFields.svelte";

  let {
    block,
    onchanged,
  }: { block: evaluationPlan.BlockView; onchanged: (scheme: evaluationPlan.SchemeView) => void } = $props();

  let form: evaluationPlan.BlockForm = $state({
    name: "",
    evidence: "",
    instructions: "",
    target: "",
    format: "text",
    condition: "always",
  });
  let problem = $state("");

  // The fields of the block chosen, again whenever another is chosen.
  $effect(() => {
    form = {
      name: block.name,
      evidence: block.evidence,
      instructions: block.instructions,
      target: block.target,
      format: block.format,
      condition: block.condition,
    };
    problem = "";
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();

    try {
      onchanged(await evaluationPlan.taskSchemeSaveBlock(block.uuid, { ...form }));
      problem = "";
    } catch (error) {
      problem = `The block was not saved: ${sentence(error)}`;
    }
  }

  async function remove() {
    try {
      onchanged(await evaluationPlan.taskSchemeRemoveBlock(block.uuid));
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<form class="block-form" onsubmit={save}>
  {#if problem}<p class="problem">{problem}</p>{/if}
  {#if block.kind === "input"}
    <h3>Input</h3>
    <label>Name <input bind:value={form.name} required /></label>
    <label>
      Expected evidence
      <textarea bind:value={form.evidence} rows="3" placeholder="What the evaluator delivers: a part of the Security Target, the output of a program, …"></textarea>
    </label>
    <label>
      Instructions for its agent
      <textarea bind:value={form.instructions} rows="3" placeholder="What to check before the evidence is ready to be assessed"></textarea>
    </label>
  {:else if block.kind === "requirement"}
    <h3>Requirement</h3>
    <p class="note">Its text is the standard's, beside. The agent of the requirement assesses the inputs connected to it.</p>
    <label>
      Instructions for its agent
      <textarea bind:value={form.instructions} rows="4" placeholder="What to assess"></textarea>
    </label>
  {:else}
    <h3>Output</h3>
    <OutputFields bind:fields={form} />
  {/if}
  <div class="buttons">
    <button type="submit">Save</button>
    {#if block.kind !== "requirement"}<button type="button" class="danger" onclick={remove}>Remove</button>{/if}
  </div>
</form>
