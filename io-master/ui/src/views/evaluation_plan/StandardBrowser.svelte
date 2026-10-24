<!--
  The element "browser of the standards" of the view Evaluation plan (its
  commands standard_browser_get, standard_browser_search and task_list_add in
  src/views/evaluation_plan/evaluation_plan.rs): a catalog of the project as
  a tree (the component CatalogTree), a search through all its catalogs, and
  the requirement chosen with its parts and quotes; "Add as a task" makes the
  whole requirement a task. After it tells the project (onchanged); a quote
  is shown in the viewer of the project (onview).
-->
<script lang="ts">
  import CatalogTree from "../../components/catalog_tree/CatalogTree.svelte";
  import PartRows from "../../components/part_rows/PartRows.svelte";
  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";

  let {
    project,
    catalogs,
    version,
    onchanged,
    onview,
  }: {
    project: string;
    catalogs: evaluationPlan.CatalogChoiceView[];
    version: number;
    onchanged: () => void;
    onview: (documentId: number, page: number, quote: string) => void;
  } = $props();

  let catalog = $state("");
  let chosen: { catalog: string; id: string } | null = $state(null);
  let requirement: evaluationPlan.RequirementView | null = $state(null);
  let text = $state("");
  let hits: evaluationPlan.HitView[] | null = $state(null);
  let problem = $state("");

  // The first catalog of the project, until another is chosen.
  $effect(() => {
    if (!catalogs.some((choice) => choice.uuid === catalog)) {
      catalog = catalogs[0]?.uuid ?? "";
    }
  });

  // The requirement chosen, read again when the project changed (version).
  $effect(() => {
    void version;
    if (chosen) {
      read(chosen.catalog, chosen.id);
    }
  });

  async function read(catalogId: string, id: string) {
    try {
      requirement = await evaluationPlan.standardBrowserGet(project, catalogId, id);
      problem = "";
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function search(event: SubmitEvent) {
    event.preventDefault();

    try {
      hits = await evaluationPlan.standardBrowserSearch(project, text);
      problem = "";
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function add() {
    if (!requirement) {
      return;
    }

    try {
      await evaluationPlan.taskListAdd(project, requirement.catalog_uuid, requirement.id);
      onchanged();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="card standard-browser">
  <h3>The standards</h3>
  {#if problem}<p class="problem">{problem}</p>{/if}
  <div class="browser-head">
    <label>
      Catalog
      <select bind:value={catalog}>
        {#each catalogs as choice (choice.uuid)}<option value={choice.uuid}>{choice.title}</option>{/each}
      </select>
    </label>
    <form class="search" onsubmit={search}>
      <input bind:value={text} placeholder="Search the requirements" />
      <button type="submit">Search</button>
    </form>
  </div>

  {#if hits}
    <div class="hits">
      <p class="note">
        {hits.length} found. <button type="button" class="link" onclick={() => (hits = null)}>Close</button>
      </p>
      {#each hits as hit (hit.catalog_uuid + hit.id)}
        <button type="button" class="link hit" onclick={() => (chosen = { catalog: hit.catalog_uuid, id: hit.id })}>
          <span class="object-id">{hit.label}</span> {hit.title} <span class="note">{hit.catalog_title}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="browser-body">
    {#if catalog}
      <CatalogTree
        uuid={catalog}
        {version}
        selected={chosen?.id ?? null}
        onchoose={(id, kind) => {
          if (kind === "control") {
            chosen = { catalog, id };
          }
        }}
        onread={() => {}}
      />
    {/if}

    <div class="requirement">
      {#if requirement}
        <h4><span class="object-id">{requirement.label}</span> {requirement.title}</h4>
        <button type="button" class="primary" disabled={requirement.in_project} onclick={add}>
          {requirement.in_project ? "A task already" : "Add as a task"}
        </button>
        <PartRows rows={requirement.parts} />
        {#each requirement.quotes as quote, index (index)}
          <blockquote class:missing={!quote.found}>
            {quote.text}
            <button type="button" class="small" onclick={() => onview(quote.document_id, quote.page, quote.text)}>
              Page {quote.page}
            </button>
          </blockquote>
        {/each}
      {:else}
        <p class="note">Choose a requirement in the tree or among the hits.</p>
      {/if}
    </div>
  </div>
</div>
