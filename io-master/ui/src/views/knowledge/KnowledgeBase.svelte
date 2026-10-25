<!--
  The element "knowledge base" of the view Knowledge (RAG) (its commands
  knowledge_base_* in src/views/knowledge/knowledge.rs): the name and the
  description of a base, its sources, and the sources added to it: a PDF or
  a text file (chosen in a dialog of the system), a catalog of OSCAL of the
  installation, a note. Every source is cut into passages in Rust; under
  them the search of the base (KnowledgeSearch).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as knowledge from "../../lib/api/knowledge";
  import ConfirmButton from "../../components/confirm_button/ConfirmButton.svelte";
  import KnowledgeSearch from "./KnowledgeSearch.svelte";

  let { id, onclose }: { id: number; onclose: () => void } = $props();

  let view: knowledge.BaseScreenView | null = $state(null);
  let form: knowledge.BaseForm = $state({ name: "", description: "" });
  let catalogs: knowledge.CatalogChoiceView[] = $state([]);
  let catalog = $state("");
  let note = $state({ name: "", text: "" });
  /** What is added now (a file is read, a catalog cut); "" when nothing. */
  let busy = $state("");
  let notice: { done: boolean; text: string } | null = $state(null);

  const kinds: Record<knowledge.SourceView["kind"], string> = {
    pdf: "PDF",
    text: "Text file",
    catalog: "Catalog of OSCAL",
    note: "Note",
  };

  onMount(async () => {
    try {
      show(await knowledge.knowledgeBaseGet(id));
      catalogs = await knowledge.knowledgeBaseCatalogs();
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  });

  function show(screen: knowledge.BaseScreenView) {
    view = screen;
    form = { name: screen.name, description: screen.description };
  }

  /** Runs `work` while `what` is shown, and shows its result or its error. */
  async function run(what: string, work: () => Promise<knowledge.BaseScreenView | null>, done: string) {
    busy = what;
    notice = null;

    try {
      const screen = await work();

      if (screen) {
        show(screen);
        notice = { done: true, text: done };
      }
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    } finally {
      busy = "";
    }
  }

  function save(event: SubmitEvent) {
    event.preventDefault();
    run("Saving…", () => knowledge.knowledgeBaseSave(id, { ...form }), "Saved.");
  }

  function addNote(event: SubmitEvent) {
    event.preventDefault();
    run("Adding the note…", () => knowledge.knowledgeBaseAddNote(id, note.name, note.text), "The note was added.").then(() => {
      if (notice?.done) note = { name: "", text: "" };
    });
  }
</script>

<div class="editor-head">
  <button type="button" class="small" onclick={onclose}>← All knowledge bases</button>
</div>

{#if view}
  <form class="card block-form" onsubmit={save}>
    <h3>{view.name}</h3>
    <label>Name <input bind:value={form.name} required /></label>
    <label>What it is for <textarea bind:value={form.description} rows="2"></textarea></label>
    <div class="buttons"><button type="submit" disabled={busy !== ""}>Save</button></div>
  </form>

  <div class="card">
    <h3>Sources</h3>
    <p class="note">Each source is cut into passages of a paragraph or so, never across a page, so that a passage given to the model can be shown with its page. Only the text is kept, encrypted with your database; the file itself is not.</p>

    {#if view.sources.length === 0}
      <p class="note">No source yet.</p>
    {:else}
      <table class="data">
        <thead><tr><th>Source</th><th>Kind</th><th>Passages</th><th></th></tr></thead>
        <tbody>
          {#each view.sources as source (source.id)}
            <tr>
              <td>{source.name}</td>
              <td>{kinds[source.kind]}</td>
              <td>{source.passages}</td>
              <td class="actions">
                <ConfirmButton label="Remove" disabled={busy !== ""} onconfirm={() => run("Removing…", () => knowledge.knowledgeBaseRemove(id, source.id), "The source was removed.")} />
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}

    <div class="knowledge-add">
      <h4>A file</h4>
      <div class="buttons">
        <button type="button" disabled={busy !== ""} onclick={() => run("Reading the file…", () => knowledge.knowledgeBaseAddFile(id), "The file was added.")}>Add a PDF or a text file…</button>
      </div>
    </div>

    {#if catalogs.length > 0}
      <div class="knowledge-add">
        <h4>A catalog of OSCAL of this installation</h4>
        <div class="buttons">
          <select bind:value={catalog}>
            <option value="">Choose a catalog…</option>
            {#each catalogs as choice (choice.uuid)}
              <option value={choice.uuid}>{choice.title}{choice.version ? ` (${choice.version})` : ""}</option>
            {/each}
          </select>
          <button type="button" disabled={busy !== "" || catalog === ""} onclick={() => run("Adding the catalog…", () => knowledge.knowledgeBaseAddCatalog(id, catalog), "The catalog was added.")}>Add the catalog</button>
        </div>
      </div>
    {/if}

    <form class="block-form knowledge-add" onsubmit={addNote}>
      <h4>A note</h4>
      <label>Its name <input bind:value={note.name} placeholder="Interpretation of the scheme" /></label>
      <label>Its text <textarea bind:value={note.text} rows="4" required></textarea></label>
      <div class="buttons"><button type="submit" disabled={busy !== ""}>Add the note</button></div>
    </form>

    {#if busy}<p class="note">{busy}</p>{/if}
    {#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}
  </div>

  <KnowledgeSearch base={id} />
{:else if notice}
  <p class="problem">{notice.text}</p>
{/if}
