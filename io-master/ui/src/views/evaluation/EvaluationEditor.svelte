<!--
  One evaluation in the panel of the evaluation (its commands
  evaluation_get, evaluation_set_agent, evaluation_document_give and
  evaluation_proof_give in src/panels/views/evaluation/): its
  identification, its agent, and its inputs: the documents of the product
  (a file chosen in the dialog of the system, with its version) and the
  proofs of the laboratory (the text with the record of the test). Each is
  given once and never changed (user, 2026-10-30).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import {
    evaluationDocumentGive,
    evaluationGet,
    evaluationProofGive,
    evaluationSetAgent,
    type EvaluationView,
  } from "../../lib/api/evaluation";

  let { evaluation, onback }: { evaluation: number; onback: () => void } = $props();

  /** A proof as its form holds it before it is given. */
  interface ProofForm {
    text: string;
    /** The day of the test, as the field of a date holds it (YYYY-MM-DD). */
    day: string;
    tester: string;
    equipment: string;
  }

  let view: EvaluationView | null = $state(null);
  let agent: number | null = $state(null);
  let versions: Record<number, string> = $state({});
  let proofForms: Record<number, ProofForm> = $state({});
  let problem = $state("");
  let busy = $state(false);

  async function reload() {
    try {
      view = await evaluationGet(evaluation);
      agent = view.aiAgent;

      for (const proof of view.proofs) {
        proofForms[proof.id] ??= { text: "", day: "", tester: view.summary.evaluator, equipment: "" };
      }
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function changeAgent() {
    if (agent === null) {
      return;
    }

    problem = "";

    try {
      await evaluationSetAgent(evaluation, agent);
    } catch (error) {
      problem = sentence(error);
      await reload();
    }
  }

  async function giveDocument(document: number) {
    problem = "";
    busy = true;

    try {
      if (await evaluationDocumentGive(evaluation, document, versions[document] ?? "")) {
        await reload();
      }
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  async function giveProof(event: SubmitEvent, proof: number) {
    event.preventDefault();

    const form = proofForms[proof];

    if (form === undefined) {
      return;
    }

    problem = "";
    busy = true;

    try {
      const performed = Math.floor(Date.parse(`${form.day}T00:00:00Z`) / 1000);

      await evaluationProofGive(evaluation, proof, form.text, performed, form.tester, form.equipment);
      await reload();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  function date(seconds: number | null): string {
    return seconds === null ? "" : new Date(seconds * 1000).toISOString().slice(0, 10);
  }

  function size(bytes: number): string {
    return bytes < 1024 * 1024 ? `${Math.ceil(bytes / 1024)} KiB` : `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
  }

  onMount(reload);
</script>

<p><button type="button" class="ghost" onclick={onback}>← Evaluations</button></p>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if view !== null}
  <section class="card">
    <h2>{view.summary.product}</h2>
    <dl class="identification">
      <dt>Client</dt><dd>{view.summary.client}</dd>
      <dt>Evaluator</dt><dd>{view.summary.evaluator}</dd>
      <dt>Plan</dt><dd>{view.summary.name} · {view.summary.workUnits} work units</dd>
      <dt>Begun</dt><dd>{date(view.summary.started)}</dd>
      <dt>State</dt><dd>{view.summary.state}</dd>
    </dl>
    <label>
      Agent of the evaluation
      <span class="note">It assesses every work unit without an agent of its own.</span>
      <select bind:value={agent} onchange={changeAgent} disabled={view.summary.state !== "evaluation"}>
        {#each view.agents as one (one.id)}
          <option value={one.id}>{one.name}</option>
        {/each}
      </select>
    </label>
  </section>

  <section class="card">
    <h2>Documents of the product</h2>
    <p class="note">A document is given once, as its file (.pdf, .txt or .md), and never changed: a wrong file is mended by a new evaluation.</p>
    <table class="data">
      <thead><tr><th>Document</th><th>Needed by</th><th>Version</th><th></th></tr></thead>
      <tbody>
        {#each view.documents as document (document.id)}
          <tr>
            <td class="wrap">{document.name}</td>
            <td>{document.neededBy} work units</td>
            {#if document.version !== null}
              <td>{document.version}</td>
              <td class="note">given · {size(document.bytes)}</td>
            {:else}
              <td><input bind:value={versions[document.id]} placeholder="1.0" /></td>
              <td>
                <button
                  type="button"
                  disabled={busy || (versions[document.id] ?? "").trim() === ""}
                  onclick={() => giveDocument(document.id)}>Give the file</button
                >
              </td>
            {/if}
          </tr>
        {:else}
          <tr><td colspan="4" class="note">The plan reserved no document.</td></tr>
        {/each}
      </tbody>
    </table>
  </section>

  <section class="card">
    <h2>Proofs of the laboratory</h2>
    <p class="note">The result of a functional test, as text, with the record of the test; given once and never changed.</p>
    {#each view.proofs as proof (proof.id)}
      {@const form = proofForms[proof.id]}
      <article class="proof">
        <h3>{proof.name} <span class="note">· needed by {proof.neededBy} work units</span></h3>
        {#if proof.proof !== null}
          <p class="note">Performed {date(proof.performed)} by {proof.tester} with {proof.equipment}</p>
          <pre>{proof.proof}</pre>
        {:else if form !== undefined}
          <form onsubmit={(event) => giveProof(event, proof.id)}>
            <label>The result <textarea bind:value={form.text} rows="5" required></textarea></label>
            <div class="record">
              <label>Performed <input type="date" bind:value={form.day} required /></label>
              <label>Tester <input bind:value={form.tester} required /></label>
              <label>Equipment <input bind:value={form.equipment} required placeholder="nmap 7.95, LAB-014" /></label>
            </div>
            <button type="submit" disabled={busy}>Give the proof</button>
          </form>
        {/if}
      </article>
    {:else}
      <p class="note">No work unit of the plan needs a proof of the laboratory.</p>
    {/each}
  </section>
{/if}

<style>
  table.data {
    width: 100%;
  }

  dl.identification {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.3rem 1rem;
    margin: 0 0 1rem;
  }

  dl.identification dt {
    color: var(--text-muted);
  }

  dl.identification dd {
    margin: 0;
  }

  article.proof + article.proof {
    margin-top: 1.25rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
  }

  article.proof pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 16rem;
    overflow-y: auto;
  }

  .record {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0 1rem;
  }
</style>
