<!--
  The element "list of evaluations" of the view Evaluation lab (its commands
  evaluation_list_get, evaluation_list_delete and package_import_import in
  src/views/evaluation_lab/evaluation_lab.rs): every evaluation with its
  progress, opened (onopen) or deleted; and the import of a work package,
  with the product it is for (the file is chosen in a dialog of the system).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as evaluationLab from "../../lib/api/evaluation_lab";

  let { onopen }: { onopen: (uuid: string) => void } = $props();

  let evaluations: evaluationLab.EvaluationSummaryView[] = $state([]);
  let form: evaluationLab.ImportForm = $state({ product: "", product_version: "" });
  let importing = $state(false);
  let notice: { done: boolean; text: string } | null = $state(null);

  onMount(async () => {
    try {
      evaluations = await evaluationLab.evaluationListGet();
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  });

  async function importPackage(event: SubmitEvent) {
    event.preventDefault();

    try {
      const listed = await evaluationLab.packageImportImport({ ...form });

      if (listed) {
        evaluations = listed;
        importing = false;
        form = { product: "", product_version: "" };
        notice = { done: true, text: "The work package was imported." };
      }
    } catch (error) {
      notice = { done: false, text: `The work package was not imported: ${sentence(error)}` };
    }
  }

  async function remove(uuid: string) {
    try {
      evaluations = await evaluationLab.evaluationListDelete(uuid);
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }
</script>

{#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}

{#if importing}
  <form class="card block-form" onsubmit={importPackage}>
    <h3>Import a work package</h3>
    <p class="note">The file the planner exported from the Evaluation plan. Its tasks and schemes are copied as they are; the catalogs attached are stored when you have them not yet.</p>
    <label>Product (TOE) <input bind:value={form.product} placeholder="The product evaluated" /></label>
    <label>Version <input bind:value={form.product_version} /></label>
    <div class="buttons">
      <button type="submit">Choose the file…</button>
      <button type="button" onclick={() => (importing = false)}>Cancel</button>
    </div>
  </form>
{:else}
  <div class="buttons lab-actions"><button type="button" onclick={() => (importing = true)}>Import a work package…</button></div>
{/if}

{#if evaluations.length === 0}
  <div class="card">
    <h3>No evaluation yet</h3>
    <p class="note">An evaluation carries out a work package of the Evaluation plan for one product: you deliver the evidence of every input, accept the fragments that matter, decide the results and the verdict of every requirement with the help of its agent, and export the report.</p>
  </div>
{:else}
  <table class="data">
    <thead><tr><th>Work package</th><th>Project</th><th>Product</th><th>Done</th><th></th></tr></thead>
    <tbody>
      {#each evaluations as item (item.uuid)}
        <tr>
          <td>{item.package_name}</td>
          <td>{item.project_title}</td>
          <td>{item.product}</td>
          <td>{item.done} of {item.tasks}</td>
          <td class="actions">
            <button type="button" class="small" onclick={() => onopen(item.uuid)}>Open</button>
            <button type="button" class="small danger" onclick={() => remove(item.uuid)}>Delete</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
