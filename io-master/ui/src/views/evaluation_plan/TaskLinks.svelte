<!--
  The element "related elements" of the screen of a task (its commands
  task_links_* in src/views/evaluation_plan/evaluation_plan.rs, step 51.4):
  the elements of the other catalogs of the project that belong to the
  evaluation of the requirement (the work units of the CEM, test cases, ...).
  "Find with AI" asks the AI, part by part; a proposal is dashed, with its
  reason, and is accepted or rejected; an accepted element travels with the
  task to the evaluator and can be removed.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";
  import AiWorkInstructions from "./AiWorkInstructions.svelte";

  let { task }: { task: string } = $props();

  let links: evaluationPlan.TaskLinkView[] = $state([]);
  /** The question to the AI while it runs: the part, the characters written. */
  let asking: { part: string; characters: number } | null = $state(null);
  let report = $state("");
  let problem = $state("");
  let instructions = $state(false);

  onMount(async () => {
    try {
      links = await evaluationPlan.taskLinksGet(task);
    } catch (error) {
      problem = sentence(error);
    }
  });

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
      report = `${report} The AI proposed ${event.added} element(s).`.trim();
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
      links = await evaluationPlan.taskLinksPropose(task, follow);
    } catch (error) {
      problem = sentence(error);
    }

    asking = null;
  }

  async function decide(work: () => Promise<evaluationPlan.TaskLinkView[]>) {
    problem = "";

    try {
      links = await work();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

{#if instructions}<AiWorkInstructions which="related_elements" onclose={() => (instructions = false)} />{/if}

<div class="card">
  <h3>Related elements</h3>
  <p class="note">The work units, test cases and other elements of the catalogs of the project that the evaluator needs for this requirement.</p>
  {#if problem}<p class="problem">{problem}</p>{/if}
  <div class="buttons">
    {#if asking === null}
      <button type="button" class="small" onclick={propose}>Find with AI</button>
    {:else}
      <button type="button" class="small danger" onclick={() => evaluationPlan.evaluationPlanStop()}>Stop</button>
    {/if}
    <button type="button" class="small" onclick={() => (instructions = true)}>Instructions…</button>
  </div>
  {#if asking !== null}
    <p class="note">The AI is reading the other catalogs{asking.part ? `, ${asking.part}` : "…"}{asking.characters > 0 ? `: ${asking.characters} characters written` : ""}</p>
  {:else if report}
    <p class="note">{report}</p>
  {/if}
  {#each links as link (`${link.catalog_uuid}/${link.control_id}`)}
    <div class="link-row" class:ai-proposal={link.proposed}>
      <div><span class="object-id">{link.label}</span> {link.title}</div>
      <div class="note">{link.catalog_title}</div>
      {#if link.proposed && link.reason}<p>{link.reason}</p>{/if}
      <div class="buttons">
        {#if link.proposed}
          <button type="button" class="small" onclick={() => decide(() => evaluationPlan.taskLinksAccept(task, link.catalog_uuid, link.control_id))}>Accept</button>
          <button type="button" class="small danger" onclick={() => decide(() => evaluationPlan.taskLinksRemove(task, link.catalog_uuid, link.control_id))}>Reject</button>
        {:else}
          <button type="button" class="small danger" onclick={() => decide(() => evaluationPlan.taskLinksRemove(task, link.catalog_uuid, link.control_id))}>Remove</button>
        {/if}
      </div>
    </div>
  {/each}
</div>
