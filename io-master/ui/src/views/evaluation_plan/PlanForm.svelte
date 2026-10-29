<!--
  The data of a plan (its commands plan_get, plan_save and
  plan_add_document in src/panels/views/evaluation_plan/plans.rs): its name,
  description, standards, places of the documents of the product, and
  whether its standards also know the verdict Inconclusive (Pass and Fail
  always). Tells the panel after every change (onchanged).
-->
<script lang="ts">
  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
  import { sentence } from "../../lib/api/errors";
  import { planAddDocument, planGet, planSave, type PlanView, type VerdictName } from "../../lib/api/evaluation_plan";

  let { plan, version, onchanged }: { plan: number; version: number; onchanged: () => void } = $props();

  let view: PlanView | null = $state(null);
  let name = $state("");
  let description = $state("");
  let standards: number[] = $state([]);
  let inconclusive = $state(false);
  let documentName = $state("");
  let problem = $state("");
  let busy = $state(false);
  let savedAt: Date | null = $state(null);

  async function load(id: number) {
    problem = "";

    try {
      view = await planGet(id);
      name = view.name;
      description = view.description;
      standards = view.standards.filter((standard) => standard.chosen).map((standard) => standard.id);
      inconclusive = view.verdicts.includes("inconclusive");
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    void version;
    void load(plan);
  });

  function toggle(id: number, on: boolean) {
    standards = on ? [...standards, id] : standards.filter((chosen) => chosen !== id);
  }

  /** Runs one write: the buttons locked while it runs, the time of the
   * write shown when it succeeded, the panel told. */
  async function write(work: () => Promise<unknown>) {
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

  function save(event: SubmitEvent) {
    event.preventDefault();

    const verdicts: VerdictName[] = inconclusive ? ["pass", "fail", "inconclusive"] : ["pass", "fail"];
    const documents = view?.documents.map((document) => document.id) ?? [];

    void write(() => planSave({ id: plan, name, description, standards, documents, verdicts }));
  }

  function addDocument(event: SubmitEvent) {
    event.preventDefault();
    void write(async () => {
      await planAddDocument(plan, documentName);
      documentName = "";
    });
  }
</script>

<section class="card">
  <h2>The plan</h2>
  {#if problem}<p class="problem">{problem}</p>{/if}
  {#if view}
    {#if view.blocked}
      <p class="problem">The plan is blocked: some work units depend on themselves (a cross reference).</p>
    {/if}
    <form onsubmit={save}>
      <label>Name <input bind:value={name} required /></label>
      <label>Description <input bind:value={description} /></label>

      <fieldset>
        <legend>Standards</legend>
        {#each view.standards as standard (standard.id)}
          <label class="check">
            <input
              type="checkbox"
              checked={standards.includes(standard.id)}
              onchange={(event) => toggle(standard.id, event.currentTarget.checked)}
            />
            {standard.title}
          </label>
        {:else}
          <p class="note">No standard imported yet.</p>
        {/each}
      </fieldset>

      <label class="check">
        <input type="checkbox" bind:checked={inconclusive} />
        The verdict Inconclusive too (ETSI EN 303 645); Pass and Fail always
      </label>

      <button type="submit" disabled={busy}>Save the plan</button>
      <SaveStatus {busy} {savedAt} />
    </form>

    <h3>Documents of the product</h3>
    {#each view.documents as document (document.id)}
      <p class="document">{document.name}</p>
    {:else}
      <p class="note">No document yet (the ST, the IXIT).</p>
    {/each}
    <form class="inline" onsubmit={addDocument}>
      <input bind:value={documentName} placeholder="ST" required />
      <button type="submit" disabled={busy}>Add a document</button>
    </form>
  {/if}
</section>

<style>
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
  }

  .inline {
    display: flex;
    flex-direction: row;
    gap: 0.5rem;
  }

  .document {
    margin: 0.2rem 0;
  }
</style>
