<!--
  The element "tasks proposed by the AI" of the view Evaluation plan (its
  commands task_proposals_* in src/views/evaluation_plan/evaluation_plan.rs,
  step 51.4): for a catalog of the project, the AI reads the scope and the
  requirements that are no task yet, part by part, and proposes those the
  project has to cover, each with its reason. "Accept" makes the task (with
  the outputs of the template), "Reject" drops the proposal; the screen of
  the project is told (onchanged), so that the list of tasks reads again.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";
  import AiWorkInstructions from "./AiWorkInstructions.svelte";

  let {
    project,
    catalogs,
    version,
    onchanged,
  }: {
    project: string;
    catalogs: { uuid: string; title: string }[];
    version: number;
    onchanged: () => void;
  } = $props();

  let proposals: evaluationPlan.TaskProposalView[] = $state([]);
  let catalog = $state("");
  /** The question to the AI while it runs: the part, the characters written. */
  let asking: { part: string; characters: number } | null = $state(null);
  let report = $state("");
  let problem = $state("");
  let instructions = $state(false);
  let open = $state(false);

  // Read again when the project changed (version): a task made elsewhere
  // decides its proposal.
  $effect(() => {
    void version;
    load(project);
  });

  // The first catalog of the project, when none is chosen yet.
  $effect(() => {
    const first = catalogs[0];

    if (catalog === "" && first) {
      catalog = first.uuid;
    }
  });

  async function load(id: string) {
    try {
      proposals = await evaluationPlan.taskProposalsGet(id);
      open = open || proposals.length > 0;
    } catch (error) {
      problem = sentence(error);
    }
  }

  /** One event of the work of the AI. */
  function follow(event: evaluationPlan.AiEvent) {
    if (asking === null) {
      return;
    }

    if (event.event === "part") {
      asking = { part: `part ${event.index} of ${event.count}`, characters: 0 };
    } else if (event.event === "writing") {
      asking = { ...asking, characters: event.characters };
    } else if (event.event === "proposed") {
      const dropped = event.dropped.length > 0 ? ` Left out: ${event.dropped.join("; ")}.` : "";
      report = `${report} The AI proposed ${event.added} requirement(s).${dropped}`.trim();
    } else if (event.event === "failed") {
      problem = event.message;
    } else if (event.stopped) {
      report = `${report} Stopped.`.trim();
    }
  }

  async function propose() {
    asking = { part: "", characters: 0 };
    report = "";
    problem = "";

    try {
      proposals = await evaluationPlan.taskProposalsPropose(project, catalog, follow);
    } catch (error) {
      problem = sentence(error);
    }

    asking = null;
  }

  /** Runs a decision and tells the project, whose list of tasks reads again. */
  async function decide(work: () => Promise<unknown>) {
    problem = "";

    try {
      await work();
      onchanged();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

{#if instructions}<AiWorkInstructions which="requirement_choice" onclose={() => (instructions = false)} />{/if}

<details class="card project-form" bind:open>
  <summary>Tasks proposed by the AI ({proposals.length})</summary>
  <p class="note">The AI reads the scope of the project and the requirements of a catalog that are no task yet, and proposes those to evaluate. Nothing becomes a task without you.</p>
  {#if problem}<p class="problem">{problem}</p>{/if}
  <div class="buttons">
    <select bind:value={catalog} aria-label="Catalog">
      {#each catalogs as choice (choice.uuid)}<option value={choice.uuid}>{choice.title}</option>{/each}
    </select>
    {#if asking === null}
      <button type="button" class="small" onclick={propose} disabled={catalog === ""}>Propose tasks with AI</button>
    {:else}
      <button type="button" class="small danger" onclick={() => evaluationPlan.aiWorkStop()}>Stop</button>
    {/if}
    <button type="button" class="small" onclick={() => (instructions = true)}>Instructions…</button>
    {#if proposals.length > 0}
      <button type="button" class="small" onclick={() => decide(() => evaluationPlan.taskProposalsAcceptAll(project))}>Accept all</button>
      <button type="button" class="small danger" onclick={() => decide(() => evaluationPlan.taskProposalsRejectAll(project))}>Reject all</button>
    {/if}
  </div>
  {#if asking !== null}
    <p class="note">The AI is reading the requirements{asking.part ? `, ${asking.part}` : "…"}{asking.characters > 0 ? `: ${asking.characters} characters written` : ""}</p>
  {:else if report}
    <p class="note">{report}</p>
  {/if}
  {#if proposals.length > 0}
    <table class="data">
      <tbody>
        {#each proposals as proposal (`${proposal.catalog_uuid}/${proposal.control_id}`)}
          <tr>
            <td>
              <span class="object-id">{proposal.label}</span> {proposal.title}
              <br /><span class="note">{proposal.catalog_title}</span>
              {#if proposal.reason}<br /><span class="note">The AI: {proposal.reason}</span>{/if}
            </td>
            <td class="actions">
              <button type="button" class="small" onclick={() => decide(() => evaluationPlan.taskProposalsAccept(project, proposal.catalog_uuid, proposal.control_id))}>Accept</button>
              <button type="button" class="small danger" onclick={() => decide(() => evaluationPlan.taskProposalsReject(project, proposal.catalog_uuid, proposal.control_id))}>Reject</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</details>
