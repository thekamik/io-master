<!--
  The tab Plans of the panel of the plan (its commands plan_list and
  plan_create in src/panels/views/evaluation_plan/plans.rs): the plans in
  planning, each opened in its editor (onopen), and a new plan, opened at
  once.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import { planCreate, planList, type PlanSummaryView } from "../../lib/api/evaluation_plan";

  let { onopen }: { onopen: (plan: number) => void } = $props();

  let plans: PlanSummaryView[] = $state([]);
  let name = $state("");
  let description = $state("");
  let problem = $state("");
  let busy = $state(false);

  async function reload() {
    try {
      plans = await planList();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function createPlan(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      onopen(await planCreate(name, description));
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  onMount(reload);
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

<section class="card">
  <h2>Plans</h2>
  <table class="data">
    <thead><tr><th>Plan</th><th>Work units</th><th></th></tr></thead>
    <tbody>
      {#each plans as plan (plan.id)}
        <tr>
          <td class="wrap">
            {plan.name}
            {#if plan.blocked}<span class="problem-text"> blocked</span>{/if}
          </td>
          <td>{plan.workUnits}</td>
          <td><button type="button" onclick={() => onopen(plan.id)}>Open</button></td>
        </tr>
      {:else}
        <tr><td colspan="3" class="note">No plan yet: make the first one below.</td></tr>
      {/each}
    </tbody>
  </table>
</section>

<section class="card">
  <h2>A new plan</h2>
  <form onsubmit={createPlan}>
    <label>Name <input bind:value={name} required /></label>
    <label>Description <input bind:value={description} /></label>
    <button type="submit" disabled={busy}>Make the plan and open it</button>
  </form>
</section>

<style>
  table.data {
    width: 100%;
  }

  .problem-text {
    color: #e5484d;
    font-size: 0.85rem;
  }
</style>
