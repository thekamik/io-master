<!--
  The element "list of tasks" of the view Evaluation plan (its commands
  task_list_get, task_list_move and task_list_remove in
  src/views/evaluation_plan/evaluation_plan.rs): the tasks of a master
  project, each a whole requirement, in the order the planner gives them.
  It reads them again when the project says they changed (version); after
  its own change it tells the project (onchanged).
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";

  let {
    project,
    version,
    onchanged,
  }: { project: string; version: number; onchanged: () => void } = $props();

  let tasks: evaluationPlan.TaskView[] = $state([]);
  let problem = $state("");

  $effect(() => {
    void version;
    load(project);
  });

  async function load(id: string) {
    try {
      tasks = await evaluationPlan.taskListGet(id);
      problem = "";
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function change(command: () => Promise<evaluationPlan.TaskView[]>) {
    try {
      tasks = await command();
      problem = "";
      onchanged();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="card task-list">
  <h3>Tasks ({tasks.length})</h3>
  {#if problem}<p class="problem">{problem}</p>{/if}
  {#if tasks.length === 0}
    <p class="note">No task yet. Choose a requirement in the browser and add it as a task.</p>
  {/if}
  <ol>
    {#each tasks as task, index (task.uuid)}
      <li>
        <div class="task-text">
          <span class="object-id">{task.label}</span>
          {task.title || "(removed from its catalog)"}
          <span class="note">{task.catalog_title}</span>
        </div>
        <div class="task-actions">
          <button
            type="button"
            class="small"
            disabled={index === 0}
            onclick={() => change(() => evaluationPlan.taskListMove(task.uuid, true))}
            aria-label="Up">↑</button
          >
          <button
            type="button"
            class="small"
            disabled={index === tasks.length - 1}
            onclick={() => change(() => evaluationPlan.taskListMove(task.uuid, false))}
            aria-label="Down">↓</button
          >
          <button
            type="button"
            class="small danger"
            onclick={() => change(() => evaluationPlan.taskListRemove(task.uuid))}
            aria-label="Remove">✕</button
          >
        </div>
      </li>
    {/each}
  </ol>
</div>
