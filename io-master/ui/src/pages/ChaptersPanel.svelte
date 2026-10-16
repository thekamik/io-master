<!--
  The analysis of several chapters with AI (step 27 of docs/todo.md):
  the user chooses the schemes; the model of the active AI connection
  reads the pages of each, one after another, and proposes its tiles
  and links (also to requirements of other chapters). A scheme without
  pages is left out. "Undo" in a scheme takes the whole work back.

  By default the accepted schemes that are still empty are chosen.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import {
    analyseSchemes,
    schemeName,
    sentence,
    stopWork,
    type AnalysisEvent,
    type ProjectDataView,
  } from "../lib/api";

  let {
    data,
    ondone,
    onclose,
  }: {
    data: ProjectDataView;
    ondone: (data: ProjectDataView) => void;
    onclose: () => void;
  } = $props();

  let chosen: number[] = $state([]);
  let partSize = $state(12000);
  let running = $state(false);
  let log: { done: boolean | null; text: string }[] = $state([]);
  let problem = $state("");
  let writing = $state(0);

  onMount(() => {
    chosen = data.schemes
      .filter((scheme) => scheme.state === "accepted" && scheme.tiles === 0 && scheme.pages)
      .map((scheme) => scheme.id);
  });

  function event(event: AnalysisEvent) {
    switch (event.event) {
      case "scheme":
        log.push({ done: null, text: `Scheme ${event.index} of ${event.count}: ${schemeName(event)}` });
        break;
      case "part":
        writing = 0;
        log.push({ done: null, text: `  part ${event.index} of ${event.count}, pages ${event.first_page} to ${event.last_page}…` });
        break;
      case "writing":
        writing = event.characters;
        break;
      case "applied": {
        const line = log[log.length - 1];

        if (line) {
          line.done = true;
          line.text += ` ${event.added} new tiles, ${event.joined} joined, ${event.links} links.`;
        }

        for (const reason of event.dropped) {
          log.push({ done: false, text: `  Dropped: ${reason}` });
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

    if (chosen.length === 0) {
      problem = "Choose at least one scheme.";
      return;
    }

    running = true;

    try {
      ondone(await analyseSchemes(data.project.id, chosen, partSize, event));
    } catch (error) {
      problem = `The analysis did not run: ${sentence(error)}`;
    } finally {
      running = false;
    }
  }
</script>

<div class="overlay" role="presentation">
  <div class="dialog wide" role="dialog" aria-label="Analyse chapters with AI">
    <h2>Analyse chapters with AI</h2>
    <p class="note">
      The model reads the pages of every chosen scheme and proposes its inputs, requirements, outputs and links. The
      instructions are those of the analysis of a scheme ("Analyse with AI" in a scheme).
    </p>

    {#if problem}<p class="problem">{problem}</p>{/if}

    <fieldset class="flat chapter-choice">
      <legend>Schemes</legend>
      {#each data.schemes as scheme (scheme.id)}
        <label class="choice">
          <input type="checkbox" bind:group={chosen} value={scheme.id} disabled={running || !scheme.pages} />
          {schemeName(scheme)}
          <span class="note">
            {scheme.pages ? `pages ${scheme.pages.first_page}–${scheme.pages.last_page}` : "no pages"} · {scheme.tiles} tiles{scheme.state === "proposed" ? " · proposed" : ""}
          </span>
        </label>
      {/each}
    </fieldset>

    <label>
      Characters in one request
      <input type="number" min="2000" max="400000" step="1000" bind:value={partSize} disabled={running} />
      <span class="note">About 4 characters make a token; a model with a large context can take far more.</span>
    </label>

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
        <button type="button" class="primary" onclick={start}>Start the analysis</button>
        <button type="button" onclick={onclose}>Close</button>
      {/if}
    </div>
  </div>
</div>
