<!--
  The analysis of one scheme with AI (steps 20 and 27 of
  docs/todo.md): which pages of which documents of the project (by
  default the pages of the scheme), how much text goes into one
  request (so that it fits the context of the model), and the
  instructions the model gets, which the user can read and change.
  The model also gets the requirements of the other schemes, to link
  to them.

  The model of the active AI connection (Settings, tab AI) proposes
  tiles and links; the program checks every quote and stores the
  proposals as *proposed* tiles on the canvas. Nothing is accepted
  without the user. "Undo" in the editor takes a whole analysis back.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { stopWork } from "../lib/api/documents";
import { sentence } from "../lib/api/errors";
import { analyseScheme, analysisInstructions, saveAnalysisInstructions, type AnalysisEvent, type SchemeDataView } from "../lib/api/projects";

  let {
    data,
    ondone,
    onclose,
  }: {
    data: SchemeDataView;
    ondone: (data: SchemeDataView) => void;
    onclose: () => void;
  } = $props();

  /** The pages of every document: whether it is analysed, and from where to where. */
  let ranges = $state<{ document_id: number; title: string; use: boolean; first: number; last: number; pages: number }[]>([]);
  let partSize = $state(12000);
  let instructions = $state("");
  let instructionsChanged = $state(false);
  let running = $state(false);
  let log: { done: boolean | null; text: string }[] = $state([]);
  let problem = $state("");
  let writing = $state(0);

  onMount(async () => {
    const own = data.scheme.pages;

    ranges = data.documents.map((document) => ({
      document_id: document.id,
      title: `${document.title}${document.part ? ` — ${document.part}` : ""}`,
      use: own ? own.document_id === document.id : true,
      first: own?.document_id === document.id ? own.first_page : 1,
      last: own?.document_id === document.id ? own.last_page : document.page_count,
      pages: document.page_count,
    }));

    try {
      instructions = await analysisInstructions("chapter");
    } catch (error) {
      problem = sentence(error);
    }
  });

  async function keepInstructions() {
    try {
      instructions = await saveAnalysisInstructions("chapter", instructions);
      instructionsChanged = false;
    } catch (error) {
      problem = `The instructions were not stored: ${sentence(error)}`;
    }
  }

  async function resetInstructions() {
    instructions = await saveAnalysisInstructions("chapter", "");
    instructionsChanged = false;
  }

  function event(event: AnalysisEvent) {
    switch (event.event) {
      case "part":
        writing = 0;
        log.push({
          done: null,
          text: `Part ${event.index} of ${event.count}: ${event.document}, pages ${event.first_page} to ${event.last_page}…`,
        });
        break;
      case "writing":
        writing = event.characters;
        break;
      case "applied": {
        const last = log[log.length - 1];

        if (last) {
          last.done = true;
          last.text += ` ${event.added} new tiles, ${event.joined} joined to known ones, ${event.links} links.`;
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
        log.push({ done: !event.stopped, text: event.stopped ? "Stopped." : "The analysis is done." });
        break;
    }
  }

  async function start() {
    problem = "";
    log = [];

    if (instructionsChanged) {
      await keepInstructions();
    }

    const chosen = ranges
      .filter((range) => range.use)
      .map((range) => ({
        document_id: range.document_id,
        first_page: Math.max(1, Math.min(range.first, range.pages)),
        last_page: Math.max(1, Math.min(range.last, range.pages)),
      }));

    if (chosen.length === 0) {
      problem = "Choose at least one document.";
      return;
    }

    running = true;

    try {
      ondone(await analyseScheme(data.scheme.id, chosen, partSize, event));
    } catch (error) {
      problem = `The analysis did not run: ${sentence(error)}`;
    } finally {
      running = false;
    }
  }
</script>

<div class="overlay" role="presentation">
  <div class="dialog wide" role="dialog" aria-label="Analyse with AI">
    <h2>Analyse the scheme with AI</h2>
    <p class="note">
      The model of your active AI connection reads the chosen pages part by part and proposes tiles and links. Every
      quote is checked against the page; the proposals appear on the canvas as proposed, for you to accept, correct or
      delete.
    </p>

    {#if problem}<p class="problem">{problem}</p>{/if}

    {#if ranges.length === 0}
      <p class="problem">The project uses no document yet: add documents in the settings of the project.</p>
    {/if}

    {#each ranges as range (range.document_id)}
      <div class="range">
        <label class="choice"><input type="checkbox" bind:checked={range.use} disabled={running} /> {range.title}</label>
        <span class="note">pages</span>
        <input type="number" min="1" max={range.pages} bind:value={range.first} disabled={running || !range.use} />
        <span class="note">to</span>
        <input type="number" min="1" max={range.pages} bind:value={range.last} disabled={running || !range.use} />
        <span class="note">of {range.pages}</span>
      </div>
    {/each}

    <label>
      Characters in one request
      <input type="number" min="2000" max="400000" step="1000" bind:value={partSize} disabled={running} />
      <span class="note">
        About 4 characters make a token. 12000 fits even a small local model; a model with a large context can take far
        more, which means fewer requests.
      </span>
    </label>

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
        <button type="button" class="primary" onclick={start} disabled={ranges.length === 0}>Start the analysis</button>
        <button type="button" onclick={onclose}>Close</button>
      {/if}
    </div>
  </div>
</div>
