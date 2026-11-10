<!--
  "Fill with AI" in the editor of a plan (its commands plan_fill_parameters,
  plan_fill_dependencies and plan_fill_stop in
  src/panels/views/evaluation_plan/fill.rs): the agent of the planner,
  chosen in the data of the plan, fills every work unit of the plan in its
  order, over what was there; two buttons, two tasks (user, 2026-10-29):
  the parameters, then the dependencies, when every work unit is described.
  A dependency that closes a cycle is written and its work units turn red in
  the list: the planner decides. The progress is asked for while the
  filling runs, and when the editor is opened again (plan_fill_progress), so
  that a filling that runs shows its bar after the planner left the panel
  (user, 2026-11-01); "Stop" ends the filling at once: the answer awaited is not taken. Before a filling starts, the
  planner confirms it: how many questions go to which integration, counted
  by the program when the button is pressed (plan_fill_preview). Tells the
  editor after the
  filling (onchanged), so that the list of the work units is read again.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import {
    planFillDependencies,
    planFillParameters,
    planFillPreview,
    planFillProgress,
    planFillStop,
    type FillPreviewView,
    type FillTask,
    type PlanView,
  } from "../../lib/api/evaluation_plan";
  import { PROGRESS_INTERVAL, type ProgressView } from "../../lib/api/progress";

  let { plan, view, onchanged }: { plan: number; view: PlanView | null; onchanged: () => void } = $props();

  /** The task started from this page while its command runs; null otherwise. */
  let started: FillTask | null = $state(null);
  /** The task the planner is asked to confirm, and what it would ask. */
  let asked: { task: FillTask; preview: FillPreviewView } | null = $state(null);
  /** The progress of the last filling of this plan; null when there was none. */
  let progress = $state<ProgressView | null>(null);
  let problem = $state("");

  let agent = $derived(view?.agents.find((one) => one.id === view?.aiAgent)?.name ?? null);
  let running = $derived(started !== null || (progress?.running ?? false));
  let gone = $derived(progress === null ? 0 : progress.assessed + progress.failed);

  /** Reads the progress; tells the editor when a filling has just ended. */
  async function refresh() {
    try {
      const next = await planFillProgress(plan);

      // Read after the await, so that the effect that calls refresh does
      // not depend on the progress it writes.
      if (progress?.running && !next?.running) {
        onchanged();
      }

      progress = next;
    } catch (error) {
      problem = sentence(error);
    }
  }

  // The progress when the editor is shown, or shows another plan.
  $effect(() => {
    void refresh();
  });

  // While a filling runs, its progress is asked for again and again.
  $effect(() => {
    if (!running) {
      return;
    }

    const timer = setInterval(() => void refresh(), PROGRESS_INTERVAL);

    return () => clearInterval(timer);
  });

  /** Counts what the task would ask, now, and asks the planner to confirm. */
  async function confirm(task: FillTask) {
    problem = "";
    asked = null;

    try {
      asked = { task, preview: await planFillPreview(plan) };
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function fill(task: FillTask) {
    asked = null;
    problem = "";
    started = task;

    try {
      // Each task is its own command.
      if (task === "parameters") {
        await planFillParameters(plan);
      } else {
        await planFillDependencies(plan);
      }
    } catch (error) {
      problem = sentence(error);
    } finally {
      started = null;
      await refresh();
      onchanged();
    }
  }

  async function stop() {
    try {
      await planFillStop();
      await refresh();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<section class="card">
  <h2>Fill with AI</h2>
  {#if agent === null}
    <p class="note">Choose the agent of the planner in the data of the plan: its knowledge fills the work units.</p>
  {:else}
    <p class="note">
      The agent <strong>{agent}</strong> goes through every work unit in the order of the plan, over what is there; each
      question is kept in its journal. First the parameters, then the dependencies, when every work unit is described. A
      dependency that closes a cycle turns its work units red: correct them in their editor.
    </p>
    <p class="buttons">
      <button type="button" disabled={running} onclick={() => confirm("parameters")}>
        {started === "parameters" ? "Filling the parameters…" : "Fill parameters"}
      </button>
      <button type="button" disabled={running} onclick={() => confirm("dependencies")}>
        {started === "dependencies" ? "Filling the dependencies…" : "Fill dependencies"}
      </button>
      {#if running}
        <button type="button" class="ghost" disabled={progress?.stopping} onclick={stop}>
          {progress?.stopping ? "Stopping…" : "Stop"}
        </button>
      {/if}
    </p>
  {/if}

  {#if asked}
    <div class="confirm">
      <p>
        {asked.task === "parameters" ? "Fill the parameters" : "Fill the dependencies"}: <strong>{asked.preview.questions}</strong>
        questions of the agent <strong>{asked.preview.agent}</strong> to <strong>{asked.preview.integration}</strong>, one per
        work unit, each answer written over what is there.
      </p>
      <p class="buttons">
        <button type="button" class="primary" onclick={() => asked && fill(asked.task)}>Go on</button>
        <button type="button" class="ghost" onclick={() => (asked = null)}>Cancel</button>
      </p>
    </div>
  {/if}

  {#if progress !== null && running}
    <p>
      {#if progress.label}{gone} of {progress.total}: {progress.label}{:else}The filling begins…{/if}
      <progress max={progress.total} value={gone}></progress>
    </p>
  {:else if progress !== null}
    <p class={progress.failed > 0 ? "problem" : "done"}>
      {progress.assessed} filled, {progress.failed} not filled{progress.stopping && gone < progress.total
        ? ", stopped"
        : ""}.
    </p>
  {/if}
  {#if problem}<p class="problem">{problem}</p>{/if}

  {#each progress?.problems ?? [] as one, index (index)}
    <div class="step">
      <span class="badge warning">{one.label}</span>
      {one.notes.join("; ")}
    </div>
  {/each}
</section>

<style>
  .confirm {
    padding: 0.75rem 1rem;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .buttons button + button {
    margin-left: 0.4rem;
  }

  progress {
    width: 100%;
  }

  .step {
    margin: 0.3rem 0;
    font-size: 0.9rem;
  }
</style>
