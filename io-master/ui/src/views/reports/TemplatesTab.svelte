<!--
  The tab Templates of the panel Reports (report_template_list,
  report_template_create and report_template_import in
  src/panels/views/reports/templates.rs): every template, a new one (it
  starts from the default template of the program) and one imported from a
  file of text chosen in the dialog of the system; a template opened shows
  its editor (TemplateEditor). A template is never removed.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import {
    reportTemplateCreate,
    reportTemplateImport,
    reportTemplateList,
    type TemplateSummaryView,
  } from "../../lib/api/reports";

  let { onopen }: { onopen: (id: number) => void } = $props();

  let templates: TemplateSummaryView[] = $state([]);
  let name = $state("");
  let problem = $state("");
  let busy = $state(false);

  async function reload() {
    try {
      templates = await reportTemplateList();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      onopen(await reportTemplateCreate(name));
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  async function importFile() {
    problem = "";
    busy = true;

    try {
      const id = await reportTemplateImport();

      if (id !== null) {
        onopen(id);
      }
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  onMount(reload);
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

<section class="card">
  <h2>Templates</h2>
  <table class="data">
    <thead>
      <tr><th>Name</th><th>Extension</th><th></th></tr>
    </thead>
    <tbody>
      {#each templates as one (one.id)}
        <tr>
          <td class="wrap">{one.name}</td>
          <td>.{one.extension}</td>
          <td><button type="button" onclick={() => onopen(one.id)}>Open</button></td>
        </tr>
      {:else}
        <tr><td colspan="3" class="note">No template yet: make the first one below.</td></tr>
      {/each}
    </tbody>
  </table>
</section>

<section class="card">
  <h2>A new template</h2>
  <form onsubmit={create}>
    <label>
      Name
      <span class="note">It starts from the default template of the program, a report in Markdown.</span>
      <input bind:value={name} required placeholder="ETR after the CEM" />
    </label>
    <button type="submit" disabled={busy}>Make the template</button>
  </form>
  <p class="note">Or a template written elsewhere, any file of text (Markdown, HTML, LaTeX):</p>
  <button type="button" disabled={busy} onclick={importFile}>Import from a file</button>
</section>

<style>
  table.data {
    width: 100%;
  }
</style>
