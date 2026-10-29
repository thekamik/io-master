<!--
  The tab Instructions of the settings of the AI (its commands
  instructions_get and instructions_save in
  src/panels/views/settings/instructions.rs): the fixed part of every
  question of a task. A task the user has not changed has the default
  instructions of the program.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
  import { sentence } from "../../lib/api/errors";
  import { instructionsGet, instructionsSave, type InstructionsView, type TaskName } from "../../lib/api/settings";

  /** What each task is, as the panel names it. */
  const TITLES: Record<TaskName, { title: string; note: string }> = {
    assessment: { title: "Assessment", note: "The assessment of a work unit: its results, quotes and verdict." },
    consultation: { title: "Consultation", note: "A message of a chat." },
  };

  let tasks: InstructionsView[] = $state([]);
  let problem = $state("");
  let busy = $state(false);
  /** The task written last, whose card shows the time of the write. */
  let savedTask: TaskName | null = $state(null);
  let savedAt: Date | null = $state(null);

  async function reload() {
    try {
      tasks = await instructionsGet();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function save(event: SubmitEvent, task: InstructionsView) {
    event.preventDefault();
    problem = "";
    busy = true;
    savedTask = task.task;
    savedAt = null;

    try {
      await instructionsSave(task.task, task.text);
      savedAt = new Date();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  onMount(reload);
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

{#each tasks as task (task.task)}
  <section class="card">
    <h2>{TITLES[task.task].title}</h2>
    <form onsubmit={(event) => save(event, task)}>
      <label>
        Instructions
        <span class="note">{TITLES[task.task].note}</span>
        <textarea rows="7" bind:value={task.text} required></textarea>
      </label>
      <button type="submit" disabled={busy}>Save</button>
    </form>
    {#if savedTask === task.task}<SaveStatus {busy} {savedAt} />{/if}
  </section>
{/each}
