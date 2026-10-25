<!--
  The element "proving ground" of the view Knowledge (RAG) (its commands
  proving_ground_find, proving_ground_get, proving_ground_ask and
  knowledge_stop in src/views/knowledge/knowledge.rs): a question tried on
  an agent. "Show the passages" shows what the agent would give the model,
  without the AI; "Ask the agent" asks the model of the active connection,
  and shows the passages it got, its answer, and its quotes, each found or
  not found in the passage it names. The last trial is kept in Rust.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as knowledge from "../../lib/api/knowledge";
  import { follow, idle, start } from "../../components/ai_work/ai_progress";

  let { agent }: { agent: number } = $props();

  let question = $state("");
  /** The passages shown without the AI; null when none were asked for. */
  let found: knowledge.HitView[] | null = $state(null);
  let trial: knowledge.TrialView | null = $state(null);
  let progress = $state(idle());

  onMount(async () => {
    try {
      trial = await knowledge.provingGroundGet(agent);
    } catch (error) {
      progress.problem = sentence(error);
    }
  });

  async function find() {
    progress.problem = "";

    try {
      found = await knowledge.provingGroundFind(agent, question);
    } catch (error) {
      progress.problem = sentence(error);
    }
  }

  async function ask(event: SubmitEvent) {
    event.preventDefault();
    found = null;
    start(progress, "The agent answers");

    try {
      await knowledge.provingGroundAsk(agent, question, followAnswer);
      trial = await knowledge.provingGroundGet(agent);
    } catch (error) {
      progress.problem = sentence(error);
    } finally {
      progress.running = false;
    }
  }

  /** One event of the question: an answer kept is said in words of its own,
      its quotes are shown under it; every other event as on the panels. */
  function followAnswer(event: knowledge.AiEvent) {
    if (event.event === "proposed") {
      progress.line = "The agent answered.";
    } else {
      follow(progress, event, "The agent answers");
    }
  }

  /** Where a passage stands: its source, requirement and page. */
  function place(hit: knowledge.HitView): string {
    const parts = [hit.source];

    if (hit.label) parts.push(hit.label);
    if (hit.page !== null) parts.push(`page ${hit.page}`);

    return parts.join(" · ");
  }
</script>

{#snippet passages(hits: knowledge.HitView[])}
  {#each hits as hit, index (hit.id)}
    <div class="link-row">
      <p class="note">[{index + 1}] {place(hit)} · relevance {(-hit.score).toFixed(1)}</p>
      <blockquote>{hit.text}</blockquote>
    </div>
  {:else}
    <p class="note">No passage of the bases of the agent holds these words.</p>
  {/each}
{/snippet}

<div class="card knowledge-search">
  <h3>Proving ground</h3>
  <p class="note">Try the agent on a question you know the answer to. If the passages are wrong, add sources or glossary entries; if the answer is wrong, change the instructions. Every question to the AI is written into its journal.</p>
  <form class="block-form" onsubmit={ask}>
    <textarea bind:value={question} rows="2" placeholder="What must the developer provide for ADV_FSP.4?"></textarea>
    <div class="buttons">
      <button type="button" disabled={progress.running || question.trim() === ""} onclick={find}>Show the passages</button>
      <button type="submit" disabled={progress.running || question.trim() === ""}>Ask the agent</button>
      {#if progress.running}<button type="button" onclick={() => knowledge.knowledgeStop()}>Stop</button>{/if}
    </div>
  </form>

  {#if progress.problem}<p class="problem">{progress.problem}</p>{/if}
  {#if progress.line}<p class="note">{progress.line}</p>{/if}

  {#if found !== null}
    <h4>The passages the agent would give the model</h4>
    {@render passages(found)}
  {:else if trial}
    <h4>The last question</h4>
    <p>{trial.question}</p>
    <h4>The answer</h4>
    <div class="ai-proposal"><pre>{trial.answer}</pre></div>
    {#if trial.quotes.length > 0}
      <h4>Its quotes</h4>
      {#each trial.quotes as quote, index (index)}
        <p class={quote.found ? "done" : "problem"}>
          {quote.found ? "Found" : "Not found"} in passage [{quote.passage}]: “{quote.quote}”
        </p>
      {/each}
    {/if}
    <h4>The passages it was given</h4>
    {@render passages(trial.passages)}
  {/if}
</div>
