<!--
  One work unit of an evaluation (its commands evaluation_unit_get and
  evaluation_unit_save in src/panels/views/evaluation/work_units.rs): its
  planning, read only (an evaluation never changes it), and its results,
  which the evaluator may correct: saved, they are of the evaluator, and
  the AI never writes them again (user, 2026-10-30). A quote must stand word
  for word in its document. Below, the last question of the AI about it and
  its answer, as the journal keeps them. "Previous" and "next" follow the
  order of the list it was opened from.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import {
    evaluationUnitGet,
    evaluationUnitSave,
    type QuoteView,
    type UnitView,
    type VerdictName,
  } from "../../lib/api/evaluation";

  let {
    evaluation,
    unit,
    order,
    onopen,
    onback,
  }: { evaluation: number; unit: number; order: number[]; onopen: (unit: number) => void; onback: () => void } =
    $props();

  let view: UnitView | null = $state(null);
  let findings = $state("");
  let analysis = $state("");
  let vulnerabilityNotes = $state("");
  let notes = $state("");
  let quotes: QuoteView[] = $state([]);
  let verdict: VerdictName | "" = $state("");
  let problem = $state("");
  let saved = $state("");

  let place = $derived(order.indexOf(unit));

  async function reload() {
    try {
      view = await evaluationUnitGet(evaluation, unit);
      findings = view.findings;
      analysis = view.analysis;
      vulnerabilityNotes = view.vulnerabilityNotes;
      notes = view.notes;
      quotes = view.quotes.map((quote) => ({ ...quote }));
      verdict = view.verdict ?? "";
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    saved = "";

    try {
      await evaluationUnitSave(evaluation, unit, { findings, analysis, vulnerabilityNotes, notes, quotes, verdict });
      saved = `Saved at ${new Date().toLocaleTimeString()}`;
      await reload();
    } catch (error) {
      problem = sentence(error);
    }
  }

  function addQuote() {
    const first = view?.documents[0];

    if (first !== undefined) {
      quotes = [...quotes, { document: first.id, text: "" }];
    }
  }

  onMount(reload);
</script>

<p class="navigation">
  <button type="button" class="ghost" onclick={onback}>← Work units</button>
  <span>
    <button type="button" class="ghost" disabled={place <= 0} onclick={() => onopen(order[place - 1] ?? unit)}
      >← Previous</button
    >
    {place + 1} of {order.length}
    <button
      type="button"
      class="ghost"
      disabled={place < 0 || place + 1 >= order.length}
      onclick={() => onopen(order[place + 1] ?? unit)}>Next →</button
    >
  </span>
</p>

{#if problem && view === null}<p class="problem">{problem}</p>{/if}

{#if view !== null}
  <section class="card">
    <h2>{view.label}</h2>
    <p class="note">{view.standard} · assessed by {view.agent}</p>
    <h3>Requirement</h3>
    <p class="text">{view.requirement}</p>
    {#if view.guidance.trim() !== ""}
      <details>
        <summary>Guidance</summary>
        <p class="text">{view.guidance}</p>
      </details>
    {/if}
    <dl class="needs">
      <dt>Depends on</dt>
      <dd>
        {#each view.dependencies as one, index (one.id)}{index > 0 ? ", " : ""}{one.name} ({one.detail ?? "not assessed"}){:else}nothing{/each}
      </dd>
      <dt>Documents</dt>
      <dd>
        {#each view.documents as one, index (one.id)}{index > 0 ? ", " : ""}{one.name} {one.detail ?? "(not given)"}{:else}none{/each}
      </dd>
      <dt>Proofs</dt>
      <dd>{view.proofs === null ? "none: a work unit of the documentation" : view.proofs.join(", ")}</dd>
    </dl>
  </section>

  <section class="card">
    <h2>
      Results
      <span class="note">· {view.author === "user" ? "written by the evaluator" : view.author === "ai" ? "proposed by the AI" : "not assessed"}</span>
    </h2>
    <form onsubmit={save}>
      <label>
        Verdict
        <select bind:value={verdict}>
          <option value="">— not assessed —</option>
          {#each view.verdicts as one (one)}
            <option value={one}>{one}</option>
          {/each}
        </select>
      </label>
      <label>Findings <textarea bind:value={findings} rows="4"></textarea></label>
      <label>Analysis <textarea bind:value={analysis} rows="6"></textarea></label>
      <label>Vulnerability notes <textarea bind:value={vulnerabilityNotes} rows="3"></textarea></label>
      <label>Notes <textarea bind:value={notes} rows="3"></textarea></label>

      <h3>Quotes</h3>
      <p class="note">Each word for word from its document; one that is not found there is refused.</p>
      {#each quotes as quote, index (index)}
        <div class="quote">
          <select bind:value={quote.document}>
            {#each view.documents as one (one.id)}
              <option value={one.id}>{one.name}</option>
            {/each}
          </select>
          <textarea bind:value={quote.text} rows="2"></textarea>
          <button type="button" class="ghost" onclick={() => (quotes = quotes.filter((_, at) => at !== index))}
            >Remove</button
          >
        </div>
      {:else}
        <p class="note">No quote.</p>
      {/each}
      {#if view.documents.length > 0}
        <p><button type="button" class="ghost" onclick={addQuote}>+ A quote</button></p>
      {/if}

      <div class="buttons">
        <button type="submit">Save</button>
        {#if saved}<span class="note">{saved}</span>{/if}
      </div>
      {#if problem}<p class="problem">{problem}</p>{/if}
    </form>
  </section>

  <section class="card">
    <h2>The question and the answer of the AI</h2>
    {#if view.journal === null}
      <p class="note">The AI was not asked about this work unit yet.</p>
    {:else}
      <p class="note">
        {new Date(view.journal.time * 1000).toLocaleString()} · {view.journal.model} · {view.journal.ending}
      </p>
      <details>
        <summary>The question, as it was sent</summary>
        <pre>{view.journal.request}</pre>
      </details>
      <details>
        <summary>The answer</summary>
        <pre>{view.journal.answer}</pre>
      </details>
    {/if}
  </section>
{/if}

<style>
  p.navigation {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  p.text {
    white-space: pre-wrap;
  }

  dl.needs {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.3rem 1rem;
  }

  dl.needs dt {
    color: var(--text-muted);
  }

  dl.needs dd {
    margin: 0;
  }

  .quote {
    display: grid;
    grid-template-columns: 10rem 1fr auto;
    gap: 0.5rem;
    align-items: start;
    margin-bottom: 0.5rem;
  }

  .quote select,
  .quote textarea {
    margin: 0;
  }

  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 24rem;
    overflow-y: auto;
  }
</style>
