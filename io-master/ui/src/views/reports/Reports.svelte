<!--
  The panel Reports (its commands in src/panels/views/reports/): a report is
  a template filled with an evaluation (user, 2026-11-03). Two tabs: Reports
  (every evaluation, its report shown by a template and exported into a
  file) and Templates (a new one, one imported from a file, one opened in
  its editor, TemplateEditor). Which screen is shown is presentation only:
  it stays in the page.
-->
<script lang="ts">
  import EvaluationReport from "./EvaluationReport.svelte";
  import ReportsTab from "./ReportsTab.svelte";
  import TemplateEditor from "./TemplateEditor.svelte";
  import TemplatesTab from "./TemplatesTab.svelte";

  let tab: "reports" | "templates" = $state("reports");
  let openEvaluation: number | null = $state(null);
  let openTemplate: number | null = $state(null);
</script>

<div class="page-head">
  <h1>Reports</h1>
  <p>The report of an evaluation, written by a template with the markers of its fields.</p>
</div>

{#if openEvaluation !== null}
  <EvaluationReport evaluation={openEvaluation} onback={() => (openEvaluation = null)} />
{:else if openTemplate !== null}
  <TemplateEditor template={openTemplate} onback={() => (openTemplate = null)} />
{:else}
  <div class="tabs">
    <button type="button" class:active={tab === "reports"} onclick={() => (tab = "reports")}>Reports</button>
    <button type="button" class:active={tab === "templates"} onclick={() => (tab = "templates")}>Templates</button>
  </div>

  {#if tab === "reports"}
    <ReportsTab onopen={(id) => (openEvaluation = id)} />
  {:else}
    <TemplatesTab onopen={(id) => (openTemplate = id)} />
  {/if}
{/if}
