<!--
  The tab "AI journal" of the view Settings (its commands ai_journal_get and
  ai_journal_entry in src/views/settings/settings.rs, step 58): every
  question asked to the AI and its answer, the newest first, by task and
  words, page by page; an entry opened shows the request as it was sent
  (with the passages of an agent, when there was one) and the answer. The
  journal is only read here: it is written by the program at every
  question (P3). Which filter, page and entry are shown is presentation.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as settings from "../../lib/api/settings";

  let view: settings.JournalPageView | null = $state(null);
  let task = $state("");
  let text = $state("");
  let page = $state(0);
  let opened: settings.JournalEntryView | null = $state(null);
  let problem = $state("");

  onMount(() => load(0));

  async function load(wanted: number) {
    problem = "";

    try {
      view = await settings.aiJournalGet(task === "" ? null : task, text, wanted);
      page = wanted;
    } catch (error) {
      problem = sentence(error);
    }
  }

  function search(event: SubmitEvent) {
    event.preventDefault();
    load(0);
  }

  async function open(id: number) {
    try {
      opened = await settings.aiJournalEntry(id);
    } catch (error) {
      problem = sentence(error);
    }
  }

  /** The time of an entry in the time of this computer. */
  function when(seconds: number): string {
    return new Date(seconds * 1000).toLocaleString();
  }
</script>

<p class="note">Every question the program asked the AI, with what was sent and what came back, the newest first. The journal is written at every question and can only be read: it shows what a result of the AI rests on.</p>

<form class="buttons journal-filter" onsubmit={search}>
  <select bind:value={task} onchange={() => load(0)}>
    <option value="">all tasks</option>
    {#each view?.tasks ?? [] as name (name)}<option value={name}>{name}</option>{/each}
  </select>
  <input bind:value={text} placeholder="Words in the question or the answer" />
  <button type="submit">Search</button>
</form>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if opened}
  <div class="card">
    <div class="buttons"><button type="button" class="small" onclick={() => (opened = null)}>← The list</button></div>
    <h3>{opened.row.task} · {when(opened.row.time)}</h3>
    <p class="note">
      Role {opened.row.role || "none"} · connection {opened.row.connection} · model {opened.row.model} · {opened.row.ending}
      · {opened.row.input_tokens} tokens in, {opened.row.output_tokens} out
    </p>
    {#if opened.note}<p class="problem">{opened.note}</p>{/if}
    <h4>What was sent</h4>
    <div class="ai-proposal"><pre>{opened.request}</pre></div>
    <h4>What came back</h4>
    <div class="ai-proposal"><pre>{opened.answer}</pre></div>
  </div>
{:else if view}
  {#if view.rows.length === 0}
    <p class="note">No entry.</p>
  {:else}
    <table class="data">
      <thead><tr><th>When</th><th>Task</th><th>Model</th><th>Ending</th><th>Tokens</th><th></th></tr></thead>
      <tbody>
        {#each view.rows as row (row.id)}
          <tr>
            <td>{when(row.time)}</td>
            <td>{row.task}</td>
            <td>{row.model}</td>
            <td>{row.ending}</td>
            <td>{row.input_tokens} / {row.output_tokens}</td>
            <td class="actions"><button type="button" class="small" onclick={() => open(row.id)}>Open</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
    <div class="buttons">
      <button type="button" class="small" disabled={page === 0} onclick={() => load(page - 1)}>← Newer</button>
      <span class="note">
        {page * view.page_size + 1}–{page * view.page_size + view.rows.length} of {view.total}
      </span>
      <button type="button" class="small" disabled={(page + 1) * view.page_size >= view.total} onclick={() => load(page + 1)}>Older →</button>
    </div>
  {/if}
{/if}
