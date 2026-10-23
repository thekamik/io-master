<!--
  The element "check of a connection" of the view Settings, tab AI
  (its commands in src/views/settings/settings.rs): sends one short message over the
  saved connection and shows what went out and what came back (the key
  masked). Not possible while the form has changes that are not saved
  (disabled).
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as settings from "../../lib/api/settings";

  let { id, disabled }: { id: number; disabled: boolean } = $props();

  let report: settings.CheckView | null = $state(null);
  let problem = $state("");
  let checking = $state(false);

  $effect(() => {
    // Another connection: its report is not this one.
    void id;
    report = null;
    problem = "";
  });

  async function check() {
    checking = true;
    report = null;
    problem = "";

    try {
      report = await settings.connectionCheckRun(id);
    } catch (error) {
      problem = `The connection cannot be checked: ${sentence(error)}`;
    } finally {
      checking = false;
    }
  }
</script>

<div class="buttons">
  <button type="button" onclick={check} disabled={checking || disabled}>
    {checking ? "Checking…" : "Check connection"}
  </button>
  {#if disabled}<span class="note">Save the changes to check or use them.</span>{/if}
</div>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if report}
  <fieldset class="report">
    <legend>Check</legend>
    <p class={report.works ? "done" : "problem"}>{report.outcome}</p>

    <h3>Sent</h3>
    <pre>POST {report.url}
{#each report.headers as header, index (index)}{header.name}: {header.value}
{/each}
{report.body}</pre>

    <h3>Answer{report.status !== null ? ` (HTTP ${report.status})` : ""}</h3>
    {#if report.raw}<pre>{report.raw}</pre>{:else}<p class="note">No answer came.</p>{/if}

    <h3>Found by the paths</h3>
    <dl>
      <dt>Text</dt>
      <dd>{report.text || "—"}</dd>
      <dt>Model</dt>
      <dd>{report.model ?? "—"}</dd>
      <dt>Tokens</dt>
      <dd>{report.input_tokens} of the request, {report.output_tokens} of the reply</dd>
    </dl>
  </fieldset>
{/if}
