<!--
  The view Evaluation plan (src/views/evaluation_plan/): it works only on the
  documents of NIST OSCAL, in two tabs:

  - Projects: the master projects (step 51 of docs/history.md), each a list of
    tasks, every task a whole requirement chosen in a browser of the
    standards; a project opens in its screen (EvaluationProject);
  - OSCAL documents: the catalogs, mappings, profiles and plans, imported,
    exported and deleted.

  The catalogs are made from the PDFs of a standard in the view Converter.
  The view keeps only what is presentation: the tab, the project open, the
  dialog of a new project.
-->
<script lang="ts">
  import EvaluationProject from "./EvaluationProject.svelte";
  import NewProject from "./NewProject.svelte";
  import OscalDocuments from "./OscalDocuments.svelte";
  import ProjectList from "./ProjectList.svelte";

  type Tab = "projects" | "documents";

  let tab: Tab = $state("projects");
  let project: string | null = $state(null);
  let making = $state(false);

  function made(uuid: string) {
    making = false;
    project = uuid;
  }
</script>

<div class="tabs">
  <button type="button" class:active={tab === "projects"} onclick={() => (tab = "projects")}>Master projects</button>
  <button type="button" class:active={tab === "documents"} onclick={() => (tab = "documents")}>OSCAL documents</button>
</div>

{#if tab === "projects" && project}
  <EvaluationProject uuid={project} onclose={() => (project = null)} />
{:else if tab === "projects"}
  <ProjectList onopen={(uuid) => (project = uuid)} onnew={() => (making = true)} />
{:else}
  <OscalDocuments />
{/if}

{#if making}
  <NewProject onmade={made} onclose={() => (making = false)} />
{/if}
