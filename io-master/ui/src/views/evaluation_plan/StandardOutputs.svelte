<!--
  The element "standard outputs" of the view Evaluation plan (its commands
  standard_outputs_* in src/views/evaluation_plan/evaluation_plan.rs, step
  51.3): the template of the outputs of a master project, shown folded.
  Every task added from now on gets a copy of these outputs, connected from
  its requirement, which the planner may change in that task; the tasks
  made before keep their outputs. An output is added, changed and deleted
  here (OutputFields); the program answers with the whole template.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";
  import OutputFields from "./OutputFields.svelte";

  let { project }: { project: string } = $props();

  const formats = { text: "a text file", latex: "LaTeX" };
  const conditions = { always: "always", not_met: "when not met", met: "when met" };

  let outputs: evaluationPlan.StandardOutputView[] = $state([]);
  /** The output in the form: its UUID, "new", or none. */
  let editing: string | null = $state(null);
  let form: evaluationPlan.StandardOutputForm = $state(empty());
  let problem = $state("");

  onMount(async () => {
    try {
      outputs = await evaluationPlan.standardOutputsGet(project);
    } catch (error) {
      problem = sentence(error);
    }
  });

  function empty(): evaluationPlan.StandardOutputForm {
    return { name: "", instructions: "", target: "", format: "latex", condition: "always" };
  }

  function edit(output: evaluationPlan.StandardOutputView | null) {
    editing = output ? output.uuid : "new";
    form = output
      ? { name: output.name, instructions: output.instructions, target: output.target, format: output.format, condition: output.condition }
      : empty();
    problem = "";
  }

  /** Runs a change and shows the template the program gives back. */
  async function act(work: () => Promise<evaluationPlan.StandardOutputView[]>) {
    try {
      outputs = await work();
      editing = null;
      problem = "";
    } catch (error) {
      problem = sentence(error);
    }
  }

  function save(event: SubmitEvent) {
    event.preventDefault();
    const fields = { ...form };
    const target = editing;

    act(() =>
      target === "new" || target === null
        ? evaluationPlan.standardOutputsAdd(project, fields)
        : evaluationPlan.standardOutputsSave(target, fields),
    );
  }
</script>

<details class="card project-form">
  <summary>Standard outputs ({outputs.length})</summary>
  <p class="note">The default outputs of every task added from now on, connected from its requirement; a task may change them.</p>
  {#if problem}<p class="problem">{problem}</p>{/if}
  {#if outputs.length > 0}
    <table class="data">
      <tbody>
        {#each outputs as output (output.uuid)}
          <tr>
            <td>{output.name}</td>
            <td>{formats[output.format]}</td>
            <td>{conditions[output.condition]}</td>
            <td class="actions">
              <button type="button" class="small" onclick={() => edit(output)}>Change</button>
              <button type="button" class="small danger" onclick={() => act(() => evaluationPlan.standardOutputsDelete(output.uuid))}>Delete</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
  {#if editing}
    <form class="block-form" onsubmit={save}>
      <OutputFields bind:fields={form} />
      <div class="buttons">
        <button type="submit">{editing === "new" ? "Add" : "Save"}</button>
        <button type="button" onclick={() => (editing = null)}>Cancel</button>
      </div>
    </form>
  {:else}
    <button type="button" class="small" onclick={() => edit(null)}>+ Standard output</button>
  {/if}
</details>
