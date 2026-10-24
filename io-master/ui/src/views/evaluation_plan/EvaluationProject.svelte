<!--
  The screen of a master project of the view Evaluation plan: its title,
  scope and catalogs (ProjectForm, folded), its tasks (TaskList) beside the
  browser of the standards (StandardBrowser), and the viewer of a standard
  for a quote. When an element changes the project, the screen counts up
  `version`, and every element reads its data again. Presentation only.
-->
<script lang="ts">
  import PdfViewer from "../../components/pdf_viewer/PdfViewer.svelte";
  import type { ProjectView } from "../../lib/api/evaluation_plan";
  import ProjectForm from "./ProjectForm.svelte";
  import StandardBrowser from "./StandardBrowser.svelte";
  import TaskList from "./TaskList.svelte";

  let { uuid, onclose }: { uuid: string; onclose: () => void } = $props();

  let project: ProjectView | null = $state(null);
  let version = $state(0);
  let viewer: { documentId: number; page: number; quote: string } | null = $state(null);

  function changed() {
    version += 1;
  }
</script>

<div class="editor-head">
  <button type="button" class="small" onclick={onclose}>← All master projects</button>
  {#if project}<h2>{project.title}</h2>{/if}
</div>

<ProjectForm {uuid} onread={(view) => (project = view)} onchanged={changed} />

<div class="project-body">
  <TaskList project={uuid} {version} onchanged={changed} />
  {#if project}
    <StandardBrowser
      project={uuid}
      catalogs={project.catalogs}
      {version}
      onchanged={changed}
      onview={(documentId, page, quote) => (viewer = { documentId, page, quote })}
    />
  {/if}
</div>

{#if viewer}
  <PdfViewer documentId={viewer.documentId} page={viewer.page} quote={viewer.quote} onclose={() => (viewer = null)} />
{/if}
