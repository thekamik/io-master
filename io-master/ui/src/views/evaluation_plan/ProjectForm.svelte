<!--
  The element "project" of the view Evaluation plan (its commands
  project_form_get and project_form_save in
  src/views/evaluation_plan/evaluation_plan.rs): the title, the scope and the
  catalogs of a master project, shown folded; a catalog a task still points
  to cannot leave it. It reads them again when the project changed (version). After a save it tells the project (onchanged), which
  reads its elements again.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";
  import AgentPicker from "../../components/knowledge_agent/AgentPicker.svelte";

  let {
    uuid,
    version,
    onread,
    onchanged,
  }: {
    uuid: string;
    version: number;
    onread: (project: evaluationPlan.ProjectView) => void;
    onchanged: () => void;
  } = $props();

  let project: evaluationPlan.ProjectView | null = $state(null);
  let form: evaluationPlan.ProjectForm = $state({ title: "", description: "", catalogs: [] });
  let open = $state(false);
  let notice: { done: boolean; text: string } | null = $state(null);

  // Read again when the project changed (version): a requirement of the
  // laboratory may have added its catalog.
  $effect(() => {
    void version;
    load(uuid);
  });

  async function load(id: string) {
    try {
      show(await evaluationPlan.projectFormGet(id));
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }

  function show(view: evaluationPlan.ProjectView) {
    project = view;
    form = { title: view.title, description: view.description, catalogs: view.catalogs.map((catalog) => catalog.uuid) };
    onread(view);
  }

  async function setAgent(agent: number | null) {
    if (!project) return;

    try {
      show(await evaluationPlan.projectFormAgent(project.uuid, agent));
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();

    try {
      show(await evaluationPlan.projectFormSave(uuid, { ...form, catalogs: [...form.catalogs] }));
      notice = { done: true, text: "The project was saved." };
      onchanged();
    } catch (error) {
      notice = { done: false, text: `The project was not saved: ${sentence(error)}` };
    }
  }
</script>

{#if project}
  <details class="card project-form" bind:open>
    <summary>Title, scope and catalogs</summary>
    {#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}
    <form onsubmit={save}>
      <label>Title <input bind:value={form.title} required /></label>
      <label>Scope <textarea bind:value={form.description} rows="3"></textarea></label>
      <fieldset>
        <legend>Catalogs</legend>
        {#each project.available as catalog (catalog.uuid)}
          <label class="choice">
            <input type="checkbox" value={catalog.uuid} bind:group={form.catalogs} />
            {catalog.title} ({catalog.version})
          </label>
        {/each}
      </fieldset>
      <button type="submit">Save</button>
    </form>
    <AgentPicker value={project.agent} label="Agent with knowledge for the works of the AI" onchange={setAgent} />
  </details>
{/if}
