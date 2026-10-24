<!--
  The element "new master project" of the view Evaluation plan (its commands
  new_project_catalogs and new_project_create in
  src/views/evaluation_plan/evaluation_plan.rs): a dialog with the title, the
  scope and the catalogs (only the finished ones). When the project is made
  it tells the view (onmade), which opens it.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";

  let { onmade, onclose }: { onmade: (uuid: string) => void; onclose: () => void } = $props();

  let available: evaluationPlan.CatalogChoiceView[] = $state([]);
  let title = $state("");
  let description = $state("");
  let chosen: string[] = $state([]);
  let problem = $state("");

  onMount(async () => {
    try {
      available = await evaluationPlan.newProjectCatalogs();
    } catch (error) {
      problem = sentence(error);
    }
  });

  async function make(event: SubmitEvent) {
    event.preventDefault();

    try {
      const made = await evaluationPlan.newProjectCreate({ title, description, catalogs: chosen });
      onmade(made.uuid);
    } catch (error) {
      problem = `The project was not made: ${sentence(error)}`;
    }
  }
</script>

<div class="overlay" role="presentation">
  <form class="dialog" onsubmit={make}>
    <h2>A new master project</h2>
    <p class="note">One for a version of a standard: its catalogs and, later, all its tasks.</p>
    {#if problem}<p class="problem">{problem}</p>{/if}
    <label>Title <input bind:value={title} required placeholder="Common Criteria CC:2022 Revision 1" /></label>
    <label>Scope <textarea bind:value={description} rows="3" placeholder="What the project covers"></textarea></label>
    <fieldset>
      <legend>Catalogs</legend>
      {#each available as catalog (catalog.uuid)}
        <label class="choice">
          <input type="checkbox" value={catalog.uuid} bind:group={chosen} />
          {catalog.title} ({catalog.version})
        </label>
      {:else}
        <p class="note">No finished catalog yet: convert one in the view Converter, or import one.</p>
      {/each}
    </fieldset>
    <div class="buttons">
      <button type="submit">Make</button>
      <button type="button" onclick={onclose}>Cancel</button>
    </div>
  </form>
</div>
