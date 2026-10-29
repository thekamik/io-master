<!--
  "Fill with AI" in the editor of a plan (its commands plan_fill_parameters,
  plan_fill_dependencies and plan_fill_stop in
  src/panels/views/evaluation_plan/fill.rs): the agent of the planner,
  chosen in the data of the plan, fills every work unit of the plan in its
  order, over what was there; two buttons, two tasks (user, 2026-10-29):
  the parameters, then the dependencies, when every work unit is described.
  A dependency that closes a cycle is written and its work units turn red in
  the list: the planner decides. Each step is shown as it ends; "Stop" ends
  the filling after its present work unit. Tells the editor after the
  filling (onchanged), so that the list of the work units is read again.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import {
    planFill,
    planFillStop,
    type FillStepView,
    type FillSummaryView,
    type FillTask,
    type PlanView,
  } from "../../lib/api/evaluation_plan";

  let { plan, view, onchanged }: { plan: number; view: PlanView | null; onchanged: () => void } = $props();

  /** The most steps kept on the page: the newest. */
  const SHOWN = 200;

  let running: FillTask | null = $state(null);
  let stopping = $state(false);
  let last: FillStepView | null = $state(null);
  let steps: FillStepView[] = $state([]);
  let summary: FillSummaryView | null = $state(null);
  let problem = $state("");

  let agent = $derived(view?.agents.find((one) => one.id === view?.planningAgent)?.name ?? null);

  async function fill(task: FillTask) {
    problem = "";
    summary = null;
    steps = [];
    last = null;
    running = task;
    stopping = false;

    try {
      summary = await planFill(task, plan, (step) => {
        last = step;

        if (!step.filled || step.notes.length > 0) {
          steps = [step, ...steps].slice(0, SHOWN);
        }
      });
    } catch (error) {
      problem = sentence(error);
    } finally {
      running = null;
      onchanged();
    }
  }

  async function stop() {
    stopping = true;

    try {
      await planFillStop();
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
      <button type="button" disabled={running !== null} onclick={() => fill("parameters")}>
        {running === "parameters" ? "Filling the parameters…" : "Fill parameters"}
      </button>
      <button type="button" disabled={running !== null} onclick={() => fill("dependencies")}>
        {running === "dependencies" ? "Filling the dependencies…" : "Fill dependencies"}
      </button>
      {#if running !== null}
        <button type="button" class="ghost" disabled={stopping} onclick={stop}>{stopping ? "Stopping…" : "Stop"}</button>
      {/if}
    </p>
  {/if}

  {#if last}
    <p>
      {last.number} of {last.total}: {last.label}
      <progress max={last.total} value={last.number}></progress>
    </p>
  {/if}
  {#if summary}
    <p class={summary.failed > 0 ? "problem" : "done"}>
      {summary.filled} filled, {summary.failed} not filled{summary.stopped ? ", stopped" : ""}.
    </p>
  {/if}
  {#if problem}<p class="problem">{problem}</p>{/if}

  {#each steps as step (step.number)}
    <div class="step">
      <span class="badge" class:danger={!step.filled} class:warning={step.filled}>{step.label}</span>
      {step.notes.join("; ")}
    </div>
  {/each}
</section>

<style>
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
