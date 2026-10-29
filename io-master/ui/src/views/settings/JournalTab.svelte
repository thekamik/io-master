<!--
  The tab Journal of the settings of the AI (its commands journal_list and
  journal_get in src/panels/views/settings/journal.rs): every question the
  program asked an AI service, the newest first, 50 at a time ("Older"
  reads the next page). An entry opened shows its request as it was sent
  and its answer as it came back. The journal is only read: no entry is
  changed or removed.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import { journalGet, journalList, type JournalEntryView, type JournalRowView } from "../../lib/api/settings";

  /** As many entries as one page of the journal (db/ai/journal.rs, PAGE). */
  const PAGE = 50;

  let rows: JournalRowView[] = $state([]);
  let more = $state(false);
  let open: JournalEntryView | null = $state(null);
  let problem = $state("");

  async function load(before: number | null) {
    problem = "";

    try {
      const page = await journalList(before);
      rows = before === null ? page : [...rows, ...page];
      more = page.length === PAGE;
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function show(id: number) {
    problem = "";

    try {
      open = await journalGet(id);
    } catch (error) {
      problem = sentence(error);
    }
  }

  /** The request as it was sent, indented to be read; as it is when it is
   * no JSON. */
  function readable(request: string): string {
    try {
      return JSON.stringify(JSON.parse(request), null, 2);
    } catch {
      return request;
    }
  }

  function when(time: number): string {
    return new Date(time * 1000).toLocaleString();
  }

  onMount(() => load(null));
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if open}
  <section class="card">
    <h2>Entry {open.row.id}</h2>
    <dl class="facts">
      <dt>Asked</dt><dd>{when(open.row.time)}</dd>
      <dt>Integration</dt><dd>{open.row.integration}</dd>
      <dt>Model</dt><dd class="mono">{open.row.model}</dd>
      {#if open.row.agent}<dt>Agent</dt><dd>{open.row.agent}</dd>{/if}
      {#if open.row.workUnit !== null}<dt>Work unit</dt><dd>{open.row.workUnit}</dd>{/if}
      <dt>Ending</dt><dd>{open.row.ending}</dd>
      <dt>Tokens</dt><dd>{open.row.inputTokens} in, {open.row.outputTokens} out</dd>
    </dl>
    <h3>Request</h3>
    <pre class="text">{readable(open.request)}</pre>
    <h3>Answer</h3>
    <pre class="text">{open.answer}</pre>
    <p><button type="button" class="ghost" onclick={() => (open = null)}>Close</button></p>
  </section>
{/if}

<section class="card">
  <h2>Journal</h2>
  <table class="data">
    <thead><tr><th>Asked</th><th>Integration</th><th>Model</th><th>Ending</th><th>Tokens</th><th></th></tr></thead>
    <tbody>
      {#each rows as row (row.id)}
        <tr>
          <td>{when(row.time)}</td>
          <td class="wrap">{row.integration}{row.agent ? ` · ${row.agent}` : ""}</td>
          <td class="wrap mono">{row.model}</td>
          <td>
            <span class="badge" class:success={row.ending === "complete"} class:danger={row.ending === "failed"}
              class:warning={row.ending !== "complete" && row.ending !== "failed"}>{row.ending}</span>
          </td>
          <td>{row.inputTokens} / {row.outputTokens}</td>
          <td><button type="button" onclick={() => show(row.id)}>Open</button></td>
        </tr>
      {:else}
        <tr><td colspan="6" class="note">No question asked yet.</td></tr>
      {/each}
    </tbody>
  </table>
  {#if more}
    <p><button type="button" onclick={() => load(rows[rows.length - 1]?.id ?? null)}>Older</button></p>
  {/if}
</section>

<style>
  h3 {
    margin-top: 1.25rem;
  }

  table.data {
    width: 100%;
  }

  pre.text {
    max-height: 24rem;
    overflow: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 0.85rem;
  }
</style>
