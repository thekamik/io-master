<!--
  The custom formats of the settings of the AI (their commands
  custom_format_list, custom_format_new, custom_format_create,
  custom_format_save and custom_format_try in
  src/panels/views/settings/custom_formats.rs): the JSON of the question and
  of the answer of a service that speaks its own, written whole by the user,
  with placeholders where the program writes its question and reads the
  answer. A format is tried here without any connection (the request filled,
  a sample of an answer read); the connection itself is tried with the check
  of an integration that names the format. The page holds no value of its
  own: a new format starts from the program (custom_format_new).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
  import { sentence } from "../../lib/api/errors";
  import {
    customFormatCreate,
    customFormatList,
    customFormatNew,
    customFormatSave,
    customFormatTry,
    type CustomFormatView,
    type TrialView,
  } from "../../lib/api/settings";

  let formats: CustomFormatView[] = $state([]);
  /** The format in the editor, as typed; null for none. */
  let edited: CustomFormatView | null = $state(null);
  let sample = $state("");
  let trial: TrialView | null = $state(null);
  let problem = $state("");
  let busy = $state(false);
  let savedAt: Date | null = $state(null);

  async function reload() {
    try {
      formats = await customFormatList();
    } catch (error) {
      problem = sentence(error);
    }
  }

  function open(format: CustomFormatView) {
    edited = { ...format };
    sample = "";
    trial = null;
    problem = "";
    savedAt = null;
  }

  async function openNew() {
    try {
      open(await customFormatNew());
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();

    if (edited === null) {
      return;
    }

    problem = "";
    busy = true;

    try {
      if (edited.id === 0) {
        edited.id = await customFormatCreate(edited);
      } else {
        await customFormatSave(edited);
      }

      savedAt = new Date();
      await reload();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  async function tryFormat() {
    if (edited === null) {
      return;
    }

    problem = "";
    trial = null;
    busy = true;

    try {
      trial = await customFormatTry(edited, sample);
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  onMount(reload);
</script>

<section class="card">
  <h2>Custom formats</h2>
  <p class="note">
    For a service that speaks neither the chat completions of OpenAI nor the messages of Anthropic: the tool of an
    institute with a JSON of its own. An integration of the protocol "Custom" names one of these formats.
  </p>
  <table class="data">
    <thead><tr><th>Name</th><th>Answer</th><th>Key in</th><th></th></tr></thead>
    <tbody>
      {#each formats as format (format.id)}
        <tr>
          <td class="wrap">{format.name}</td>
          <td>{format.responseKind === "whole" ? "one JSON document" : "a stream of events"}</td>
          <td class="mono">{format.keyHeader}</td>
          <td><button type="button" onclick={() => open(format)}>Open</button></td>
        </tr>
      {:else}
        <tr><td colspan="4" class="note">No custom format yet.</td></tr>
      {/each}
    </tbody>
  </table>
  <p><button type="button" onclick={openNew}>A new format</button></p>
</section>

{#if edited !== null}
  <section class="card ai-config">
    <h2>{edited.id === 0 ? "A new format" : edited.name}</h2>

    <form onsubmit={save}>
      <div class="row">
        <label>Name <input bind:value={edited.name} required /></label>
        <label>
          The answer comes as
          <select bind:value={edited.responseKind}>
            <option value="whole">one JSON document</option>
            <option value="events">a stream of events (SSE)</option>
          </select>
        </label>
      </div>
      <label>
        Request: the JSON the service takes, whole
        <span class="note">
          The program writes its question where these stand: <code>&lt;prompt&gt;</code> (the last question) or
          <code>&lt;messages&gt;</code> (the whole conversation, a list of roles and contents; without it a chat sends
          only its last question), <code>&lt;system&gt;</code>, <code>&lt;model&gt;</code>,
          <code>&lt;max_answer_tokens&gt;</code>, <code>&lt;temperature&gt;</code>. Everything else is sent as written.
        </span>
        <textarea class="mono" rows="12" spellcheck="false" bind:value={edited.request} required></textarea>
      </label>
      <label>
        Response: the JSON the service gives{edited.responseKind === "events" ? ", one event of its stream" : ""}
        <span class="note">
          The program reads the answer where <code>&lt;response&gt;</code> stands; optional:
          <code>&lt;error&gt;</code>, <code>&lt;model&gt;</code>, <code>&lt;input_tokens&gt;</code>,
          <code>&lt;output_tokens&gt;</code>. Each stands alone as a value; the rest of the JSON is passed over.
        </span>
        <textarea class="mono" rows="10" spellcheck="false" bind:value={edited.response} required></textarea>
      </label>
      <div class="row">
        <label>
          Header of the API key
          <input class="mono" bind:value={edited.keyHeader} required />
        </label>
        <label>
          Before the key
          <span class="note">"Bearer " with its space, or empty.</span>
          <input class="mono" bind:value={edited.keyPrefix} />
        </label>
      </div>
      <div class="buttons">
        <button type="submit" disabled={busy}>{edited.id === 0 ? "Add the format" : "Save"}</button>
        <button type="button" class="ghost" onclick={() => (edited = null)}>Close</button>
      </div>
    </form>

    <h3>Try the format</h3>
    <p class="note">
      Nothing is sent and nothing stored: the request is filled with a sample of a question, and an answer of the
      service pasted here (from its documentation) is read as its answer would be. Empty: the request alone.
    </p>
    <label>
      A sample of an answer
      <textarea class="mono" rows="6" spellcheck="false" bind:value={sample}></textarea>
    </label>
    <p><button type="button" disabled={busy} onclick={tryFormat}>Try</button></p>

    {#if trial}
      <dl class="facts">
        <dt>Request</dt><dd><pre class="mono">{trial.request}</pre></dd>
        {#if trial.read}
          <dt>Answer read</dt><dd>{trial.text || "—"}</dd>
          <dt>Model</dt><dd class="mono">{trial.model || "—"}</dd>
          <dt>Tokens</dt><dd>{trial.inputTokens} in, {trial.outputTokens} out</dd>
        {/if}
      </dl>
    {/if}

    {#if problem}<p class="problem">{problem}</p>{/if}
    <SaveStatus {busy} {savedAt} />
  </section>
{:else if problem}
  <p class="problem">{problem}</p>
{/if}

<style>
  table.data {
    width: 100%;
  }

  pre {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
