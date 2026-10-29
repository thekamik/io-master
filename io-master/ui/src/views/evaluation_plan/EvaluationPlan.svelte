<!--
  The panel of the plan (its commands in src/panels/views/evaluation_plan/):
  the standards of the user (an OSCAL catalog imported through the dialog
  of the system), the plans in planning and a new plan; the plan chosen is
  shown with its data (PlanForm) and its work units (WorkUnits). After a
  change one element tells the panel (onchanged), which counts `version`
  up, so that every element reads its data again.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import {
    planCreate,
    planList,
    standardImport,
    standardList,
    type PlanSummaryView,
    type StandardView,
  } from "../../lib/api/evaluation_plan";
  import PlanForm from "./PlanForm.svelte";
  import WorkUnits from "./WorkUnits.svelte";

  let standards: StandardView[] = $state([]);
  let plans: PlanSummaryView[] = $state([]);
  let chosen: number | null = $state(null);
  let version = $state(0);
  let name = $state("");
  let description = $state("");
  let problem = $state("");
  let busy = $state(false);

  async function reload() {
    try {
      standards = await standardList();
      plans = await planList();
    } catch (error) {
      problem = sentence(error);
    }
  }

  function changed() {
    version += 1;
    void reload();
  }

  async function importStandard() {
    problem = "";
    busy = true;

    try {
      await standardImport();
      changed();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  async function createPlan(event: SubmitEvent) {
    event.preventDefault();
    problem = "";

    try {
      chosen = await planCreate(name, description);
      name = description = "";
      changed();
    } catch (error) {
      problem = sentence(error);
    }
  }

  onMount(reload);
</script>

<div class="page-head">
  <h1>Evaluation plan</h1>
  <p>The standards, the plans in planning and their work units.</p>
</div>

{#if problem}<p class="problem">{problem}</p>{/if}

<div class="plan-columns">
  <div class="plan-side">
    <section class="card">
      <h2>Standards</h2>
      {#each standards as standard (standard.id)}
        <p class="standard">{standard.title} <span class="note">{standard.version}</span></p>
      {:else}
        <p class="note">No standard yet: import an OSCAL catalog.</p>
      {/each}
      <button type="button" onclick={importStandard} disabled={busy}>
        {busy ? "Importing…" : "Import a standard"}
      </button>
    </section>

    <section class="card">
      <h2>Plans</h2>
      {#each plans as plan (plan.id)}
        <button type="button" class="nav-item plan-item" class:active={plan.id === chosen} onclick={() => (chosen = plan.id)}>
          {plan.name}
          <span class="note">{plan.workUnits} work units{plan.blocked ? ", blocked" : ""}</span>
        </button>
      {:else}
        <p class="note">No plan yet.</p>
      {/each}

      <form onsubmit={createPlan}>
        <label>Name <input bind:value={name} required /></label>
        <label>Description <input bind:value={description} /></label>
        <button type="submit">New plan</button>
      </form>
    </section>
  </div>

  <div class="plan-main">
    {#if chosen !== null}
      <PlanForm plan={chosen} {version} onchanged={changed} />
      <WorkUnits plan={chosen} {version} onchanged={changed} />
    {:else}
      <p class="note">Choose a plan, or make a new one.</p>
    {/if}
  </div>
</div>

<style>
  .plan-columns {
    display: grid;
    grid-template-columns: 20rem 1fr;
    gap: 1rem;
    align-items: start;
  }

  .plan-side,
  .plan-main {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    min-width: 0;
  }

  .plan-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    text-align: left;
  }

  .standard {
    margin: 0.25rem 0;
  }
</style>
