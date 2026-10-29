<!--
  The work units of a plan (their commands in
  src/panels/views/evaluation_plan/work_units.rs): the list of the plan
  (a work unit chosen opens its form, WorkUnitForm), and the controls of a
  standard of the plan it does not hold yet, each added alone or all at
  once. Tells the panel after every change (onchanged).
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import {
    planGet,
    workUnitAdd,
    workUnitAddAll,
    workUnitControls,
    workUnitList,
    workUnitRemove,
    type ControlView,
    type PlanView,
    type WorkUnitRowView,
  } from "../../lib/api/evaluation_plan";
  import WorkUnitForm from "./WorkUnitForm.svelte";

  let { plan, version, onchanged }: { plan: number; version: number; onchanged: () => void } = $props();

  let view = $state<PlanView | null>(null);
  let units: WorkUnitRowView[] = $state([]);
  let standard: number | null = $state(null);
  let controls: ControlView[] = $state([]);
  let open: number | null = $state(null);
  let problem = $state("");
  let busy = $state(false);

  const chosenStandards = $derived(view ? view.standards.filter((item) => item.chosen) : []);

  async function load(id: number) {
    problem = "";

    try {
      view = await planGet(id);
      units = await workUnitList(id);
      controls = standard === null ? [] : await workUnitControls(id, standard);
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    void version;
    void load(plan);
  });

  async function chooseStandard(id: number | null) {
    standard = id;
    controls = id === null ? [] : await workUnitControls(plan, id).catch(() => []);
  }

  async function act(work: () => Promise<unknown>) {
    problem = "";
    busy = true;

    try {
      await work();
      onchanged();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }
</script>

<section class="card">
  <h2>Work units</h2>
  {#if problem}<p class="problem">{problem}</p>{/if}

  {#if open !== null && view}
    <WorkUnitForm {plan} unit={open} {units} documents={view.documents} onchanged={onchanged} onclose={() => (open = null)} />
  {/if}

  <table class="data">
    <colgroup>
      <col />
      <col style="width: 9.5rem" />
      <col style="width: 6rem" />
      <col style="width: 7rem" />
    </colgroup>
    <thead><tr><th>Work unit</th><th>Kind</th><th>Depends on</th><th></th></tr></thead>
    <tbody>
      {#each units as unit (unit.id)}
        <tr class:active={unit.id === open}>
          <td>
            <button type="button" class="link" onclick={() => (open = unit.id)}>{unit.label}</button>
            {#if unit.crossReference}<span class="problem-text"> cross reference</span>{/if}
            <div class="standard">{unit.standard}</div>
          </td>
          <td>{unit.functional ? "Functional" : "Documentation"}</td>
          <td>{unit.dependencies.length}</td>
          <td>
            <button type="button" disabled={busy} onclick={() => act(() => workUnitRemove(plan, unit.id))}>Remove</button>
          </td>
        </tr>
      {:else}
        <tr><td colspan="4" class="note">No work unit yet: add them from a standard of the plan.</td></tr>
      {/each}
    </tbody>
  </table>

  <h3>Add from a standard</h3>
  {#if chosenStandards.length === 0}
    <p class="note">Give the plan a standard first (above).</p>
  {:else}
    <div class="inline">
      <select onchange={(event) => chooseStandard(Number(event.currentTarget.value) || null)}>
        <option value="">Choose a standard…</option>
        {#each chosenStandards as item (item.id)}
          <option value={item.id} selected={item.id === standard}>{item.title}</option>
        {/each}
      </select>
      {#if standard !== null}
        <button type="button" disabled={busy} onclick={() => standard !== null && act(() => workUnitAddAll(plan, standard ?? 0))}>
          {busy ? "Adding…" : `Add all (${controls.length})`}
        </button>
      {/if}
    </div>

    {#if standard !== null}
      <table class="data">
        <tbody>
          {#each controls as control (control.oscalKey)}
            <tr>
              <td class="label">{control.label}</td>
              <td class="wrap">{control.requirement}</td>
              <td>
                <button type="button" disabled={busy} onclick={() => act(() => workUnitAdd(plan, standard ?? 0, control.oscalKey))}>
                  Add
                </button>
              </td>
            </tr>
          {:else}
            <tr><td class="note">The plan holds every control of this standard.</td></tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</section>

<style>
  table.data {
    width: 100%;
    table-layout: fixed;
  }

  table.data td {
    white-space: normal;
  }

  .inline {
    display: flex;
    flex-direction: row;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .label {
    width: 9rem;
  }

  /* The title of a standard is never cut: two standards may differ only at
     its end ("... for network devices" and "... for network devices,
     industrial"); it stands whole under the label of its work unit. */
  .standard {
    font-size: 0.8rem;
    color: var(--muted, #9aa3b2);
    white-space: normal;
  }

  .problem-text {
    color: #e5484d;
    font-size: 0.85rem;
  }

  tr.active td {
    background: var(--hover, rgba(127, 127, 127, 0.12));
  }
</style>
