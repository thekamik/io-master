<!--
  The element "documents of standards" of the view Converter
  (its commands in src/views/converter/converter.rs): the PDFs of the standards, loaded
  into the encrypted database with the text of their pages, their fields
  changed and deleted here. "View" only tells the view (onview), which
  shows the document in the viewer.

  Loading is a long work: its events arrive while it runs (the file, the
  pages read, loaded or not) and "Stop" ends it after the part being read.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as converter from "../../lib/api/converter";
  import ConfirmButton from "../../components/confirm_button/ConfirmButton.svelte";

  let { onview }: { onview: (documentId: number) => void } = $props();

  let documents: converter.DocumentView[] = $state([]);
  let editing: converter.DocumentView | null = $state(null);
  let problem = $state("");

  let loading = $state(false);
  let progress = $state("");
  let log: { done: boolean; text: string }[] = $state([]);

  async function reload() {
    try {
      documents = await converter.standardDocumentsGet();
    } catch (error) {
      problem = sentence(error);
    }
  }

  onMount(reload);

  function title(document: converter.DocumentView): string {
    return `${document.title}${document.part ? ` — ${document.part}` : ""}`;
  }

  function loadEvent(event: converter.LoadEvent) {
    switch (event.event) {
      case "file":
        progress = `Reading ${event.name} (${event.index} of ${event.count})…`;
        break;
      case "pages":
        progress = progress.replace(/ — page .*$/, "") + ` — page ${event.done} of ${event.total}`;
        break;
      case "loaded":
        log.push({ done: true, text: `Loaded: ${title(event.document)}, ${event.document.page_count} pages.` });
        break;
      case "failed":
        log.push({ done: false, text: `${event.name}: ${event.message}` });
        break;
      case "end":
        progress = event.stopped ? "Stopped." : "";
        break;
    }
  }

  async function load() {
    loading = true;
    problem = "";
    log = [];

    try {
      await converter.standardDocumentsLoad(loadEvent);
      await reload();
    } catch (error) {
      problem = sentence(error);
    } finally {
      loading = false;
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();

    if (editing) {
      try {
        await converter.standardDocumentsUpdate(editing.id, { title: editing.title, part: editing.part, version: editing.version });
        editing = null;
        await reload();
      } catch (error) {
        problem = `The document was not saved: ${sentence(error)}`;
      }
    }
  }

  async function remove(id: number) {
    try {
      await converter.standardDocumentsDelete(id);
      await reload();
    } catch (error) {
      problem = `The document was not deleted: ${sentence(error)}`;
    }
  }
</script>

<div class="buttons section-actions">
  <button type="button" class="primary" onclick={load} disabled={loading}>Load PDFs…</button>
</div>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if loading || progress || log.length > 0}
  <div class="card load-log">
    {#if progress}<p class="note">{progress}</p>{/if}
    {#each log as line, index (index)}
      <div class={line.done ? "done" : "problem"}>{line.text}</div>
    {/each}
    {#if loading}
      <button type="button" class="small danger" onclick={converter.standardDocumentsStop}>Stop</button>
    {:else}
      <button type="button" class="small" onclick={() => (log = [])}>Close</button>
    {/if}
  </div>
{/if}

{#if documents.length === 0}
  <div class="card">
    <h3>No document yet</h3>
    <p class="note">
      Load the documents of a standard as PDFs (a standard often comes in several parts: load each of them). Their text
      is read page by page and kept, with the files, in your encrypted database.
    </p>
  </div>
{:else}
  <div class="table-card">
    <table class="data documents">
      <thead>
        <tr><th>Title</th><th>Part</th><th>Version</th><th>Pages</th><th>File</th><th></th></tr>
      </thead>
      <tbody>
        {#each documents as document (document.id)}
          <tr>
            <td>{document.title}</td>
            <td>{document.part}</td>
            <td>{document.version}</td>
            <td>{document.page_count}</td>
            <td class="note">{document.file_name}</td>
            <td class="actions">
              <button type="button" class="small" onclick={() => onview(document.id)}>View</button>
              <button type="button" class="small" onclick={() => (editing = { ...document })}>Edit</button>
              <ConfirmButton onconfirm={() => remove(document.id)} />
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

{#if editing}
  <div class="overlay" role="presentation">
    <form class="dialog" onsubmit={save}>
      <h2>The document</h2>
      <label>Title <input bind:value={editing.title} required /></label>
      <div class="row">
        <label>Part <input bind:value={editing.part} placeholder="Part 3: Security assurance components" /></label>
        <label>Version <input bind:value={editing.version} placeholder="CC:2022 Revision 1" /></label>
      </div>
      <p class="note mono">{editing.file_name} · SHA-256 {editing.sha256.slice(0, 16)}…</p>
      <div class="buttons">
        <button type="submit">Save</button>
        <button type="button" onclick={() => (editing = null)}>Cancel</button>
      </div>
    </form>
  </div>
{/if}
