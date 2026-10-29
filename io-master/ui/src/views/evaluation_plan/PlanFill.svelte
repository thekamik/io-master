<!--
  "Fill with AI" in the editor of a plan (its commands plan_fill and
  plan_fill_stop in src/panels/views/evaluation_plan/fill.rs): the agent of
  the planner, chosen in the data of the plan, fills every work unit of the
  plan in its order, over what was there. Each step is shown as it ends;
  "Stop" ends the filling after its present work unit. A dependency that
  would close a cycle is left out by the program. Tells the editor after the
  filling (onchanged), so that the list of the work units is read again.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import { planFill, planFillStop, type FillStepView, type FillSummaryView, type PlanView } from "../../lib/api/evaluation_plan";

  let { plan, view, onchanged }: { plan: number; view: PlanView | null; onchanged: () => void } = $props();

  /** The most steps kept on the page: the newest. */
  const SHOWN = 200;

  let running = $state(false);
  let stopping = $state(false);
  let last: FillStepView | null = $state(null);
  let steps: FillStepView[] = $state([]);
  let summary: FillSummaryView | null = $state(null);
  let problem = $state("");

  let agent = $derived(view?.agents.find((one) => one.id === view?.planningAgent)?.name ?? null);

  async function fill() {
    problem = "";
    summary = null;
    steps = [];
    last = null;
    running = true;
    stopping = false;

    try {
      summary = await planFill(plan, (step) => {
        last = step;

        if (!step.filled || step.notes.length > 0) {
          steps = [step, ...steps].slice(0, SHOWN);
        }
      });
    } catch (error) {
      problem = sentence(error);
    } finally {
      running = false;
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
      The agent <strong>{agent}</strong> proposes every parameter of each work unit, in the order of the plan, over what
      is there. Each question is kept in the journal of the agent.
    </p>
    <p>
      <button type="button" disabled={running} onclick={fill}>{running ? "Filling…" : "Fill with AI"}</button>
      {#if running}
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
  progress {
    width: 100%;
  }

  .step {
    margin: 0.3rem 0;
    font-size: 0.9rem;
  }
</style>
