<!--
  The editor of a template (report_template_get, report_template_save,
  report_template_export and report_markers in
  src/panels/views/reports/templates.rs): its name, the extension of its
  reports and its text, with the markers beside it; a click on a marker
  puts it at the cursor, a click on a block puts its two tags around the
  cursor. "Save" keeps it (a block not closed or nested is refused, an
  unknown marker warned of); "Export to a file" writes it into a file chosen
  in the dialog of the system.
-->
<script lang="ts">
  import { onMount, tick } from "svelte";

  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
  import { sentence } from "../../lib/api/errors";
  import {
    reportMarkers,
    reportTemplateExport,
    reportTemplateGet,
    reportTemplateSave,
    type MarkerGroupView,
  } from "../../lib/api/reports";

  let { template, onback }: { template: number; onback: () => void } = $props();

  let name = $state("");
  let extension = $state("");
  let text = $state("");
  let warnings: string[] = $state([]);
  let groups: MarkerGroupView[] = $state([]);
  let editor: HTMLTextAreaElement | null = $state(null);
  let problem = $state("");
  let notice = $state("");
  let busy = $state(false);
  let savedAt: Date | null = $state(null);

  async function load(id: number) {
    problem = "";

    try {
      const view = await reportTemplateGet(id);

      name = view.name;
      extension = view.extension;
      text = view.text;
      warnings = view.warnings;
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    void load(template);
  });

  /** Puts `before` and `after` around the selection of the editor, and the
   * cursor between them (after `before` when nothing is selected). */
  async function insert(before: string, after: string) {
    const start = editor?.selectionStart ?? text.length;
    const end = editor?.selectionEnd ?? text.length;

    text = text.slice(0, start) + before + text.slice(start, end) + after + text.slice(end);
    await tick();
    editor?.focus();
    editor?.setSelectionRange(start + before.length, end + before.length);
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    notice = "";
    busy = true;

    try {
      warnings = await reportTemplateSave(template, name, extension, text);
      savedAt = new Date();
      // Read again: the program keeps the extension in lower case, without its dot.
      await load(template);
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  async function exportFile() {
    problem = "";
    notice = "";
    busy = true;

    try {
      if (await reportTemplateExport(template)) {
        notice = "The template as last saved is written.";
      }
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  onMount(async () => {
    try {
      groups = await reportMarkers();
    } catch (error) {
      problem = sentence(error);
    }
  });
</script>

<p><button type="button" class="ghost" onclick={onback}>← Templates</button></p>

{#if problem}<p class="problem">{problem}</p>{/if}

<section class="card">
  <h2>{name}</h2>
  {#each warnings as warning (warning)}
    <p class="note"><span class="badge warning">marker</span> {warning}</p>
  {/each}
  <form onsubmit={save}>
    <div class="row">
      <label>Name <input bind:value={name} required /></label>
      <label>
        Extension of the reports
        <input bind:value={extension} required placeholder="md" />
      </label>
    </div>
    <div class="template-editor">
      <label>
        Text
        <span class="note">Any text format; a marker of a field is replaced by its value, nothing else is changed.</span>
        <textarea class="mono template-text" bind:this={editor} bind:value={text} spellcheck="false"></textarea>
      </label>
      <aside class="markers" aria-label="Markers">
        {#each groups as group (group.opening)}
          <div class="marker-group">
            {#if group.opening}
              <button type="button" class="ghost block" onclick={() => insert(`${group.opening}\n`, `\n${group.closing}`)}>
                {group.opening}
              </button>
              <p class="note">Written for each element; inside it:</p>
            {:else}
              <p class="note">The evaluation, anywhere:</p>
            {/if}
            {#each group.markers as marker (marker)}
              <button type="button" class="ghost marker" onclick={() => insert(marker, "")}>{marker}</button>
            {/each}
          </div>
        {/each}
        <p class="note">A block of work units takes a filter: <code>verdict=fail</code> after its name (pass, fail, inconclusive, not_applicable, none).</p>
      </aside>
    </div>
    <SaveStatus {busy} {savedAt} />
    <div class="row">
      <button type="submit" disabled={busy}>Save</button>
      <button type="button" disabled={busy} onclick={exportFile}>Export to a file</button>
    </div>
    {#if notice}<p class="note">{notice}</p>{/if}
  </form>
</section>
