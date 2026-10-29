<!--
  The tab Standards of the panel of the plan (its commands standard_list and
  standard_import in src/panels/views/evaluation_plan/standards.rs): every
  standard of the user, its title whole and its version, and the import of
  an OSCAL catalog through the dialog of the system.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
  import { sentence } from "../../lib/api/errors";
  import { standardImport, standardList, type StandardView } from "../../lib/api/evaluation_plan";

  let standards: StandardView[] = $state([]);
  let problem = $state("");
  let busy = $state(false);
  let savedAt: Date | null = $state(null);

  async function reload() {
    try {
      standards = await standardList();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function importStandard() {
    problem = "";
    busy = true;

    try {
      if ((await standardImport()) !== null) {
        savedAt = new Date();
      }

      await reload();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  onMount(reload);
</script>

<section class="card">
  <h2>Standards</h2>
  <p class="note">The OSCAL catalogs the plans are made from, each checked against the schema of NIST when imported.</p>
  {#if problem}<p class="problem">{problem}</p>{/if}

  <table class="data">
    <thead><tr><th>Title</th><th>Version</th></tr></thead>
    <tbody>
      {#each standards as standard (standard.id)}
        <tr><td class="wrap">{standard.title}</td><td class="wrap">{standard.version}</td></tr>
      {:else}
        <tr><td colspan="2" class="note">No standard yet: import an OSCAL catalog.</td></tr>
      {/each}
    </tbody>
  </table>

  <div class="actions">
    <button type="button" onclick={importStandard} disabled={busy}>{busy ? "Importing…" : "Import a standard"}</button>
    <SaveStatus {busy} {savedAt} />
  </div>
</section>

<style>
  .actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-top: 1rem;
  }

  table.data {
    width: 100%;
  }

  table.data td {
    white-space: normal;
  }
</style>
