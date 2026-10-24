<!--
  The panel of the requirement of a task of the Evaluation lab (its
  commands assessment_* in src/views/evaluation_lab/evaluation_lab.rs, step
  52): its agent of the AI reads the requirement and only the accepted
  fragments and proposes the four results and a verdict, with its reason;
  the evaluator accepts or rejects the proposal, writes the results and
  sets the verdict (P1). After a change it gives the screen back
  (onchanged).
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as evaluationLab from "../../lib/api/evaluation_lab";
  import { follow, idle, start } from "./ai_progress";
  import LabInstructions from "./LabInstructions.svelte";

  let {
    evaluation,
    screen,
    onchanged,
  }: {
    evaluation: string;
    screen: evaluationLab.LabTaskScreenView;
    onchanged: (screen: evaluationLab.LabTaskScreenView) => void;
  } = $props();

  const verdicts = { met: "met", not_met: "not met", inconclusive: "inconclusive" };
  const fields = ["findings", "analysis", "assessment", "vulnerabilities"] as const;

  let form: evaluationLab.ResultsForm = $state({ findings: "", analysis: "", assessment: "", vulnerabilities: "", verdict: null });
  let progress = $state(idle());
  let instructions = $state(false);
  let saved = $state("");

  let accepted = $derived(screen.inputs.reduce((sum, input) => sum + input.fragments.filter((fragment) => fragment.accepted).length, 0));

  // The results kept, again whenever the program gives a new screen.
  $effect(() => {
    form = { ...screen.results };
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

  async function assess() {
    const what = "The agent is assessing the requirement";

    start(progress, what);
    await act(() => evaluationLab.assessmentPropose(evaluation, screen.uuid, (event) => follow(progress, event, what)));
    progress.running = false;
  }

  function save(event: SubmitEvent) {
    event.preventDefault();
    act(() => evaluationLab.assessmentSave(evaluation, screen.uuid, { ...form }), "The results were saved.");
  }
</script>

{#if instructions}<LabInstructions which="assessment" onclose={() => (instructions = false)} />{/if}

<h3>Requirement</h3>
<p class="note">Its agent assesses the requirement on the accepted fragments only ({accepted} now). The verdict is yours.</p>
{#if progress.problem}<p class="problem">{progress.problem}</p>{/if}
{#if progress.line}<p class="note">{progress.line}</p>{/if}
<div class="buttons">
  {#if progress.running}
    <button type="button" class="small danger" onclick={() => evaluationLab.evaluationLabStop()}>Stop</button>
  {:else}
    <button type="button" class="small" onclick={assess}>Assess with AI</button>
  {/if}
  <button type="button" class="small" onclick={() => (instructions = true)}>Instructions…</button>
</div>

{#if screen.proposal}
  <div class="ai-proposal">
    <strong>The agent proposes{screen.proposal.results.verdict ? `: ${verdicts[screen.proposal.results.verdict]}` : ""}.</strong>
    {#if screen.proposal.reason}<p>{screen.proposal.reason}</p>{/if}
    <dl>
      {#each fields as field, index (field)}
        <dt>{screen.result_titles[index]}</dt><dd>{screen.proposal.results[field]}</dd>
      {/each}
    </dl>
    <div class="buttons">
      <button type="button" onclick={() => act(() => evaluationLab.assessmentAccept(evaluation, screen.uuid))}>Accept the proposal</button>
      <button type="button" class="danger" onclick={() => act(() => evaluationLab.assessmentReject(evaluation, screen.uuid))}>Reject the proposal</button>
    </div>
  </div>
{/if}

<form class="block-form" onsubmit={save}>
  {#if saved}<p class="done">{saved}</p>{/if}
  {#each fields as field, index (field)}
    <label>{screen.result_titles[index]} <textarea bind:value={form[field]} rows="4"></textarea></label>
  {/each}
  <label>
    Verdict
    <select bind:value={form.verdict}>
      <option value={null}>not set</option>
      {#each Object.entries(verdicts) as [value, title] (value)}<option {value}>{title}</option>{/each}
    </select>
  </label>
  <div class="buttons"><button type="submit">Save</button></div>
</form>
