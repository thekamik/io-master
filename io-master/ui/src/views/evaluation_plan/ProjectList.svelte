<!--
  The element "list of master projects" of the view Evaluation plan (its
  commands project_list_get and project_list_delete in
  src/views/evaluation_plan/evaluation_plan.rs). "Open" and "New master
  project…" only tell the view (onopen, onnew). The list is read again when
  the view shows it.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";

  let { onopen, onnew }: { onopen: (uuid: string) => void; onnew: () => void } = $props();

  let projects: evaluationPlan.ProjectSummaryView[] = $state([]);
  let confirmDelete: string | null = $state(null);
  let problem = $state("");

  onMount(reload);

  async function reload() {
    try {
      projects = await evaluationPlan.projectListGet();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function remove(uuid: string) {
    try {
      await evaluationPlan.projectListDelete(uuid);
      confirmDelete = null;
      await reload();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

<div class="buttons section-actions">
  <button type="button" class="primary" onclick={onnew}>New master project…</button>
</div>

{#if projects.length === 0}
  <div class="card">
    <h3>No master project yet</h3>
    <p class="note">
      A master project holds the catalogs of a standard and all its tasks, each a whole requirement. It is made once
      for a version of the standard and used again and again; the packages for the evaluators are taken from it.
    </p>
  </div>
{:else}
  <div class="table-card">
    <table class="data documents">
      <thead><tr><th>Master project</th><th>Catalogs</th><th>Tasks</th><th></th></tr></thead>
      <tbody>
        {#each projects as project (project.uuid)}
          <tr>
            <td>{project.title}</td>
            <td>{project.catalogs}</td>
            <td>{project.tasks}</td>
            <td class="actions">
              <button type="button" class="small" onclick={() => onopen(project.uuid)}>Open</button>
              {#if confirmDelete === project.uuid}
                <button type="button" class="small danger" onclick={() => remove(project.uuid)}>Delete</button>
                <button type="button" class="small" onclick={() => (confirmDelete = null)}>Keep</button>
              {:else}
                <button type="button" class="small danger" onclick={() => (confirmDelete = project.uuid)}>Delete</button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
