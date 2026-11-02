<!--
  One knowledge base (its commands knowledge_base_list, knowledge_base_save,
  knowledge_source_list, knowledge_source_add, knowledge_source_replace,
  knowledge_source_rename and knowledge_search in
  src/panels/views/agents/knowledge.rs): its name and description, the
  length of its passages (its sources indexed anew when it changes) and its
  stop words, its sources (a .txt, .md or .pdf file chosen in the dialog of the system; a
  source is never removed, but its text may be replaced by a newer file and
  its name changed), and the search in the base.
-->
<script lang="ts">
  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
  import {
    knowledgeBaseList,
    knowledgeBaseSave,
    knowledgeSearch,
    knowledgeSourceAdd,
    knowledgeSourceList,
    knowledgeSourceRename,
    knowledgeSourceReplace,
    type BaseView,
    type SourceView,
  } from "../../lib/api/agents";
  import { sentence } from "../../lib/api/errors";

  let { base, onback }: { base: number; onback: () => void } = $props();

  let view: BaseView | null = $state(null);
  let name = $state("");
  let description = $state("");
  let passageCharacters: number | null = $state(null);
  let stopWords = $state("");
  let sources: SourceView[] = $state([]);
  /** The source being renamed, and its new name. */
  let renaming: number | null = $state(null);
  let newName = $state("");
  let question = $state("");
  let found: string[] | null = $state(null);
  let problem = $state("");
  let busy = $state(false);
  let savedAt: Date | null = $state(null);

  async function load(id: number) {
    problem = "";

    try {
      view = (await knowledgeBaseList()).find((one) => one.id === id) ?? null;
      name = view?.name ?? "";
      description = view?.description ?? "";
      passageCharacters = view?.passageCharacters ?? null;
      stopWords = view?.stopWords ?? "";
      sources = await knowledgeSourceList(id);
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    void load(base);
  });

  /** Runs one write: the buttons locked while it runs, the time of the
   * write shown when it changed something, the sources read again. */
  async function write(work: () => Promise<boolean>) {
    problem = "";
    busy = true;

    try {
      if (await work()) {
        savedAt = new Date();
        sources = await knowledgeSourceList(base);
      }
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  function save(event: SubmitEvent) {
    event.preventDefault();

    // The field is required: nothing is sent until it is given.
    if (passageCharacters === null) {
      return;
    }

    const fields = { id: base, name, description, passageCharacters, stopWords };

    void write(async () => {
      await knowledgeBaseSave(fields);
      return true;
    });
  }

  function rename(event: SubmitEvent, id: number) {
    event.preventDefault();
    void write(async () => {
      await knowledgeSourceRename(id, newName);
      renaming = null;
      return true;
    });
  }

  async function search(event: SubmitEvent) {
    event.preventDefault();
    problem = "";

    try {
      found = await knowledgeSearch(base, question);
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<p><button type="button" class="ghost" onclick={onback}>← Knowledge bases</button></p>

<section class="card ai-config">
  <h2>{name}</h2>
  <form onsubmit={save}>
    <div class="row">
      <label>Name <input bind:value={name} required /></label>
      <label>Description <input bind:value={description} /></label>
    </div>
    {#if view}
      <label>
        Passage, at most (characters)
        <span class="note">The sources are cut into passages of this length; a change indexes them anew.</span>
        <input
          type="number"
          min={view.minPassageCharacters}
          max={view.maxPassageCharacters}
          step="1"
          bind:value={passageCharacters}
          required
        />
      </label>
    {/if}
    <label>
      Stop words
      <span class="note">Words a search passes over, parted by spaces: they are in almost every passage.</span>
      <textarea rows="4" bind:value={stopWords}></textarea>
    </label>
    <div class="buttons"><button type="submit" disabled={busy}>Save</button></div>
  </form>

  <h3>Sources</h3>
  <table class="data">
    <thead><tr><th>Source</th><th>Characters</th><th></th></tr></thead>
    <tbody>
      {#each sources as source (source.id)}
        <tr>
          <td class="wrap">
            {#if renaming === source.id}
              <form class="inline" onsubmit={(event) => rename(event, source.id)}>
                <input bind:value={newName} required />
                <button type="submit" disabled={busy}>Rename</button>
                <button type="button" class="ghost" onclick={() => (renaming = null)}>Cancel</button>
              </form>
            {:else}
              {source.name}
            {/if}
          </td>
          <td>{source.characters.toLocaleString()}</td>
          <td class="actions">
            <button type="button" disabled={busy} onclick={() => ((renaming = source.id), (newName = source.name))}>
              Rename
            </button>
            <button type="button" disabled={busy} onclick={() => write(() => knowledgeSourceReplace(source.id))}>
              Replace the text
            </button>
          </td>
        </tr>
      {:else}
        <tr><td colspan="3" class="note">No source yet: add a .txt, .md or .pdf file.</td></tr>
      {/each}
    </tbody>
  </table>
  <p>
    <button type="button" disabled={busy} onclick={() => write(async () => (await knowledgeSourceAdd(base)) !== null)}>
      {busy ? "Reading…" : "Add a file"}
    </button>
  </p>
  <SaveStatus {busy} {savedAt} />
</section>

<section class="card">
  <h2>Search in the base</h2>
  <form onsubmit={search}>
    <label>Question <input bind:value={question} required /></label>
    <button type="submit">Search</button>
  </form>
  {#if found !== null}
    {#if found.length === 0}
      <p class="note">No passage fits the question.</p>
    {:else}
      {#each found as passage, index (index)}
        <pre class="passage">{passage}</pre>
      {/each}
    {/if}
  {/if}
</section>

{#if problem}<p class="problem">{problem}</p>{/if}

<style>
  table.data {
    width: 100%;
  }

  td.actions {
    white-space: nowrap;
  }

  td.actions button + button {
    margin-left: 0.4rem;
  }

  form.inline {
    display: flex;
    gap: 0.4rem;
  }

  pre.passage {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 0.85rem;
  }
</style>
