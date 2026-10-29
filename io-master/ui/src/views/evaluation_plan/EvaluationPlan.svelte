<!--
  The panel of the plan (its commands in src/panels/views/evaluation_plan/),
  following the model of the data, where the work units belong to their
  plan (user, 2026-10-29). Two tabs, the two ways in: Standards (the import)
  and Plans (a new plan, a plan opened). An open plan shows its editor
  (PlanEditor), and a work unit opened there its own (WorkUnitEditor). Which
  screen is shown is presentation only: it stays in the page. The page and
  the filter of the list of the work units are kept here, so that "back"
  from a work unit finds the list as it was left.
-->
<script lang="ts">
  import PlanEditor from "./PlanEditor.svelte";
  import PlansTab from "./PlansTab.svelte";
  import StandardsTab from "./StandardsTab.svelte";
  import WorkUnitEditor from "./WorkUnitEditor.svelte";

  let tab: "standards" | "plans" = $state("plans");
  let openPlan: number | null = $state(null);
  let openUnit: number | null = $state(null);
  let page = $state(0);
  let filter = $state("");

  function openPlanEditor(id: number) {
    openPlan = id;
    openUnit = null;
    page = 0;
    filter = "";
  }
</script>

<div class="page-head">
  <h1>Evaluation plan</h1>
  <p>The standards, the plans in planning and their work units.</p>
</div>

{#if openPlan !== null && openUnit !== null}
  <WorkUnitEditor plan={openPlan} unit={openUnit} onopen={(id) => (openUnit = id)} onback={() => (openUnit = null)} />
{:else if openPlan !== null}
  <PlanEditor
    plan={openPlan}
    bind:page
    bind:filter
    onopen={(id) => (openUnit = id)}
    onback={() => (openPlan = null)}
  />
{:else}
  <div class="tabs">
    <button type="button" class:active={tab === "standards"} onclick={() => (tab = "standards")}>Standards</button>
    <button type="button" class:active={tab === "plans"} onclick={() => (tab = "plans")}>Plans</button>
  </div>

  {#if tab === "standards"}
    <StandardsTab />
  {:else}
    <PlansTab onopen={openPlanEditor} />
  {/if}
{/if}
