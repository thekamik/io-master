<!--
  The panel of an output of a task of the Evaluation lab (its commands
  outputs_* in src/views/evaluation_lab/evaluation_lab.rs, step 52): where
  it goes and when; while its condition holds for the verdict, its AI drafts
  its text from the four results (as its target, format and instructions
  ask), and the evaluator accepts or corrects it. The accepted texts make
  the report. After a change it gives the screen back (onchanged).
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as evaluationLab from "../../lib/api/evaluation_lab";
  import { follow, idle, start } from "../../components/ai_work/ai_progress";
  import LabInstructions from "./LabInstructions.svelte";

  let {
    evaluation,
    screen,
    block,
    onchanged,
  }: {
    evaluation: string;
    screen: evaluationLab.LabTaskScreenView;
    block: evaluationLab.LabBlockView;
    onchanged: (screen: evaluationLab.LabTaskScreenView) => void;
  } = $props();

  const conditions = { always: "always", not_met: "only when the requirement is not met", met: "only when the requirement is met" };

  let output = $derived(screen.outputs.find((candidate) => candidate.block === block.uuid));
  let text = $state("");
  let progress = $state(idle());
  let instructions = $state(false);
  let saved = $state("");

  // The text kept, again whenever the program gives a new screen.
  $effect(() => {
    text = output?.text ?? "";
  });

  async function act(work: () => Promise<evaluationLab.LabTaskScreenView>, done = "") {
    progress.problem = "";

    try {
      onchanged(await work());
      saved = done;
    } catch (error) {
      progress.problem = sentence(error);
    }
  }

  async function draft() {
    const what = "The AI is writing the text";

    start(progress, what);
    await act(() => evaluationLab.outputsDraft(evaluation, block.uuid, (event) => follow(progress, event, what)));
    progress.running = false;
  }
</script>

{#if instructions}<LabInstructions which="output_draft" onclose={() => (instructions = false)} />{/if}

<h3>Output: {block.name}</h3>
<p class="note">{block.format === "latex" ? "A LaTeX template" : "A text file"} · filled {conditions[block.condition]}{block.target ? ` · ${block.target}` : ""}</p>
{#if block.instructions}<p class="note">{block.instructions}</p>{/if}
{#if progress.problem}<p class="problem">{progress.problem}</p>{/if}
{#if progress.line}<p class="note">{progress.line}</p>{/if}

{#if !output?.applies}
  <p class="note">This output waits: its condition does not hold for the verdict set (on the requirement).</p>
{:else}
  <div class="buttons">
    {#if progress.running}
      <button type="button" class="small danger" onclick={() => evaluationLab.evaluationLabStop()}>Stop</button>
    {:else}
      <button type="button" class="small" onclick={draft}>Draft with AI</button>
    {/if}
    <button type="button" class="small" onclick={() => (instructions = true)}>Instructions…</button>
  </div>
  {#if output?.proposal}
    <div class="ai-proposal">
      <strong>The AI drafted:</strong>
      <pre class="mono">{output.proposal}</pre>
      <div class="buttons">
        <button type="button" onclick={() => act(() => evaluationLab.outputsAccept(evaluation, block.uuid))}>Accept the draft</button>
        <button type="button" class="danger" onclick={() => act(() => evaluationLab.outputsReject(evaluation, block.uuid))}>Reject the draft</button>
      </div>
    </div>
  {/if}
  <form class="block-form" onsubmit={(event) => { event.preventDefault(); act(() => evaluationLab.outputsSave(evaluation, block.uuid, text), "The text was saved."); }}>
    {#if saved}<p class="done">{saved}</p>{/if}
    <label>Text <textarea class="mono" bind:value={text} rows="10"></textarea></label>
    <div class="buttons"><button type="submit">Save</button></div>
  </form>
{/if}
