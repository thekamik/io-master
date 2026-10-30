<!--
  The panel of the evaluation (its commands in src/panels/views/evaluation/;
  plan_list and agent_list for the choices): every evaluation and report,
  and a new evaluation, begun on a copy of a plan in planning with the
  identification ISO/IEC 17025 asks for and the agent of the work units
  without one of their own (user, 2026-10-30). An evaluation opened shows
  its editor (EvaluationEditor). Which screen is shown is presentation
  only: it stays in the page.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { agentList, type AgentSummaryView } from "../../lib/api/agents";
  import { sentence } from "../../lib/api/errors";
  import { evaluationCreate, evaluationList, type EvaluationSummaryView } from "../../lib/api/evaluation";
  import { planList, type PlanSummaryView } from "../../lib/api/evaluation_plan";
  import EvaluationEditor from "./EvaluationEditor.svelte";

  let evaluations: EvaluationSummaryView[] = $state([]);
  let plans: PlanSummaryView[] = $state([]);
  let agents: AgentSummaryView[] = $state([]);
  let open: number | null = $state(null);
  let plan: number | null = $state(null);
  let agent: number | null = $state(null);
  let product = $state("");
  let client = $state("");
  let evaluator = $state("");
  let problem = $state("");
  let busy = $state(false);

  /** The plans that may be copied: not blocked, with work units. */
  let ready = $derived(plans.filter((one) => !one.blocked && one.workUnits > 0));

  async function reload() {
    try {
      evaluations = await evaluationList();
      plans = await planList();
      agents = await agentList();
      plan = plan ?? ready[0]?.id ?? null;
      agent = agent ?? agents[0]?.id ?? null;
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();

    if (plan === null || agent === null) {
      return;
    }

    problem = "";
    busy = true;

    try {
      open = await evaluationCreate(plan, product, client, evaluator, agent);
      product = "";
      client = "";
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  function date(seconds: number | null): string {
    return seconds === null ? "" : new Date(seconds * 1000).toISOString().slice(0, 10);
  }

  onMount(reload);
</script>

<div class="page-head">
  <h1>Evaluation</h1>
  <p>A plan copied for one product: its documents and proofs given, its work units assessed by the AI.</p>
</div>

{#if open !== null}
  <EvaluationEditor
    evaluation={open}
    onback={() => {
      open = null;
      reload();
    }}
  />
{:else}
  {#if problem}<p class="problem">{problem}</p>{/if}

  <section class="card">
    <h2>Evaluations</h2>
    <table class="data">
      <thead>
        <tr><th>Product</th><th>Client</th><th>Plan</th><th>Begun</th><th>State</th><th></th></tr>
      </thead>
      <tbody>
        {#each evaluations as one (one.id)}
          <tr>
            <td class="wrap">{one.product}</td>
            <td class="wrap">{one.client}</td>
            <td class="wrap">{one.name}</td>
            <td>{date(one.started)}</td>
            <td>{one.state}</td>
            <td><button type="button" onclick={() => (open = one.id)}>Open</button></td>
          </tr>
        {:else}
          <tr><td colspan="6" class="note">No evaluation yet: begin the first one below.</td></tr>
        {/each}
      </tbody>
    </table>
  </section>

  <section class="card">
    <h2>A new evaluation</h2>
    {#if ready.length === 0}
      <p class="note">An evaluation is a copy of a plan: make one, with work units and without a cross reference, in the panel Evaluation plan.</p>
    {:else if agents.length === 0}
      <p class="note">The work units without an agent of their own are assessed by the agent of the evaluation: make one first in the panel Agents.</p>
    {:else}
      <form onsubmit={create}>
        <label>
          Plan
          <select bind:value={plan}>
            {#each ready as one (one.id)}
              <option value={one.id}>{one.name} · {one.workUnits} work units</option>
            {/each}
          </select>
        </label>
        <label>Product <input bind:value={product} required placeholder="Router R7, firmware 2.4.1" /></label>
        <label>Client <input bind:value={client} required placeholder="ACME" /></label>
        <label>Evaluator <input bind:value={evaluator} required /></label>
        <label>
          Agent of the evaluation
          <span class="note">It assesses every work unit without an agent of its own; it can be changed later.</span>
          <select bind:value={agent}>
            {#each agents as one (one.id)}
              <option value={one.id}>{one.name}</option>
            {/each}
          </select>
        </label>
        <button type="submit" disabled={busy}>Begin the evaluation</button>
      </form>
    {/if}
  </section>
{/if}

<style>
  table.data {
    width: 100%;
  }
</style>
