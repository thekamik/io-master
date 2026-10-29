<!--
  The form of a work unit of a plan (its commands work_unit_get,
  work_unit_save and work_unit_add_proof in
  src/panels/views/evaluation_plan/work_units.rs): its requirement, the
  work units it depends on, the documents of the product it needs, the
  proofs of the laboratory (with one or more it is a functional test,
  without any a test of the documentation) and its guidance. Tells the
  panel after every change (onchanged).
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import {
    workUnitAddProof,
    workUnitGet,
    workUnitSave,
    type DocumentView,
    type WorkUnitRowView,
    type WorkUnitView,
  } from "../../lib/api/evaluation_plan";

  let {
    plan,
    unit,
    units,
    documents,
    onchanged,
    onclose,
  }: {
    plan: number;
    unit: number;
    units: WorkUnitRowView[];
    documents: DocumentView[];
    onchanged: () => void;
    onclose: () => void;
  } = $props();

  let view: WorkUnitView | null = $state(null);
  let dependencies: number[] = $state([]);
  let chosenDocuments: number[] = $state([]);
  let guidance = $state("");
  let proofName = $state("");
  let filter = $state("");

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
  let problem = $state("");

  async function load(id: number) {
    problem = "";

    try {
      view = await workUnitGet(id);
      dependencies = [...view.dependencies];
      chosenDocuments = [...view.documents];
      guidance = view.guidance;
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    void load(unit);
  });

  function toggled(list: number[], id: number, on: boolean): number[] {
    return on ? [...list, id] : list.filter((item) => item !== id);
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    problem = "";

    try {
      await workUnitSave({
        plan,
        unit,
        dependencies,
        documents: chosenDocuments,
        proofs: view?.proofs.map((proof) => proof.id) ?? [],
        aiAgent: view?.aiAgent ?? null,
        guidance,
      });
      onchanged();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function addProof(event: SubmitEvent) {
    event.preventDefault();
    problem = "";

    try {
      await workUnitAddProof(plan, unit, proofName);
      proofName = "";
      await load(unit);
      onchanged();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="card work-unit-form">
  {#if view}
    <h3>{view.label} <button type="button" class="link" onclick={onclose}>close</button></h3>
    {#if problem}<p class="problem">{problem}</p>{/if}
    <p class="requirement">{view.requirement}</p>

    <form onsubmit={save}>
      <fieldset>
        <legend>Depends on ({dependencies.length})</legend>
        <input class="filter" bind:value={filter} placeholder="Filter by the label: APE_INT" />
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
      </fieldset>

      <fieldset>
        <legend>Documents of the product it needs</legend>
        {#each documents as document (document.id)}
          <label class="check">
            <input
              type="checkbox"
              checked={chosenDocuments.includes(document.id)}
              onchange={(event) => (chosenDocuments = toggled(chosenDocuments, document.id, event.currentTarget.checked))}
            />
            {document.name}
          </label>
        {:else}
          <p class="note">The plan has no document yet.</p>
        {/each}
      </fieldset>

      <label>Guidance <textarea rows="4" bind:value={guidance}></textarea></label>
      <button type="submit">Save the work unit</button>
    </form>

    <fieldset>
      <legend>Proofs of the laboratory ({view.proofs.length === 0 ? "a test of the documentation" : "a functional test"})</legend>
      {#each view.proofs as proof (proof.id)}
        <p class="proof">{proof.name}</p>
      {/each}
      <form class="inline" onsubmit={addProof}>
        <input bind:value={proofName} placeholder="Results of the scan of the ports" required />
        <button type="submit">Add a proof</button>
      </form>
    </fieldset>
  {:else if problem}
    <p class="problem">{problem}</p>
  {/if}
</div>

<style>
  .work-unit-form {
    margin: 0.75rem 0;
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

  fieldset {
    border: 1px solid var(--border, #333);
    border-radius: 0.5rem;
    padding: 0.5rem 0.75rem;
    margin-bottom: 0.75rem;
  }

  .inline {
    display: flex;
    flex-direction: row;
    gap: 0.5rem;
  }

  .proof {
    margin: 0.2rem 0;
  }

  .choices-list {
    max-height: 14rem;
    overflow-y: auto;
  }

  .filter {
    margin-bottom: 0.5rem;
  }
</style>
