<!--
  The work units of a plan, in its editor (their commands in
  src/panels/views/evaluation_plan/work_units.rs): a filter by the label, the
  list by pages of PAGE_SIZE rows (a plan has hundreds of work units), a work
  unit opened in its editor (onopen) or removed; and, folded, the controls
  of a standard of the plan it does not hold yet, each added alone or all at
  once. Tells the editor after every change (onchanged).
-->
<script lang="ts">
  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
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

  /** The rows of one page of the list. */
  const PAGE_SIZE = 50;

  let {
    plan,
    version,
    page = $bindable(),
    filter = $bindable(),
    onchanged,
    onopen,
  }: {
    plan: number;
    version: number;
    page: number;
    filter: string;
    onchanged: () => void;
    onopen: (unit: number) => void;
  } = $props();

  let view = $state<PlanView | null>(null);
  let units: WorkUnitRowView[] = $state([]);
  let standard: number | null = $state(null);
  let controls: ControlView[] = $state([]);
  let problem = $state("");
  let busy = $state(false);
  let savedAt: Date | null = $state(null);

  const chosenStandards = $derived(view ? view.standards.filter((item) => item.chosen) : []);
  const found = $derived(units.filter((unit) => unit.label.toLowerCase().includes(filter.trim().toLowerCase())));
  const pages = $derived(Math.max(1, Math.ceil(found.length / PAGE_SIZE)));
  const shown = $derived(found.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE));

  async function load(id: number) {
    problem = "";

    try {
      view = await planGet(id);
      units = await workUnitList(id);
      controls = standard === null ? [] : await workUnitControls(id, standard);
      page = Math.min(page, Math.max(0, Math.ceil(units.length / PAGE_SIZE) - 1));
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
      savedAt = new Date();
      onchanged();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }
</script>

<section class="card">
  <h2>Work units ({units.length})</h2>
  {#if problem}<p class="problem">{problem}</p>{/if}

  <div class="list-tools">
    <input bind:value={filter} oninput={() => (page = 0)} placeholder="Filter by the label: ADV_ARC" />
    <SaveStatus {busy} {savedAt} />
  </div>

  <table class="data">
    <colgroup>
      <col />
      <col style="width: 9.5rem" />
      <col style="width: 6rem" />
      <col style="width: 7rem" />
    </colgroup>
    <thead><tr><th>Work unit</th><th>Kind</th><th>Depends on</th><th></th></tr></thead>
    <tbody>
      {#each shown as unit (unit.id)}
        <tr>
          <td>
            <button type="button" class="link" onclick={() => onopen(unit.id)}>{unit.label}</button>
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
        <tr>
          <td colspan="4" class="note">
            {units.length === 0 ? "No work unit yet: add them from a standard of the plan (below)." : "No work unit fits the filter."}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  {#if pages > 1}
    <div class="pager">
      <button type="button" disabled={page === 0} onclick={() => (page -= 1)}>← Previous</button>
      <span class="note">Page {page + 1} of {pages} ({found.length} work units)</span>
      <button type="button" disabled={page >= pages - 1} onclick={() => (page += 1)}>Next →</button>
    </div>
  {/if}

  <details>
    <summary>Add from a standard</summary>
    {#if chosenStandards.length === 0}
      <p class="note">Give the plan a standard first (the data of the plan, above).</p>
    {:else}
      <div class="inline">
        <select onchange={(event) => chooseStandard(Number(event.currentTarget.value) || null)}>
          <option value="">Choose a standard…</option>
          {#each chosenStandards as item (item.id)}
            <option value={item.id} selected={item.id === standard}>{item.title}</option>
          {/each}
        </select>
        {#if standard !== null}
          <button type="button" disabled={busy} onclick={() => act(() => workUnitAddAll(plan, standard ?? 0))}>
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
                <td>{control.requirement}</td>
                <td class="action">
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
  </details>
</section>

<style>
  table.data {
    width: 100%;
    table-layout: fixed;
  }

  table.data td {
    white-space: normal;
  }

  .list-tools,
  .inline,
  .pager {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.75rem;
    margin: 0.5rem 0 0.75rem;
  }

  .list-tools input {
    max-width: 22rem;
  }

  .label {
    width: 9rem;
  }

  .action {
    width: 6rem;
  }

  details {
    margin-top: 1rem;
  }

  summary {
    cursor: pointer;
    font-weight: 600;
    margin-bottom: 0.75rem;
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
</style>
