<!--
  The report of one evaluation (report_preview and report_export in
  src/panels/views/reports/report.rs, report_template_list in
  templates.rs): a template chosen, the report it writes shown as text, and
  "Export", which writes it into a file chosen in the dialog of the system.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import { reportExport, reportPreview, reportTemplateList, type TemplateSummaryView } from "../../lib/api/reports";

  let { evaluation, onback }: { evaluation: number; onback: () => void } = $props();

  let templates: TemplateSummaryView[] = $state([]);
  let template: number | null = $state(null);
  let report = $state("");
  let problem = $state("");
  let notice = $state("");
  let busy = $state(false);

  async function load() {
    try {
      templates = await reportTemplateList();
      template = templates[0]?.id ?? null;
    } catch (error) {
      problem = sentence(error);
    }
  }

  /** The report of the evaluation by the template chosen, read again
   * whenever another one is chosen. */
  async function preview(chosen: number | null) {
    problem = "";
    notice = "";
    report = "";

    if (chosen === null) {
      return;
    }

    try {
      report = await reportPreview(evaluation, chosen);
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    void preview(template);
  });

  async function exportReport() {
    if (template === null) {
      return;
    }

    problem = "";
    notice = "";
    busy = true;

    try {
      if (await reportExport(evaluation, template)) {
        notice = "The report is written.";
      }
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  onMount(load);
</script>

<p><button type="button" class="ghost" onclick={onback}>← Reports</button></p>

{#if problem}<p class="problem">{problem}</p>{/if}

<section class="card">
  <h2>The report</h2>
  {#if templates.length === 0}
    <p class="note">A report is written by a template: make one in the tab Templates.</p>
  {:else}
    <div class="row report-bar">
      <label>
        Template
        <select bind:value={template}>
          {#each templates as one (one.id)}
            <option value={one.id}>{one.name} (.{one.extension})</option>
          {/each}
        </select>
      </label>
      <button type="button" disabled={busy || template === null} onclick={exportReport}>Export</button>
    </div>
    {#if notice}<p class="note">{notice}</p>{/if}
    <textarea class="mono report-text" readonly value={report} aria-label="The report"></textarea>
  {/if}
</section>
