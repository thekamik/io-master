<!--
  The work units of an evaluation (its command evaluation_unit_list in
  src/panels/views/evaluation/work_units.rs), in the order of the work, with
  the counts of their verdicts, a filter by the label and by the verdict,
  and pages. Read again whenever `changes` grows (a step of the
  assessment). A work unit opened is told with the order of the work units
  shown (onopen), for "previous" and "next".
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import { evaluationUnitList, type UnitRowView } from "../../lib/api/evaluation";

  let {
    evaluation,
    changes,
    page = $bindable(),
    filter = $bindable(),
    verdict = $bindable(),
    onopen,
  }: {
    evaluation: number;
    changes: number;
    page: number;
    filter: string;
    verdict: string;
    onopen: (unit: number, order: number[]) => void;
  } = $props();

  /** The work units of one page. */
  const PAGE = 50;

  let rows: UnitRowView[] = $state([]);
  let problem = $state("");

  /** The rows with their number in the order of the work, as the filters leave them. */
  let shown = $derived.by(() => {
    const wanted = filter.trim().toLowerCase();
    const numbered = rows.map((row, index) => ({ row, number: index + 1 }));

    return numbered.filter(({ row }) => {
      const byLabel = wanted === "" || row.label.toLowerCase().includes(wanted);
      const byVerdict =
        verdict === "" ||
        (verdict === "none" && row.verdict === null) ||
        (verdict === "user" && row.author === "user") ||
        row.verdict === verdict;

      return byLabel && byVerdict;
    });
  });
  let pages = $derived(Math.max(1, Math.ceil(shown.length / PAGE)));
  let assessed = $derived(rows.filter((row) => row.verdict !== null).length);

  function counted(name: string): number {
    return rows.filter((row) => row.verdict === name).length;
  }

  async function reload() {
    try {
      rows = await evaluationUnitList(evaluation);
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    // Read again after every step of the assessment.
    void changes;
    reload();
  });

  $effect(() => {
    // A filter changed: back to a page that exists.
    if (page >= pages) {
      page = pages - 1;
    }
  });
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

<section class="card">
  <h2>Work units</h2>
  <p>
    {assessed} of {rows.length} assessed · Pass {counted("pass")} · Fail {counted("fail")} · Inconclusive
    {counted("inconclusive")}
  </p>

  <div class="filters">
    <input bind:value={filter} placeholder="Filter by the label: ADV_ARC" oninput={() => (page = 0)} />
    <select bind:value={verdict} onchange={() => (page = 0)}>
      <option value="">Every verdict</option>
      <option value="none">Not assessed</option>
      <option value="pass">Pass</option>
      <option value="fail">Fail</option>
      <option value="inconclusive">Inconclusive</option>
      <option value="user">Written by the evaluator</option>
    </select>
  </div>

  <table class="data">
    <thead><tr><th>#</th><th>Work unit</th><th>Kind</th><th>Agent</th><th>Verdict</th><th>Author</th></tr></thead>
    <tbody>
      {#each shown.slice(page * PAGE, (page + 1) * PAGE) as { row, number } (row.id)}
        <tr>
          <td>{number}</td>
          <td>
            <button
              type="button"
              class="link"
              onclick={() =>
                onopen(
                  row.id,
                  shown.map((one) => one.row.id),
                )}>{row.label}</button
            >
          </td>
          <td>{row.functional ? "Functional" : "Documentation"}</td>
          <td class="wrap">{row.agent}{row.general ? " (of the evaluation)" : ""}</td>
          <td>
            {#if row.verdict !== null}
              <span
                class="badge"
                class:success={row.verdict === "pass"}
                class:danger={row.verdict === "fail"}
                class:warning={row.verdict === "inconclusive"}>{row.verdict}</span
              >
            {:else}
              <span class="note">—</span>
            {/if}
          </td>
          <td>{row.author === "user" ? "Evaluator" : row.author === "ai" ? "AI" : ""}</td>
        </tr>
      {:else}
        <tr><td colspan="6" class="note">No work unit fits the filter.</td></tr>
      {/each}
    </tbody>
  </table>

  {#if pages > 1}
    <p class="pages">
      <button type="button" class="ghost" disabled={page === 0} onclick={() => (page -= 1)}>← Previous</button>
      {page + 1} of {pages}
      <button type="button" class="ghost" disabled={page + 1 >= pages} onclick={() => (page += 1)}>Next →</button>
    </p>
  {/if}
</section>

<style>
  table.data {
    width: 100%;
  }

  .filters {
    display: flex;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .filters input,
  .filters select {
    margin: 0;
  }

  .filters select {
    max-width: 16rem;
  }

  p.pages {
    display: flex;
    gap: 1rem;
    align-items: center;
    justify-content: center;
  }
</style>
