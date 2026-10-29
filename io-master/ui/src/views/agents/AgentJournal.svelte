<!--
  The journal of one agent (its commands agent_journal_list and
  agent_journal_get in src/panels/views/agents/journal.rs): every question
  the agent asked and its answer, the newest first, 50 at a time ("Older"
  reads the next page). An entry opened shows its request as it was sent
  and its answer as it came back. The journal is only read: no entry is
  changed or removed.
-->
<script lang="ts">
  import { agentJournalGet, agentJournalList, type JournalEntryView, type JournalRowView } from "../../lib/api/agents";
  import { sentence } from "../../lib/api/errors";

  let { agent }: { agent: number } = $props();

  /** As many entries as one page of the journal (db/ai/journal.rs, PAGE). */
  const PAGE = 50;

  let rows: JournalRowView[] = $state([]);
  let more = $state(false);
  let open: JournalEntryView | null = $state(null);
  let problem = $state("");

  async function load(id: number, before: number | null) {
    problem = "";

    try {
      const page = await agentJournalList(id, before);
      rows = before === null ? page : [...rows, ...page];
      more = page.length === PAGE;
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    open = null;
    void load(agent, null);
  });

  async function show(id: number) {
    problem = "";

    try {
      open = await agentJournalGet(id);
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
</script>

<section class="card">
  <h2>Journal</h2>
  {#if problem}<p class="problem">{problem}</p>{/if}

  {#if open}
    <dl class="facts">
      <dt>Asked</dt><dd>{when(open.row.time)}</dd>
      <dt>Integration</dt><dd>{open.row.integration}</dd>
      <dt>Model</dt><dd class="mono">{open.row.model}</dd>
      {#if open.row.workUnit !== null}<dt>Work unit</dt><dd>{open.row.workUnit}</dd>{/if}
      <dt>Ending</dt><dd>{open.row.ending}</dd>
      <dt>Tokens</dt><dd>{open.row.inputTokens} in, {open.row.outputTokens} out</dd>
    </dl>
    <h3>Request</h3>
    <pre class="text">{readable(open.request)}</pre>
    <h3>Answer</h3>
    <pre class="text">{open.answer}</pre>
    <p><button type="button" class="ghost" onclick={() => (open = null)}>Back to the journal</button></p>
  {:else}
    <table class="data">
      <thead><tr><th>Asked</th><th>Model</th><th>Work unit</th><th>Ending</th><th>Tokens</th><th></th></tr></thead>
      <tbody>
        {#each rows as row (row.id)}
          <tr>
            <td>{when(row.time)}</td>
            <td class="wrap mono">{row.model}</td>
            <td>{row.workUnit ?? "—"}</td>
            <td>
              <span
                class="badge"
                class:success={row.ending === "complete"}
                class:danger={row.ending === "failed"}
                class:warning={row.ending !== "complete" && row.ending !== "failed"}>{row.ending}</span
              >
            </td>
            <td>{row.inputTokens} / {row.outputTokens}</td>
            <td><button type="button" onclick={() => show(row.id)}>Open</button></td>
          </tr>
        {:else}
          <tr><td colspan="6" class="note">The agent has asked nothing yet.</td></tr>
        {/each}
      </tbody>
    </table>
    {#if more}
      <p><button type="button" onclick={() => load(agent, rows[rows.length - 1]?.id ?? null)}>Older</button></p>
    {/if}
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
