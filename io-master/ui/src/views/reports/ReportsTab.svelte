<!--
  The tab Reports of the panel Reports (report_list in
  src/panels/views/reports/report.rs): every evaluation and report, as the
  panel Evaluation lists them; one opened shows its report
  (EvaluationReport).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import { reportList, type ReportSummaryView } from "../../lib/api/reports";

  let { onopen }: { onopen: (id: number) => void } = $props();

  let evaluations: ReportSummaryView[] = $state([]);
  let problem = $state("");

  async function reload() {
    try {
      evaluations = await reportList();
    } catch (error) {
      problem = sentence(error);
    }
  }

  function date(seconds: number | null): string {
    return seconds === null ? "" : new Date(seconds * 1000).toISOString().slice(0, 10);
  }

  onMount(reload);
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

<section class="card">
  <h2>Evaluations</h2>
  <table class="data">
    <thead>
      <tr><th>Product</th><th>Client</th><th>Plan</th><th>Begun</th><th>State</th><th></th></tr>
    </thead>
    <tbody>
      {#each evaluations as one (one.id)}
        <tr>
          <td class="wrap">{one.product}</td>
          <td class="wrap">{one.client}</td>
          <td class="wrap">{one.name}</td>
          <td>{date(one.started)}</td>
          <td>{one.state}</td>
          <td><button type="button" onclick={() => onopen(one.id)}>Open</button></td>
        </tr>
      {:else}
        <tr><td colspan="6" class="note">No evaluation yet: begin one in the panel Evaluation.</td></tr>
      {/each}
    </tbody>
  </table>
</section>

<style>
  table.data {
    width: 100%;
  }
</style>
