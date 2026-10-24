<!--
  The element "list of catalogs" of the view Converter (its command
  catalog_list_get in src/views/converter/converter.rs): the catalogs of OSCAL
  of the evaluator. "Open" and "New catalog…" only tell the view (onopen,
  onnew), which opens the editor of a catalog or the dialog of a new one.
  The list is read again when the view shows it (after the editor closes).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as converter from "../../lib/api/converter";

  let { onopen, onnew }: { onopen: (uuid: string) => void; onnew: () => void } = $props();

  let catalogs: converter.CatalogView[] = $state([]);
  let problem = $state("");

  onMount(async () => {
    try {
      catalogs = await converter.catalogListGet();
    } catch (error) {
      problem = sentence(error);
    }
  });
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

<div class="buttons section-actions">
  <button type="button" class="primary" onclick={onnew}>New catalog…</button>
</div>

{#if catalogs.length === 0}
  <div class="card">
    <h3>No catalog yet</h3>
    <p class="note">
      The converter turns the documents of a standard (PDF) into a catalog of NIST OSCAL. Load the PDFs on the tab
      Documents of standards, make a new catalog from them, and let the AI propose its groups and controls; you accept,
      correct or reject every proposal. The finished catalog is used in the view Evaluation plan.
    </p>
  </div>
{:else}
  <div class="table-card">
    <table class="data documents">
      <thead>
        <tr><th>Title</th><th>Version</th><th>Last modified</th><th></th></tr>
      </thead>
      <tbody>
        {#each catalogs as catalog (catalog.uuid)}
          <tr>
            <td>{catalog.title}</td>
            <td>{catalog.version}</td>
            <td class="note">{catalog.last_modified}</td>
            <td class="actions">
              <button type="button" class="small" onclick={() => onopen(catalog.uuid)}>Open</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
