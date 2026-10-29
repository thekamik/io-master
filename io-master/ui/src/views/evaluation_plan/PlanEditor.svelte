<!--
  The editor of a plan, opened from the tab Plans: "← Plans", the data of the
  plan (PlanForm), "Fill with AI" (PlanFill) and its work units
  (PlanWorkUnits), a work unit opened in
  its own editor (onopen). After a change one element counts `version` up,
  so that both read their data again. The page and the filter of the list
  belong to the panel (bindable), so that they outlive this screen.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import { planGet, type PlanView } from "../../lib/api/evaluation_plan";
  import PlanFill from "./PlanFill.svelte";
  import PlanForm from "./PlanForm.svelte";
  import PlanWorkUnits from "./PlanWorkUnits.svelte";

  let {
    plan,
    page = $bindable(),
    filter = $bindable(),
    onopen,
    onback,
  }: {
    plan: number;
    page: number;
    filter: string;
    onopen: (unit: number) => void;
    onback: () => void;
  } = $props();

  let view = $state<PlanView | null>(null);
  let version = $state(0);
  let problem = $state("");

  async function load(id: number) {
    try {
      view = await planGet(id);
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    void version;
    void load(plan);
  });

  function changed() {
    version += 1;
  }
</script>

<div class="plan-head">
  <button type="button" class="link" onclick={onback}>← Plans</button>
  <h2>{view?.name ?? "…"}</h2>
</div>

{#if problem}<p class="problem">{problem}</p>{/if}

<PlanForm {plan} {version} onchanged={changed} />
<PlanFill {plan} {view} onchanged={changed} />
<PlanWorkUnits {plan} {version} bind:page bind:filter onchanged={changed} {onopen} />

<style>
  .plan-head {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.25rem;
    margin-bottom: 1rem;
  }

  .plan-head h2 {
    margin: 0;
  }
</style>
