<!--
  The element "work packages" of the view Evaluation plan (its commands
  work_packages_* in src/views/evaluation_plan/evaluation_plan.rs, step
  51.5): the tasks of the master project delegated to the evaluators, shown
  folded. A package has a name, the evaluator it goes to, a note and its
  tasks; a task is in one package at most (a task in another package cannot
  be chosen). "Export…" writes the package into a file, chosen in a dialog
  of the system, for the evaluator. After a change the screen of the project
  is told (onchanged), so that the list of tasks shows the packages.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";
  import ConfirmButton from "../../components/confirm_button/ConfirmButton.svelte";

  let { project, version, onchanged }: { project: string; version: number; onchanged: () => void } = $props();

  let packages: evaluationPlan.WorkPackageView[] = $state([]);
  let tasks: evaluationPlan.TaskView[] = $state([]);
  /** The package in the form: its UUID, "new", or none. */
  let editing: string | null = $state(null);
  let form: evaluationPlan.WorkPackageForm = $state({ name: "", evaluator: "", note: "", tasks: [] });
  let notice: { done: boolean; text: string } | null = $state(null);

  // Read again when the project changed (version): tasks come and go.
  $effect(() => {
    void version;
    load(project);
  });

  async function load(id: string) {
    try {
      packages = await evaluationPlan.workPackagesGet(id);
      tasks = await evaluationPlan.taskListGet(id);
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }

  function edit(chosen: evaluationPlan.WorkPackageView | null) {
    editing = chosen ? chosen.uuid : "new";
    form = chosen
      ? { name: chosen.name, evaluator: chosen.evaluator, note: chosen.note, tasks: [...chosen.tasks] }
      : { name: "", evaluator: "", note: "", tasks: [] };
    notice = null;
  }

  /** The name of the package other than the one in the form that holds `task`. */
  function holder(task: string): string | null {
    const other = packages.find((candidate) => candidate.uuid !== editing && candidate.tasks.includes(task));

    return other ? other.name : null;
  }

  /** Runs a change, shows the packages the program gives back, tells the project. */
  async function act(work: () => Promise<evaluationPlan.WorkPackageView[]>, text: string) {
    try {
      packages = await work();
      editing = null;
      notice = { done: true, text };
      onchanged();
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }

  function save(event: SubmitEvent) {
    event.preventDefault();
    const written = { ...form, tasks: [...form.tasks] };
    const target = editing;

    act(
      () =>
        target === "new" || target === null
          ? evaluationPlan.workPackagesAdd(project, written)
          : evaluationPlan.workPackagesSave(target, written),
      "The package was saved.",
    );
  }

  async function exportPackage(chosen: evaluationPlan.WorkPackageView) {
    try {
      if (await evaluationPlan.workPackagesExport(chosen.uuid)) {
        notice = { done: true, text: `The package "${chosen.name}" was exported.` };
      }
    } catch (error) {
      notice = { done: false, text: `The package was not exported: ${sentence(error)}` };
    }
  }
</script>

<details class="card project-form">
  <summary>Work packages ({packages.length})</summary>
  <p class="note">The tasks delegated to the evaluators. A task is in one package at most; the export takes a picture of the tasks, their schemes and related elements as they are now, with the catalogs attached.</p>
  {#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}
  {#if packages.length > 0}
    <table class="data">
      <tbody>
        {#each packages as item (item.uuid)}
          <tr>
            <td>{item.name}</td>
            <td>{item.evaluator}</td>
            <td>{item.tasks.length} task(s)</td>
            <td class="actions">
              <button type="button" class="small" onclick={() => exportPackage(item)}>Export…</button>
              <button type="button" class="small" onclick={() => edit(item)}>Change</button>
              <ConfirmButton onconfirm={() => act(() => evaluationPlan.workPackagesDelete(item.uuid), "The package was deleted.")} />
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
  {#if editing}
    <form class="block-form" onsubmit={save}>
      <label>Name <input bind:value={form.name} required placeholder="ADV for the first week, …" /></label>
      <label>Evaluator <input bind:value={form.evaluator} placeholder="Who carries it out" /></label>
      <label>Note <textarea bind:value={form.note} rows="2"></textarea></label>
      <fieldset>
        <legend>Tasks</legend>
        {#each tasks as task (task.uuid)}
          {@const other = holder(task.uuid)}
          <label class="choice">
            <input type="checkbox" value={task.uuid} bind:group={form.tasks} disabled={other !== null} />
            <span class="object-id">{task.label}</span> {task.title}
            {#if other}<span class="note">(in "{other}")</span>{/if}
          </label>
        {/each}
      </fieldset>
      <div class="buttons">
        <button type="submit">{editing === "new" ? "Make" : "Save"}</button>
        <button type="button" onclick={() => (editing = null)}>Cancel</button>
      </div>
    </form>
  {:else}
    <button type="button" class="small" onclick={() => edit(null)}>+ Work package</button>
  {/if}
</details>
