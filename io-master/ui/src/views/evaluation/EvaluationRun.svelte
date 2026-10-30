<!--
  The assessment of an evaluation by the AI (its commands
  evaluation_assess_preview, evaluation_assess and evaluation_assess_stop in
  src/panels/views/evaluation/run.rs): every work unit in the order of the
  work, without the evaluator (user, 2026-10-30). "Assess with AI" passes
  over the work units with a verdict, "Assess again" assesses anew those of
  the AI; those of the evaluator never. Before it starts, the evaluator
  confirms how many questions go to which agents; a document or a proof
  missing is named and the assessment cannot start. Each step is told as it
  ends (onstep, so that the list is read again); "Stop" ends it after its
  present work unit.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import {
    evaluationAssess,
    evaluationAssessPreview,
    evaluationAssessStop,
    type AssessPreviewView,
    type AssessStepView,
    type AssessSummaryView,
  } from "../../lib/api/evaluation";

  let { evaluation, onstep }: { evaluation: number; onstep: () => void } = $props();

  /** The most steps with notes kept on the page: the newest. */
  const SHOWN = 200;

  let asked: { again: boolean; preview: AssessPreviewView } | null = $state(null);
  let running = $state(false);
  let stopping = $state(false);
  let last: AssessStepView | null = $state(null);
  let steps: AssessStepView[] = $state([]);
  let summary: AssessSummaryView | null = $state(null);
  let problem = $state("");

  /** Counts what the assessment would ask, now, and asks the evaluator to confirm. */
  async function confirm(again: boolean) {
    problem = "";
    asked = null;

    try {
      asked = { again, preview: await evaluationAssessPreview(evaluation, again) };
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function assess(again: boolean) {
    asked = null;
    problem = "";
    summary = null;
    steps = [];
    last = null;
    running = true;
    stopping = false;

    try {
      summary = await evaluationAssess(evaluation, again, (step) => {
        last = step;

        if (step.verdict === null || step.notes.length > 0) {
          steps = [step, ...steps].slice(0, SHOWN);
        }

        onstep();
      });
    } catch (error) {
      problem = sentence(error);
    } finally {
      running = false;
      onstep();
    }
  }

  async function stop() {
    stopping = true;

    try {
      await evaluationAssessStop();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<section class="card">
  <h2>Assessment by the AI</h2>

  {#if problem}<p class="problem">{problem}</p>{/if}

  {#if running}
    <p>
      {#if last !== null}
        {last.number} of {last.total}: {last.label} · {last.verdict ?? "not assessed"}
      {:else}
        The first work unit is being assessed…
      {/if}
      <button type="button" class="ghost" disabled={stopping} onclick={stop}>{stopping ? "Stopping…" : "Stop"}</button>
    </p>
  {:else if asked !== null}
    {@const preview = asked.preview}
    {#if preview.missing.length > 0}
      <p class="problem">Give first what the work units need:</p>
      <ul>
        {#each preview.missing as one (one.name)}
          <li>{one.name} · needed by {one.count} work units</li>
        {/each}
      </ul>
      <p><button type="button" class="ghost" onclick={() => (asked = null)}>Close</button></p>
    {:else if preview.questions === 0}
      <p class="note">No work unit to assess: each is assessed already, or written by the evaluator.</p>
      <p><button type="button" class="ghost" onclick={() => (asked = null)}>Close</button></p>
    {:else}
      <p>
        {preview.questions} questions will be asked:
        {preview.agents.map((one) => `${one.count} of ${one.name}`).join(", ")}.
      </p>
      <p class="buttons">
        <button type="button" onclick={() => assess(asked?.again ?? false)}>Start</button>
        <button type="button" class="ghost" onclick={() => (asked = null)}>Cancel</button>
      </p>
    {/if}
  {:else}
    <p class="note">
      Every work unit in the order of the work: first those without dependencies, then those whose dependencies are
      assessed. The results of the evaluator are never written over.
    </p>
    <p class="buttons">
      <button type="button" onclick={() => confirm(false)}>Assess with AI</button>
      <button type="button" class="ghost" onclick={() => confirm(true)}>Assess again</button>
    </p>
  {/if}

  {#if summary !== null}
    <p class="note">
      {summary.assessed} assessed, {summary.failed} not assessed{summary.stopped ? ", stopped" : ""}.
    </p>
  {/if}

  {#if steps.length > 0}
    <details>
      <summary>What was not taken or not assessed ({steps.length})</summary>
      <ul class="steps">
        {#each steps as step (step.number)}
          <li><strong>{step.label}</strong>: {step.notes.join("; ")}</li>
        {/each}
      </ul>
    </details>
  {/if}
</section>

<style>
  ul.steps {
    max-height: 16rem;
    overflow-y: auto;
  }
</style>
