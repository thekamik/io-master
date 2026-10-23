<!--
  The element "documents of OSCAL" of the view Evaluation plan
  (src/views/evaluation_plan/oscal_documents/): the catalogs, mappings, profiles and
  plans, imported, exported and deleted here. "Open" and "New catalog…"
  only tell the view (onopen, onnew). The view asks it to read the list
  again after a catalog was made or closed (reload(), below).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as oscalDocuments from "../../lib/api/evaluation_plan/oscal_documents";

  let { onopen, onnew }: { onopen: (uuid: string) => void; onnew: () => void } = $props();

  let documents: oscalDocuments.OscalDocumentView[] = $state([]);
  let confirmDelete: string | null = $state(null);
  let problem = $state("");
  let notice = $state("");

  /** Reads the documents again; called by the view after a change. */
  export async function reload() {
    try {
      documents = await oscalDocuments.get();
    } catch (error) {
      problem = sentence(error);
    }
  }

  onMount(reload);

  async function importFile() {
    problem = notice = "";

    try {
      const imported = await oscalDocuments.importFile();

      if (imported) {
        notice = `Imported: ${imported.model_title} "${imported.title}".`;
        await reload();
      }
    } catch (error) {
      problem = `The OSCAL document was not imported: ${sentence(error)}`;
    }
  }

  async function exportFile(document: oscalDocuments.OscalDocumentView) {
    problem = notice = "";

    try {
      if (await oscalDocuments.exportFile(document.uuid, document.title)) {
        notice = `Exported: ${document.title}.`;
      }
    } catch (error) {
      problem = `The OSCAL document was not exported: ${sentence(error)}`;
    }
  }

  async function remove(uuid: string) {
    try {
      await oscalDocuments.remove(uuid);
      confirmDelete = null;
      await reload();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

{#if problem}<p class="problem">{problem}</p>{/if}
{#if notice}<p class="done">{notice}</p>{/if}

<div class="buttons section-actions">
  <button type="button" class="primary" onclick={onnew}>New catalog…</button>
  <button type="button" onclick={importFile}>Import an OSCAL document…</button>
</div>

{#if documents.length === 0}
  <div class="card">
    <h3>No OSCAL document yet</h3>
    <p class="note">
      The evaluation plan is built on NIST OSCAL: the requirements of a standard as a catalog, the relations between
      documents as a mapping, the choice for one evaluation as a profile, the plan as an assessment plan. Make a catalog
      of a standard from its documents (PDF), with the help of the AI, or import a document of OSCAL (JSON, version
      1.x). Every document is checked against the schema of NIST.
    </p>
  </div>
{:else}
  <div class="table-card">
    <table class="ports documents">
      <thead>
        <tr><th>Model</th><th>Title</th><th>Version</th><th>OSCAL</th><th>Last modified</th><th></th></tr>
      </thead>
      <tbody>
        {#each documents as document (document.uuid)}
          <tr>
            <td>{document.model_title}</td>
            <td>{document.title}</td>
            <td>{document.version}</td>
            <td class="note">{document.oscal_version}</td>
            <td class="note">{document.last_modified}</td>
            <td class="actions">
              {#if document.model === "catalog"}
                <button type="button" class="small" onclick={() => onopen(document.uuid)}>Open</button>
              {/if}
              <button type="button" class="small" onclick={() => exportFile(document)}>Export</button>
              {#if confirmDelete === document.uuid}
                <button type="button" class="small danger" onclick={() => remove(document.uuid)}>Delete</button>
                <button type="button" class="small" onclick={() => (confirmDelete = null)}>Keep</button>
              {:else}
                <button type="button" class="small danger" onclick={() => (confirmDelete = document.uuid)}>Delete</button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
