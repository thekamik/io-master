<!--
  Evaluation: carrying out a scheme for one product with the help of
  AI (step 23 of docs/todo.md). Designed when the schemes are done
  (user, 2026-10-16); until then the page lists the test projects that are
  ready to be carried out.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../lib/api/errors";
import { listProjects, type ProjectView } from "../lib/api/projects";

  let projects: ProjectView[] = $state([]);
  let problem = $state("");

  onMount(async () => {
    try {
      projects = (await listProjects()).filter((project) => project.ready);
    } catch (error) {
      problem = sentence(error);
    }
  });
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

<div class="card">
  <h3>Coming next</h3>
  <p class="note">
    An evaluation carries out a scheme for one product: the inputs are provided, a requirement is met when all its
    inputs are, and the outputs make the report in its standard form. The AI helps at each step with its sources; the
    verdicts and the final decisions are yours. The test projects are made in the tool Create evaluation scheme.
  </p>
</div>

<div class="card">
  <h3>Test projects ready to be carried out</h3>
  {#if projects.length === 0}
    <p class="note">No test project is marked ready yet.</p>
  {:else}
    <ul class="plain">
      {#each projects as project (project.id)}
        <li><strong>{project.name}</strong> <span class="note">{project.version}{project.author ? ` · ${project.author}` : ""}</span></li>
      {/each}
    </ul>
  {/if}
</div>
