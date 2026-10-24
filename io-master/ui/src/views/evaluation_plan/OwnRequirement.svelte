<!--
  The element "requirement of the laboratory" of the view Evaluation plan (its
  commands own_requirement_add and own_requirement_save in
  src/views/evaluation_plan/evaluation_plan.rs): a dialog for a requirement
  of the laboratory's own, outside the standards: its identifier (written by
  the planner as they choose), its title and its statement. A new one
  becomes a task of the project; a changed one changes for every project.
  When it is done it tells the browser (ondone).
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";

  let {
    project,
    editing,
    ondone,
    onclose,
  }: {
    project: string;
    /** The requirement changed (its id and fields), or null for a new one. */
    editing: { id: string; fields: evaluationPlan.OwnRequirement } | null;
    ondone: () => void;
    onclose: () => void;
  } = $props();

  let fields: evaluationPlan.OwnRequirement = $state({ identifier: "", title: "", statement: "" });
  let problem = $state("");

  $effect(() => {
    fields = editing ? { ...editing.fields } : { identifier: "", title: "", statement: "" };
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();

    try {
      if (editing) {
        await evaluationPlan.ownRequirementSave(editing.id, { ...fields });
      } else {
        await evaluationPlan.ownRequirementAdd(project, { ...fields });
      }
      ondone();
    } catch (error) {
      problem = `The requirement was not saved: ${sentence(error)}`;
    }
  }
</script>

<div class="overlay" role="presentation">
  <form class="dialog" onsubmit={submit}>
    <h2>{editing ? "A requirement of the laboratory" : "A requirement of your own"}</h2>
    <p class="note">
      {editing
        ? "It is changed for every project whose task points to it."
        : "Outside the standards. It is kept in the shared catalog of the laboratory, becomes a task of this project, and can serve other projects."}
    </p>
    {#if problem}<p class="problem">{problem}</p>{/if}
    <label>Identifier <input bind:value={fields.identifier} required readonly={editing !== null} /></label>
    <label>Title <input bind:value={fields.title} required /></label>
    <label>Statement <textarea bind:value={fields.statement} rows="6" required></textarea></label>
    <div class="buttons">
      <button type="submit">{editing ? "Save" : "Add"}</button>
      <button type="button" onclick={onclose}>Cancel</button>
    </div>
  </form>
</div>
