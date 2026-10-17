<!--
  The table of contents read by the AI (step 27 of docs/todo.md): the
  user names the pages of the table of contents; the model of the
  active AI connection proposes the chapters that hold requirements;
  the program finds where each chapter starts in the document and
  stores the chapters as proposed schemes of the project, for the user
  to correct, accept or delete.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { stopWork } from "../lib/api/documents";
import { sentence } from "../lib/api/errors";
import { analysisInstructions, proposeSchemes, saveAnalysisInstructions, type AnalysisEvent, type ProjectDataView } from "../lib/api/projects";

  let {
    data,
    ondone,
    onclose,
  }: {
    data: ProjectDataView;
    ondone: (data: ProjectDataView) => void;
    onclose: () => void;
  } = $props();

  let documentId = $state(0);
  let first = $state(1);
  let last = $state(5);
  let instructions = $state("");
  let instructionsChanged = $state(false);
  let running = $state(false);
  let log: { done: boolean | null; text: string }[] = $state([]);
  let problem = $state("");
  let writing = $state(0);

  let pages = $derived(data.documents.find((document) => document.id === documentId)?.page_count ?? 1);

  onMount(async () => {
    documentId = data.documents[0]?.id ?? 0;

    try {
      instructions = await analysisInstructions("contents");
    } catch (error) {
      problem = sentence(error);
    }
  });

  async function keepInstructions() {
    try {
      instructions = await saveAnalysisInstructions("contents", instructions);
      instructionsChanged = false;
    } catch (error) {
      problem = `The instructions were not stored: ${sentence(error)}`;
    }
  }

  async function resetInstructions() {
    instructions = await saveAnalysisInstructions("contents", "");
    instructionsChanged = false;
  }

  function event(event: AnalysisEvent) {
    switch (event.event) {
      case "part":
        writing = 0;
        log.push({ done: null, text: `Reading the table of contents, pages ${event.first_page} to ${event.last_page}…` });
        break;
      case "writing":
        writing = event.characters;
        break;
      case "chapters": {
        const line = log[log.length - 1];

        if (line) {
          line.done = true;
          line.text += ` ${event.added} new proposed schemes${event.known > 0 ? `, ${event.known} chapters the project has already` : ""}.`;
        }

        if (event.without_pages > 0) {
          log.push({ done: false, text: `The pages of ${event.without_pages} chapters were not found: give them in the scheme.` });
        }

        for (const reason of event.dropped) {
          log.push({ done: false, text: `Dropped: ${reason}` });
        }
        break;
      }
      case "failed":
        log.push({ done: false, text: event.message });
        break;
      case "end":
        log.push({ done: !event.stopped, text: event.stopped ? "Stopped." : "Done." });
        break;
    }
  }

  async function start() {
    problem = "";
    log = [];

    if (instructionsChanged) {
      await keepInstructions();
    }

    running = true;

    try {
      const range = {
        document_id: documentId,
        first_page: Math.max(1, Math.min(first, pages)),
        last_page: Math.max(1, Math.min(last, pages)),
      };

      ondone(await proposeSchemes(data.project.id, range, event));
    } catch (error) {
      problem = `The table of contents was not read: ${sentence(error)}`;
    } finally {
      running = false;
    }
  }
</script>

<div class="overlay" role="presentation">
  <div class="dialog wide" role="dialog" aria-label="Read the table of contents with AI">
    <h2>Read the table of contents with AI</h2>
    <p class="note">
      The model of your active AI connection reads the table of contents and proposes the chapters that hold requirements.
      Each becomes a proposed scheme with the pages where its heading was found. Check them, correct the pages where
      needed, and accept them; then let the AI analyse the chapters.
    </p>

    {#if problem}<p class="problem">{problem}</p>{/if}

    <div class="range">
      <label>
        Document
        <select bind:value={documentId} disabled={running}>
          {#each data.documents as document (document.id)}
            <option value={document.id}>{document.title}{document.part ? ` — ${document.part}` : ""}</option>
          {/each}
        </select>
      </label>
      <span class="note">table of contents on pages</span>
      <input type="number" min="1" max={pages} bind:value={first} disabled={running} />
      <span class="note">to</span>
      <input type="number" min="1" max={pages} bind:value={last} disabled={running} />
      <span class="note">of {pages}</span>
    </div>

    <details>
      <summary>Instructions to the model</summary>
      <textarea class="mono" rows="14" bind:value={instructions} oninput={() => (instructionsChanged = true)} disabled={running}></textarea>
      <div class="buttons">
        <button type="button" class="small" onclick={keepInstructions} disabled={running || !instructionsChanged}>Keep these</button>
        <button type="button" class="small" onclick={resetInstructions} disabled={running}>Back to the default</button>
      </div>
    </details>

    {#if log.length > 0}
      <div class="log">
        {#each log as line, index (index)}
          <div class={line.done === false ? "problem" : line.done ? "done" : "note"}>{line.text}</div>
        {/each}
        {#if running && writing > 0}<div class="note">The model has written {writing} characters…</div>{/if}
      </div>
    {/if}

    <div class="buttons">
      {#if running}
        <button type="button" class="danger" onclick={stopWork}>Stop</button>
      {:else}
        <button type="button" class="primary" onclick={start} disabled={data.documents.length === 0}>Read</button>
        <button type="button" onclick={onclose}>Close</button>
      {/if}
    </div>
  </div>
</div>
