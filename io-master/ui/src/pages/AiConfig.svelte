<!--
  AI config: the connections of the evaluator to AI services (step 15
  of docs/todo.md). The program knows no provider: every connection
  says where the service is, how a request looks and where in the
  answer the text is (src/ai/connection.rs).

  The list of connections is on the left, the form of the chosen one
  on the right, with two tabs: the fields, and the presets of
  well-known services, which only fill the fields as if they were
  typed; nothing is stored until "Save". The page never holds a stored API key: it learns only
  whether one is stored, and a key typed here is sent once and then
  cleared from the page. Saving does not need the service; "Check
  connection" sends one short message over the saved connection and
  shows what went out and what came back (the key masked).

  "AI help" at a field shows (and copies) an instruction for a chatbot
  of the evaluator's choice, to help fill that field
  (src/ai/field_help.rs). Nothing comes back into the page on its
  own: the evaluator types the value the chatbot proposes. The
  instruction holds no value of a header and no key.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import {
    activateAiConnection,
    aiConfig,
    aiFieldInstruction,
    checkAiConnection,
    deleteAiConnection,
    deleteAiKey,
    isUiError,
    saveAiConnection,
    sentence,
    storeAiKey,
    type AiConfigView,
    type CheckView,
    type ConnectionFields,
    type ConnectionView,
    type PresetView,
  } from "../lib/api";

  /** A message after an action. */
  type Notice = { done: boolean; text: string } | null;

  let config: AiConfigView | null = $state(null);
  let problem = $state("");

  /** The connection in the form (a copy); null: none chosen. */
  let editing: ConnectionView | null = $state(null);
  /** The fields as they were saved, to tell whether the form changed. */
  let saved = $state("");

  let notice: Notice = $state(null);
  /** The field that breaks a rule, as the program named it. */
  let badField: string | null = $state(null);
  let busy = $state(false);
  let confirmDelete = $state(false);

  /** The tab of the form. */
  let tab: "fields" | "presets" = $state("fields");

  let apiKey = $state("");
  let keyNotice: Notice = $state(null);

  let report: CheckView | null = $state(null);
  let checking = $state(false);

  /** The field whose AI instruction is shown; null: none. */
  let helpFor: string | null = $state(null);
  let helpText = $state("");
  let helpNotice = $state("");
  let helpArea: HTMLTextAreaElement | undefined = $state();

  /** Only the fields the program takes (it refuses any other). */
  function fields(view: ConnectionView): ConnectionFields {
    return {
      name: view.name,
      url: view.url,
      model: view.model,
      ca_certificate: view.ca_certificate,
      headers: view.headers,
      body: view.body,
      message_template: view.message_template,
      user_role: view.user_role,
      model_role: view.model_role,
      system_role: view.system_role,
      system_prompt: view.system_prompt,
      answer: view.answer,
      text_path: view.text_path,
      stop_path: view.stop_path,
      cut_reasons: view.cut_reasons,
      refused_reasons: view.refused_reasons,
      error_path: view.error_path,
      model_path: view.model_path,
      input_tokens_path: view.input_tokens_path,
      output_tokens_path: view.output_tokens_path,
      done_marker: view.done_marker,
    };
  }

  /** Whether the form differs from what is saved. */
  let changed = $derived(editing !== null && JSON.stringify(fields(editing)) !== saved);

  async function load() {
    try {
      config = await aiConfig();
    } catch (error) {
      problem = `Your AI connections cannot be read: ${sentence(error)}`;
    }
  }

  onMount(load);

  /** Puts a connection into the form. */
  function choose(view: ConnectionView) {
    editing = { ...view };
    saved = view.id === null ? "" : JSON.stringify(fields(view));
    notice = keyNotice = report = null;
    badField = null;
    confirmDelete = false;
    apiKey = "";
    tab = "fields";
  }

  /**
   * Fills the form with the fields of a preset, as if they were
   * typed. The connection itself (its number, its key) stays; a name
   * already given is kept. Nothing is stored until "Save".
   */
  function usePreset(preset: PresetView) {
    if (!editing) {
      return;
    }

    const name = editing.name.trim() === "" ? preset.fields.name : editing.name;

    Object.assign(editing, fields(preset.fields), { name });
    badField = null;
    report = null;
    notice = {
      done: true,
      text: `The fields were filled from the preset ${preset.title}. Check them, above all the model, and save.`,
    };
    tab = "fields";
  }

  function newConnection() {
    if (config) {
      choose({ ...config.template, name: "" });
    }
  }

  /** After an action: the new state of the page, the form kept on `id`. */
  function refresh(next: AiConfigView, id: number | null) {
    config = next;

    const view = next.connections.find((candidate) => candidate.id === id);

    if (view && editing) {
      // The saved fields and the key state; the form itself is kept.
      editing.id = view.id;
      editing.has_key = view.has_key;
      editing.key_unused = view.key_unused;
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();

    if (!editing) {
      return;
    }

    busy = true;
    notice = null;
    badField = null;

    try {
      const result = await saveAiConnection(editing.id, fields(editing));
      const view = result.config.connections.find((candidate) => candidate.id === result.id);

      config = result.config;

      if (view) {
        choose(view);
      }

      notice = { done: true, text: result.notice };
    } catch (error) {
      badField = isUiError(error) ? error.field : null;
      notice = { done: false, text: `The connection was not saved: ${sentence(error)}` };
    } finally {
      busy = false;
    }
  }

  async function remove() {
    if (!editing || editing.id === null) {
      return;
    }

    try {
      config = await deleteAiConnection(editing.id);
      editing = null;
    } catch (error) {
      notice = { done: false, text: `The connection was not deleted: ${sentence(error)}` };
    } finally {
      confirmDelete = false;
    }
  }

  async function activate() {
    if (!editing || editing.id === null) {
      return;
    }

    try {
      refresh(await activateAiConnection(editing.id), editing.id);
      notice = { done: true, text: "The chat now uses this connection." };
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }

  async function submitKey(event: SubmitEvent) {
    event.preventDefault();

    if (!editing || editing.id === null) {
      return;
    }

    try {
      refresh(await storeAiKey(editing.id, apiKey), editing.id);
      keyNotice = { done: true, text: "The API key was stored." };
    } catch (error) {
      keyNotice = { done: false, text: `The key was not stored: ${sentence(error)}` };
    } finally {
      // The key leaves the page as soon as it was sent.
      apiKey = "";
    }
  }

  async function removeKey() {
    if (!editing || editing.id === null) {
      return;
    }

    try {
      refresh(await deleteAiKey(editing.id), editing.id);
      keyNotice = { done: true, text: "The API key was deleted." };
    } catch (error) {
      keyNotice = { done: false, text: `The key was not deleted: ${sentence(error)}` };
    }
  }

  /** Shows the instruction for a chatbot to help fill `field`, and copies it. */
  async function askHelp(field: string) {
    if (!editing) {
      return;
    }

    try {
      helpText = await aiFieldInstruction(field, fields(editing));
      helpFor = field;
      helpNotice = "";
      await copyHelp();
    } catch (error) {
      helpFor = field;
      helpText = "";
      helpNotice = `The instruction cannot be made: ${sentence(error)}`;
    }
  }

  /** Copies the instruction shown; without the clipboard, selects it to be copied by hand. */
  async function copyHelp() {
    try {
      await navigator.clipboard.writeText(helpText);
      helpNotice = "Copied. Paste it into your chatbot, describe your service where it says so, and type the value it proposes.";
    } catch {
      helpArea?.select();
      helpNotice = "The text could not be copied by itself; it is selected below: copy it with Ctrl+C.";
    }
  }

  async function check() {
    if (!editing || editing.id === null) {
      return;
    }

    checking = true;
    report = null;

    try {
      report = await checkAiConnection(editing.id);
    } catch (error) {
      badField = isUiError(error) ? error.field : null;
      notice = { done: false, text: `The connection cannot be checked: ${sentence(error)}` };
    } finally {
      checking = false;
    }
  }
</script>

{#snippet help(field: string)}
  <button type="button" class="help" onclick={() => askHelp(field)} title="An instruction for a chatbot to help fill this field">
    AI help
  </button>
{/snippet}

{#snippet panel(names: string[])}
  {#if helpFor !== null && names.includes(helpFor)}
    <div class="help-panel">
      <p class="note">{helpNotice}</p>
      {#if helpText}
        <textarea class="mono" rows="10" readonly bind:this={helpArea} value={helpText}></textarea>
      {/if}
      <div class="buttons">
        {#if helpText}<button type="button" onclick={copyHelp}>Copy</button>{/if}
        <button type="button" onclick={() => (helpFor = null)}>Close</button>
      </div>
    </div>
  {/if}
{/snippet}

{#if problem}<p class="problem">{problem}</p>{/if}

{#if config}
  <div class="chat">
    <aside class="chat-list">
      <button class="primary" onclick={newConnection} disabled={busy}>New connection</button>
      <ul>
        {#each config.connections as connection (connection.id)}
          <li>
            <button
              class="link"
              class:active={connection.id === editing?.id}
              onclick={() => choose(connection)}
              disabled={busy}
            >
              {connection.name}
            </button>
            {#if connection.id === config.active}<span class="badge success">used by the chat</span>{/if}
          </li>
        {/each}
      </ul>
      {#if config.connections.length === 0}
        <p class="note">No connection yet.</p>
      {/if}
    </aside>

    <section class="conversation ai-config">
      {#if !editing}
        <div class="card"><p class="note">
          Choose a connection, or make a new one. A connection describes completely how the program talks to an AI
          service: a local model of the laboratory, or any other service with an HTTP API.
        </p></div>
      {:else}
        <h2>
          {editing.id === null ? "New connection" : editing.name}
          {#if editing.id !== null && editing.id === config.active}<span class="badge success">used by the chat</span>{/if}
        </h2>

        {#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}

        <div class="tabs">
          <button type="button" class:active={tab === "fields"} onclick={() => (tab = "fields")}>Fields</button>
          <button type="button" class:active={tab === "presets"} onclick={() => (tab = "presets")}>Presets</button>
        </div>

        {#if tab === "presets"}
          <p class="note">
            A preset fills the fields with the settings of a well-known service, as if you typed them. Nothing is
            stored until you check the fields and save; the name you gave is kept.
          </p>
          {#each config.presets as preset (preset.id)}
            <fieldset class="preset">
              <legend>{preset.title}</legend>
              <p>{preset.description}</p>
              <p class="note mono">{preset.fields.url} · {preset.fields.model}</p>
              <button type="button" class="primary" onclick={() => usePreset(preset)}>Use</button>
            </fieldset>
          {/each}
        {:else}
          <form onsubmit={save}>
            <fieldset>
              <legend>Connection</legend>
              <label class:invalid={badField === "name"}>
                Name <input bind:value={editing.name} maxlength="60" required />
              </label>
              <label for="f-url" class:invalid={badField === "url"}>
                Endpoint (full URL)
                {@render help("url")}
                <input id="f-url" class="mono" bind:value={editing.url} spellcheck="false" required />
                <span class="note">
                  https:// with TLS 1.3, or http:// only to this computer (127.0.0.1 or [::1]). May hold {"{{model}}"}.
                </span>
              </label>
              {@render panel(["url"])}
              <label for="f-model" class:invalid={badField === "model"}>
                Model {@render help("model")}
                <input id="f-model" class="mono" bind:value={editing.model} spellcheck="false" />
                <span class="note">Put in place of {"{{model}}"}; needed only when a template uses it.</span>
              </label>
              {@render panel(["model"])}
              <details open={editing.ca_certificate !== "" || badField === "ca_certificate"}>
                <summary>CA certificate of the service (optional)</summary>
                <label for="f-ca_certificate" class:invalid={badField === "ca_certificate"}>
                  Certificate in PEM
                  {@render help("ca_certificate")}
                  <textarea id="f-ca_certificate" class="mono" rows="6" bind:value={editing.ca_certificate} spellcheck="false"
                    placeholder="-----BEGIN CERTIFICATE-----"></textarea>
                  <span class="note">
                    Given here, only this authority is trusted for the service (for example the internal authority of
                    the laboratory); empty, the trusted certificates of the system.
                  </span>
                </label>
                {@render panel(["ca_certificate"])}
              </details>
            </fieldset>

            <fieldset>
              <legend>Request</legend>
              <p class="note">
                Placeholders: {"{{messages}}"} the conversation, {"{{system}}"} the instructions, {"{{model}}"} the
                model, {"{{api_key}}"} the API key (in a header only). In the body a placeholder is a whole JSON text,
                for example <code>"messages": "{"{{messages}}"}"</code>.
              </p>
              <label for="f-headers" class:invalid={badField === "headers"}>
                Headers, one per line (name: value)
                {@render help("headers")}
                <textarea id="f-headers" class="mono" rows="4" bind:value={editing.headers} spellcheck="false"></textarea>
              </label>
              {@render panel(["headers"])}
              <label for="f-body" class:invalid={badField === "body"}>
                Body (JSON template)
                {@render help("body")}
                <textarea id="f-body" class="mono" rows="10" bind:value={editing.body} spellcheck="false"></textarea>
              </label>
              {@render panel(["body"])}
              <label for="f-message_template" class:invalid={badField === "message_template"}>
                One message in {"{{messages}}"} (JSON template)
                {@render help("message_template")}
                <textarea id="f-message_template" class="mono" rows="2" bind:value={editing.message_template} spellcheck="false"></textarea>
                <span class="note">
                  {"{{role}}"} is the role of the speaker, {"{{text}}"} the text; each a whole JSON text.
                </span>
              </label>
              {@render panel(["message_template"])}
              <div class="row">
                <label for="f-user_role" class:invalid={badField === "user_role"}>
                  Role of the evaluator {@render help("user_role")}
                  <input id="f-user_role" class="mono" bind:value={editing.user_role} spellcheck="false" />
                </label>
                <label for="f-model_role" class:invalid={badField === "model_role"}>
                  Role of the model {@render help("model_role")}
                  <input id="f-model_role" class="mono" bind:value={editing.model_role} spellcheck="false" />
                </label>
                <label for="f-system_role" class:invalid={badField === "system_role"}>
                  Role of the instructions {@render help("system_role")}
                  <input id="f-system_role" class="mono" bind:value={editing.system_role} spellcheck="false" />
                </label>
              </div>
              {@render panel(["user_role", "model_role", "system_role"])}
              <p class="note">
                With a role of the instructions, they go first into {"{{messages}}"}; leave it empty for a service that
                takes them in a field of its own ({"{{system}}"}).
              </p>
              <label for="f-system_prompt" class:invalid={badField === "system_prompt"}>
                Instructions for the model
                {@render help("system_prompt")}
                <textarea id="f-system_prompt" rows="4" bind:value={editing.system_prompt}></textarea>
              </label>
              {@render panel(["system_prompt"])}
            </fieldset>

            <fieldset>
              <legend>Answer</legend>
              <div class="row">
                {@render help("answer")}
                <label class="choice">
                  <input type="radio" bind:group={editing.answer} value="stream" /> A stream of events (shown while written)
                </label>
                <label class="choice">
                  <input type="radio" bind:group={editing.answer} value="whole" /> One JSON when complete
                </label>
              </div>
              {@render panel(["answer"])}
              <p class="note">
                Paths point into the JSON of {editing.answer === "stream" ? "every event" : "the answer"}:
                <code>choices[0].delta.content</code>, <code>content[*].text</code>. Only the path to the text is needed.
              </p>
              <label for="f-text_path" class:invalid={badField === "text_path"}>
                Path to the text {@render help("text_path")}
                <input id="f-text_path" class="mono" bind:value={editing.text_path} spellcheck="false" required />
              </label>
              {@render panel(["text_path"])}
              <div class="row">
                <label for="f-stop_path" class:invalid={badField === "stop_path"}>
                  Path to the stop reason {@render help("stop_path")}
                  <input id="f-stop_path" class="mono" bind:value={editing.stop_path} spellcheck="false" />
                </label>
                <label for="f-cut_reasons" class:invalid={badField === "cut_reasons"}>
                  Reasons for "cut" {@render help("cut_reasons")}
                  <input id="f-cut_reasons" class="mono" bind:value={editing.cut_reasons} spellcheck="false" />
                </label>
                <label for="f-refused_reasons" class:invalid={badField === "refused_reasons"}>
                  Reasons for "declined" {@render help("refused_reasons")}
                  <input id="f-refused_reasons" class="mono" bind:value={editing.refused_reasons} spellcheck="false" />
                </label>
              </div>
              {@render panel(["stop_path", "cut_reasons", "refused_reasons"])}
              <div class="row">
                <label for="f-error_path" class:invalid={badField === "error_path"}>
                  Path to an error {@render help("error_path")}
                  <input id="f-error_path" class="mono" bind:value={editing.error_path} spellcheck="false" />
                </label>
                <label for="f-model_path" class:invalid={badField === "model_path"}>
                  Path to the model {@render help("model_path")}
                  <input id="f-model_path" class="mono" bind:value={editing.model_path} spellcheck="false" />
                </label>
              </div>
              {@render panel(["error_path", "model_path"])}
              <div class="row">
                <label for="f-input_tokens_path" class:invalid={badField === "input_tokens_path"}>
                  Path to the tokens of the request
                  {@render help("input_tokens_path")}
                  <input id="f-input_tokens_path" class="mono" bind:value={editing.input_tokens_path} spellcheck="false" />
                </label>
                <label for="f-output_tokens_path" class:invalid={badField === "output_tokens_path"}>
                  Path to the tokens of the reply
                  {@render help("output_tokens_path")}
                  <input id="f-output_tokens_path" class="mono" bind:value={editing.output_tokens_path} spellcheck="false" />
                </label>
              </div>
              {@render panel(["input_tokens_path", "output_tokens_path"])}
              {#if editing.answer === "stream"}
                <label for="f-done_marker" class:invalid={badField === "done_marker"}>
                  Data of the last event (optional)
                  {@render help("done_marker")}
                  <input id="f-done_marker" class="mono" bind:value={editing.done_marker} spellcheck="false" placeholder="[DONE]" />
                </label>
                {@render panel(["done_marker"])}
              {/if}
            </fieldset>

            <div class="buttons">
              <button type="submit" disabled={busy}>{busy ? "Saving…" : "Save"}</button>
              {#if editing.id !== null}
                {#if editing.id !== config.active}
                  <button type="button" onclick={activate} disabled={changed}>Use in the chat</button>
                {/if}
                <button type="button" onclick={check} disabled={checking || changed}>
                  {checking ? "Checking…" : "Check connection"}
                </button>
                {#if confirmDelete}
                  <span>Delete this connection and its key?</span>
                  <button type="button" class="danger" onclick={remove}>Delete</button>
                  <button type="button" onclick={() => (confirmDelete = false)}>Keep</button>
                {:else}
                  <button type="button" class="danger" onclick={() => (confirmDelete = true)}>Delete connection</button>
                {/if}
              {/if}
              {#if changed && editing.id !== null}<span class="note">Save the changes to check or use them.</span>{/if}
            </div>
          </form>
        {/if}

        <fieldset>
          <legend>API key</legend>
          {#if editing.id === null}
            <p class="note">Save the connection first; then its API key can be stored.</p>
          {:else}
            <p class="note">
              Kept only in your encrypted database, never shown again. It is bound to the address of the endpoint:
              when the scheme, host or port changes, the key is removed.
            </p>
            {#if editing.has_key}
              <p>
                <span class="badge success">An API key is stored</span>
                <button type="button" class="small danger" onclick={removeKey}>Delete the key</button>
              </p>
              {#if editing.key_unused}
                <p class="problem">No header uses {"{{api_key}}"}, so the key is never sent.</p>
              {/if}
            {:else}
              <p>No API key is stored{editing.headers.includes("{{api_key}}") ? ", but a header uses one" : ""}.</p>
            {/if}
            {#if keyNotice}<p class={keyNotice.done ? "done" : "problem"}>{keyNotice.text}</p>{/if}
            <form class="row" onsubmit={submitKey}>
              <label>
                {editing.has_key ? "Replace the key" : "API key"}
                <input type="password" bind:value={apiKey} autocomplete="off" spellcheck="false" required />
              </label>
              <button type="submit">Store the key</button>
            </form>
          {/if}
        </fieldset>

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
      {/if}
    </section>
  </div>
{/if}
