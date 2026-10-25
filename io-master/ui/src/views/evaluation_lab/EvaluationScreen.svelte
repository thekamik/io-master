<!--
  The element "screen of an evaluation" of the view Evaluation lab (its
  commands lab_tasks_get, lab_tasks_state, lab_tasks_agent and report_export
  in src/views/evaluation_lab/evaluation_lab.rs): what the package says of
  itself, the product, the agent with knowledge of its requirements (made in
  the view Knowledge (RAG)), the progress of the evaluation and of every
  task (its verdict, accepted fragments, outputs ready for the report; step
  59), the tasks with their state (to do, in progress, done), and "Export the report…" (one file per output, in a folder chosen
  in a dialog of the system). A task opens its screen (LabTask).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as evaluationLab from "../../lib/api/evaluation_lab";
  import AgentPicker from "../../components/knowledge_agent/AgentPicker.svelte";
  import LabTask from "./LabTask.svelte";

  let { evaluation, onclose }: { evaluation: string; onclose: () => void } = $props();

  const states = { todo: "to do", working: "in progress", done: "done" };
  const verdicts = { met: "met", not_met: "not met", inconclusive: "inconclusive" };

  let view: evaluationLab.EvaluationView | null = $state(null);
  /** The task whose screen is open. */
  let task: string | null = $state(null);
  let notice: { done: boolean; text: string } | null = $state(null);

  onMount(load);

  async function load() {
    try {
      view = await evaluationLab.labTasksGet(evaluation);
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }

  async function setState(uuid: string, progress: evaluationLab.TaskState) {
    try {
      view = await evaluationLab.labTasksState(evaluation, uuid, progress);
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }

  async function setAgent(agent: number | null) {
    try {
      view = await evaluationLab.labTasksAgent(evaluation, agent);
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }

  async function exportReport() {
    try {
      const written = await evaluationLab.reportExport(evaluation);

      if (written !== null) {
        notice = { done: true, text: `The report was written: ${written} file(s).` };
      }
    } catch (error) {
      notice = { done: false, text: `The report was not written: ${sentence(error)}` };
    }
  }

  function closeTask() {
    task = null;
    load();
  }
</script>

{#if task}
  <LabTask {evaluation} {task} onclose={closeTask} />
{:else if view}
  <div class="editor-head">
    <button type="button" class="small" onclick={onclose}>← All evaluations</button>
    <h2>{view.package_name}</h2>
  </div>
  <div class="card">
    <p><strong>{view.project_title}</strong>{view.product ? ` · ${view.product} ${view.product_version}` : ""}</p>
    {#if view.scope}<p class="note">{view.scope}</p>{/if}
    {#if view.note}<p class="note">{view.note}</p>{/if}
    <p class="note">Delegated to: {view.evaluator || "(nobody named)"}</p>
    <AgentPicker value={view.agent} label="Agent with knowledge for the requirements" onchange={setAgent} />
    <div class="buttons"><button type="button" onclick={exportReport}>Export the report…</button></div>
  </div>
  {#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}
  <div class="card lab-progress">
    <h3>Progress</h3>
    <div class="progress-figures">
      <span><strong>{view.progress.done}</strong> of {view.progress.tasks} tasks done</span>
      <span class="verdict-met"><strong>{view.progress.met}</strong> met</span>
      <span class="verdict-not_met"><strong>{view.progress.not_met}</strong> not met</span>
      <span class="verdict-inconclusive"><strong>{view.progress.inconclusive}</strong> inconclusive</span>
      <span><strong>{view.progress.no_verdict}</strong> without a verdict</span>
      <span><strong>{view.progress.outputs_ready}</strong> of {view.progress.outputs_due} outputs ready for the report</span>
    </div>
  </div>
  <table class="data">
    <thead><tr><th>Requirement</th><th>Verdict</th><th>Fragments</th><th>Outputs</th><th>State</th><th></th></tr></thead>
    <tbody>
      {#each view.tasks as item (item.uuid)}
        <tr>
          <td><span class="object-id">{item.label}</span> {item.title}</td>
          <td class={item.verdict ? `verdict-${item.verdict}` : "note"}>{item.verdict ? verdicts[item.verdict] : "—"}</td>
          <td>{item.fragments}</td>
          <td>{item.outputs_ready} of {item.outputs_due}</td>
          <td>
            <select value={item.state} onchange={(event) => setState(item.uuid, event.currentTarget.value as evaluationLab.TaskState)} aria-label="State">
              {#each Object.entries(states) as [value, title] (value)}<option {value}>{title}</option>{/each}
            </select>
          </td>
          <td class="actions"><button type="button" class="small" onclick={() => (task = item.uuid)}>Open</button></td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else if notice}
  <p class="problem">{notice.text}</p>
{/if}
