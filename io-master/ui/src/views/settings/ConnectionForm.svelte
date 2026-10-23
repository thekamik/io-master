<!--
  The element "connection form" of the view Settings, tab AI
  (src/views/settings/connection_form/): one connection to an AI service.
  The program knows no provider: every connection says where the service
  is, how a request looks and where in the answer the text is.

  Two tabs: the fields, and the presets of well-known services, which only
  fill the fields as if they were typed; nothing is stored until "Save".
  Saving does not need the service. When the form changed the connections
  (saved, deleted, made the one the chat uses) it tells the view
  (onchanged), which asks the list and the key to read again; it also says
  whether the form has changes that are not saved (ondirty), so that the
  check waits for them.

  "AI help" at a field shows (and copies) an instruction for a chatbot of
  the evaluator's choice, to help fill that field
  (src/ai/presets/field_help.rs). Nothing comes back into the page on its
  own: the evaluator types the value the chatbot proposes. The instruction
  holds no value of a header and no key.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { isAppError, sentence } from "../../lib/api/errors";
  import * as connectionForm from "../../lib/api/settings/connection_form";

  let {
    chosen,
    active,
    onchanged,
    ondirty,
  }: {
    chosen: number | "new";
    active: number | null;
    onchanged: (chosen: number | null) => void;
    ondirty: (changed: boolean) => void;
  } = $props();

  /** A message after an action. */
  type Notice = { done: boolean; text: string } | null;

  let template: connectionForm.ConnectionView | null = $state(null);
  let presets: connectionForm.PresetView[] = $state([]);
  let problem = $state("");

  /** The connection in the form (a copy); null until it is read. */
  let editing: connectionForm.ConnectionView | null = $state(null);
  /** The fields as they were saved, to tell whether the form changed. */
  let saved = $state("");

  let notice: Notice = $state(null);
  /** The field that breaks a rule, as the program named it. */
  let badField: string | null = $state(null);
  let busy = $state(false);
  let confirmDelete = $state(false);

  /** The tab of the form. */
  let tab: "fields" | "presets" = $state("fields");

  /** The field whose AI instruction is shown; null: none. */
  let helpFor: string | null = $state(null);
  let helpText = $state("");
  let helpNotice = $state("");
  let helpArea: HTMLTextAreaElement | undefined = $state();

  /** Only the fields the program takes (it refuses any other). */
  function fields(view: connectionForm.ConnectionView): connectionForm.ConnectionFields {
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

  $effect(() => {
    ondirty(changed);
  });

  onMount(async () => {
    try {
      const fresh = await connectionForm.fresh();

      // The effect below then shows the chosen connection.
      template = fresh.template;
      presets = fresh.presets;
    } catch (error) {
      problem = `The form cannot be prepared: ${sentence(error)}`;
    }
  });

  // Shows the connection the view chose (after the template is read).
  $effect(() => {
    const wanted = chosen;

    if (template !== null && (editing === null || (editing.id ?? "new") !== wanted)) {
      show(wanted);
    }
  });

  /** Puts a connection into the form: a stored one, or a new one. */
  async function show(wanted: number | "new") {
    notice = null;
    badField = null;
    confirmDelete = false;
    tab = "fields";

    try {
      const view = wanted === "new" ? { ...template!, name: "" } : await connectionForm.get(wanted);

      editing = { ...view };
      saved = view.id === null ? "" : JSON.stringify(fields(view));
    } catch (error) {
      problem = sentence(error);
    }
  }

  /**
   * Fills the form with the fields of a preset, as if they were typed.
   * The connection itself (its number, its key) stays; a name already
   * given is kept. Nothing is stored until "Save".
   */
  function usePreset(preset: connectionForm.PresetView) {
    if (!editing) {
      return;
    }

    const name = editing.name.trim() === "" ? preset.fields.name : editing.name;

    Object.assign(editing, fields(preset.fields), { name });
    badField = null;
    notice = {
      done: true,
      text: `The fields were filled from the preset ${preset.title}. Check them, above all the model, and save.`,
    };
    tab = "fields";
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
      const result = await connectionForm.save(editing.id, fields(editing));

      editing.id = result.id;
      saved = JSON.stringify(fields(editing));
      notice = { done: true, text: result.notice };
      onchanged(result.id);
    } catch (error) {
      badField = isAppError(error) ? error.field : null;
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
      await connectionForm.remove(editing.id);
      editing = null;
      onchanged(null);
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
      await connectionForm.activate(editing.id);
      notice = { done: true, text: "The chat now uses this connection." };
      onchanged(editing.id);
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }

  /** Shows the instruction for a chatbot to help fill `field`, and copies it. */
  async function askHelp(field: string) {
    if (!editing) {
      return;
    }

    try {
      helpText = await connectionForm.fieldHelp(field, fields(editing));
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

{#if editing}
  <h2>
    {editing.id === null ? "New connection" : editing.name}
    {#if editing.id !== null && editing.id === active}<span class="badge success">used by the chat</span>{/if}
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
      {#each presets as preset (preset.id)}
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
            {#if editing.id !== active}
              <button type="button" onclick={activate} disabled={changed}>Use in the chat</button>
            {/if}
            {#if confirmDelete}
              <span>Delete this connection and its key?</span>
              <button type="button" class="danger" onclick={remove}>Delete</button>
              <button type="button" onclick={() => (confirmDelete = false)}>Keep</button>
            {:else}
              <button type="button" class="danger" onclick={() => (confirmDelete = true)}>Delete connection</button>
            {/if}
          {/if}
        </div>
      </form>
    {/if}
{/if}
