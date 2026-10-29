<!--
  The editor of a work unit, opened from the editor of its plan (its
  commands work_unit_get, work_unit_save and work_unit_add_proof in
  src/panels/views/evaluation_plan/work_units.rs): "← the plan" (onback),
  "Previous / Next" in the order of the plan (onopen); its label and its
  standard whole, the requirement read only, the work units it depends on
  (chosen ones, and others found by a filter of the labels), those that
  depend on it (read only), the documents of the product it needs, the
  agent that will work on it (made in the panel Agents), the guidance, the proofs of the laboratory (with one or more it is a
  functional test, without any a test of the documentation). Every write
  locks the buttons, reads the work unit again and shows its time.
-->
<script lang="ts">
  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
  import { sentence } from "../../lib/api/errors";
  import {
    planGet,
    workUnitAddProof,
    workUnitGet,
    workUnitList,
    workUnitSave,
    type PlanView,
    type WorkUnitRowView,
    type WorkUnitView,
  } from "../../lib/api/evaluation_plan";

  let {
    plan,
    unit,
    onopen,
    onback,
  }: { plan: number; unit: number; onopen: (unit: number) => void; onback: () => void } = $props();

  let planView = $state<PlanView | null>(null);
  let units: WorkUnitRowView[] = $state([]);
  let view = $state<WorkUnitView | null>(null);
  let dependencies: number[] = $state([]);
  let chosenDocuments: number[] = $state([]);
  let guidance = $state("");
  let agent: number | null = $state(null);
  let proofName = $state("");
  let filter = $state("");
  let problem = $state("");
  let busy = $state(false);
  let savedAt: Date | null = $state(null);

  const position = $derived(units.findIndex((other) => other.id === unit));
  const row = $derived(units[position]);
  const dependents = $derived(units.filter((other) => other.dependencies.includes(unit)));

  // The work units already chosen, and, once a filter is typed, the other
  // work units of the plan whose label holds it: a plan has hundreds of
  // work units, too many to show at once.
  const candidates = $derived(
    units.filter(
      (other) =>
        other.id !== unit &&
        (dependencies.includes(other.id) ||
          (filter.trim() !== "" && other.label.toLowerCase().includes(filter.trim().toLowerCase()))),
    ),
  );

  async function load(id: number) {
    problem = "";

    try {
      planView = await planGet(plan);
      units = await workUnitList(plan);
      view = await workUnitGet(id);
      dependencies = [...view.dependencies];
      chosenDocuments = [...view.documents];
      guidance = view.guidance;
      agent = view.aiAgent;
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    filter = "";
    savedAt = null;
    void load(unit);
  });

  function toggled(list: number[], id: number, on: boolean): number[] {
    return on ? [...list, id] : list.filter((item) => item !== id);
  }

  /** Runs one write: the buttons locked while it runs, the work unit read
   * again from the program and the time of the write shown when it
   * succeeded. */
  async function write(work: () => Promise<unknown>) {
    problem = "";
    busy = true;

    try {
      await work();
      await load(unit);
      savedAt = new Date();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  function save(event: SubmitEvent) {
    event.preventDefault();
    void write(() =>
      workUnitSave({
        plan,
        unit,
        dependencies,
        documents: chosenDocuments,
        proofs: view?.proofs.map((proof) => proof.id) ?? [],
        aiAgent: agent,
        guidance,
      }),
    );
  }

  function addProof(event: SubmitEvent) {
    event.preventDefault();
    void write(async () => {
      await workUnitAddProof(plan, unit, proofName);
      proofName = "";
    });
  }
</script>

<div class="unit-head">
  <button type="button" class="link" onclick={onback}>← {planView?.name ?? "the plan"}</button>
  <div class="nav">
    <button type="button" disabled={busy || position <= 0} onclick={() => onopen(units[position - 1]?.id ?? unit)}>← Previous</button>
    <span class="note">{position + 1} of {units.length}</span>
    <button type="button" disabled={busy || position < 0 || position >= units.length - 1} onclick={() => onopen(units[position + 1]?.id ?? unit)}>Next →</button>
  </div>
</div>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if view}
  <section class="card">
    <h2>{view.label}</h2>
    <p class="standard">{row?.standard ?? ""}</p>
    {#if view.crossReference}
      <p class="problem">This work unit closes a cycle of dependencies (a cross reference): the plan is blocked until it is broken.</p>
    {/if}
    <h3>Requirement</h3>
    <p class="requirement">{view.requirement}</p>
  </section>

  <form onsubmit={save}>
    <section class="card">
      <h3>Depends on ({dependencies.length})</h3>
      <input class="filter" bind:value={filter} placeholder="Filter by the label: ADV_ARC" />
      <div class="choices-list">
        {#each candidates as other (other.id)}
          <label class="check">
            <input
              type="checkbox"
              checked={dependencies.includes(other.id)}
              onchange={(event) => (dependencies = toggled(dependencies, other.id, event.currentTarget.checked))}
            />
            {other.label}
          </label>
        {:else}
          <p class="note">{filter.trim() === "" ? "Type a part of a label to find a work unit." : "No other work unit fits."}</p>
        {/each}
      </div>

      <h3>Depended on by ({dependents.length})</h3>
      {#if dependents.length === 0}
        <p class="note">No work unit depends on this one.</p>
      {:else}
        <p>{dependents.map((other) => other.label).join(", ")}</p>
      {/if}
    </section>

    <section class="card">
      <h3>Documents of the product it needs</h3>
      {#each planView?.documents ?? [] as document (document.id)}
        <label class="check">
          <input
            type="checkbox"
            checked={chosenDocuments.includes(document.id)}
            onchange={(event) => (chosenDocuments = toggled(chosenDocuments, document.id, event.currentTarget.checked))}
          />
          {document.name}
        </label>
      {:else}
        <p class="note">The plan has no document yet: add one in the data of the plan.</p>
      {/each}

      <h3>Agent</h3>
      {#if (view?.agents ?? []).length === 0}
        <p class="note">No agent yet: make one in the panel Agents.</p>
      {:else}
        <label>
          The agent that will work on this unit (its knowledge goes with the questions of the evaluation)
          <select bind:value={agent}>
            <option value={null}>— none —</option>
            {#each view?.agents ?? [] as one (one.id)}
              <option value={one.id}>{one.name}</option>
            {/each}
          </select>
        </label>
      {/if}

      <h3>Guidance</h3>
      <textarea rows="6" bind:value={guidance}></textarea>

      <div class="save-row">
        <button type="submit" disabled={busy}>Save the work unit</button>
        <SaveStatus {busy} {savedAt} />
      </div>
    </section>
  </form>

  <section class="card">
    <h3>Proofs of the laboratory ({view.proofs.length === 0 ? "a test of the documentation" : "a functional test"})</h3>
    {#each view.proofs as proof (proof.id)}
      <p class="proof">{proof.name}</p>
    {/each}
    <form class="inline" onsubmit={addProof}>
      <input bind:value={proofName} placeholder="Results of the scan of the ports" required />
      <button type="submit" disabled={busy}>Add a proof</button>
    </form>
  </section>
{/if}

<style>
  .unit-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .nav button,
  .nav span {
    white-space: nowrap;
  }

  .nav,
  .inline,
  .save-row {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.75rem;
  }

  .standard {
    color: var(--muted, #9aa3b2);
    margin-top: -0.5rem;
  }

  .requirement {
    white-space: pre-wrap;
  }

  .check {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
    font-weight: normal;
  }

  .check input {
    width: auto;
  }

  .choices-list {
    max-height: 16rem;
    overflow-y: auto;
  }

  .filter {
    max-width: 22rem;
    margin-bottom: 0.5rem;
  }

  .proof {
    margin: 0.2rem 0;
  }

  textarea {
    width: 100%;
  }
</style>
