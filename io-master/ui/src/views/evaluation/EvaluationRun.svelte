<!--
  The assessment of an evaluation by the AI (its commands
  evaluation_assess_preview, evaluation_assess and evaluation_assess_stop in
  src/panels/views/evaluation/run.rs): every work unit in the order of the
  work, without the evaluator (user, 2026-10-30). "Assess with AI" passes
  over the work units with a verdict, "Assess again" assesses anew those of
  the AI; those of the evaluator never. Before it starts, the evaluator
  confirms how many questions go to which agents; a document or a proof
  missing is named and the assessment cannot start. The progress is asked
  for while the assessment runs, and when the panel is opened again
  (evaluation_assess_progress), so that an assessment that runs shows its bar
  after the evaluator left the panel (user, 2026-11-01); each step gone
  through is told (onstep, so that the list is read again). "Stop" ends it
  at once: the answer awaited is not taken.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import {
    evaluationAssess,
    evaluationAssessPreview,
    evaluationAssessProgress,
    evaluationAssessStop,
    type AssessPreviewView,
  } from "../../lib/api/evaluation";
  import { PROGRESS_INTERVAL, type ProgressView } from "../../lib/api/progress";

  let { evaluation, onstep }: { evaluation: number; onstep: () => void } = $props();

  let asked: { again: boolean; preview: AssessPreviewView } | null = $state(null);
  /** The command of an assessment started from this page runs. */
  let started = $state(false);
  /** The progress of the last assessment of this evaluation; null when there was none. */
  let progress = $state<ProgressView | null>(null);
  let problem = $state("");

  let running = $derived(started || (progress?.running ?? false));
  let gone = $derived(progress === null ? 0 : progress.assessed + progress.failed);

  /** Reads the progress; tells the editor when a work unit was gone through. */
  async function refresh() {
    try {
      const next = await evaluationAssessProgress(evaluation);
      // Read after the await, so that the effect that calls refresh does
      // not depend on the progress it writes.
      const before = progress === null ? 0 : progress.assessed + progress.failed;
      const after = next === null ? 0 : next.assessed + next.failed;

      progress = next;

      if (after !== before) {
        onstep();
      }
    } catch (error) {
      problem = sentence(error);
    }
  }

  // The progress when the panel is shown, or shows another evaluation.
  $effect(() => {
    void refresh();
  });

  // While an assessment runs, its progress is asked for again and again.
  $effect(() => {
    if (!running) {
      return;
    }

    const timer = setInterval(() => void refresh(), PROGRESS_INTERVAL);

    return () => clearInterval(timer);
  });

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
    started = true;

    try {
      await evaluationAssess(evaluation, again);
    } catch (error) {
      problem = sentence(error);
    } finally {
      started = false;
      await refresh();
      onstep();
    }
  }

  async function stop() {
    try {
      await evaluationAssessStop();
      await refresh();
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
      {#if progress !== null && progress.label}
        {gone} of {progress.total}: {progress.label} · {progress.lastVerdict ?? "not assessed"}
      {:else}
        The first work unit is being assessed…
      {/if}
      <button type="button" class="ghost" disabled={progress?.stopping} onclick={stop}>
        {progress?.stopping ? "Stopping…" : "Stop"}
      </button>
    </p>
    {#if progress !== null}<progress max={progress.total} value={gone}></progress>{/if}
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

  {#if progress !== null && !running}
    <p class="note">
      {progress.assessed} assessed, {progress.failed} not assessed{progress.stopping && gone < progress.total
        ? ", stopped"
        : ""}.
    </p>
  {/if}

  {#if progress !== null && progress.problems.length > 0}
    <details>
      <summary>What was not taken or not assessed ({progress.problems.length})</summary>
      <ul class="steps">
        {#each progress.problems as one, index (index)}
          <li><strong>{one.label}</strong>: {one.notes.join("; ")}</li>
        {/each}
      </ul>
    </details>
  {/if}
</section>

<style>
  progress {
    width: 100%;
  }

  ul.steps {
    max-height: 16rem;
    overflow-y: auto;
  }
</style>
