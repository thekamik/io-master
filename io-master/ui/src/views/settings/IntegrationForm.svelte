<!--
  One integration with an AI service (its commands integration_form,
  integration_create, integration_save, integration_set_key and
  integration_check in
  src/panels/views/settings/integrations.rs): its data, its API key and the
  check of the connection. The key is only written: the program never gives
  it back, the form shows only whether one is stored. A stored integration
  whose address changed needs its key given anew (the key is bound to its
  address). Where a new integration starts and the limits of the fields
  come from the program (integration_form): the page holds no value of its
  own. In a new integration a change of the protocol brings the start of the
  new protocol into every field not changed by hand.
  Tells the tab after a write (onsaved, with the id).
-->
<script lang="ts">
  import { untrack } from "svelte";

  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
  import { sentence } from "../../lib/api/errors";
  import {
    integrationCheck,
    integrationCreate,
    integrationForm,
    integrationSave,
    integrationSetKey,
    type CheckView,
    type IntegrationFormView,
    type IntegrationView,
    type ProtocolName,
  } from "../../lib/api/settings";

  let {
    integration,
    onsaved,
    onclose,
  }: { integration: IntegrationView | null; onsaved: (id: number) => void; onclose: () => void } = $props();

  // The form starts from the integration given; it is made anew for another
  // one (the {#key} of the tab), so the first value is enough (untrack: read
  // once, on purpose).
  const start = untrack(() => integration);

  // Every field starts from the integration given, or from the program
  // (loadForm) for a new one.
  let name = $state(start?.name ?? "");
  let protocol: ProtocolName | null = $state(start?.protocol ?? null);
  let url = $state(start?.url ?? "");
  let model = $state(start?.model ?? "");
  let contextSize: number | null = $state(start?.contextSize ?? null);
  let charactersPerToken: number | null = $state(start?.charactersPerToken ?? null);
  let connectTimeoutSeconds: number | null = $state(start?.connectTimeoutSeconds ?? null);
  let streamSilenceSeconds: number | null = $state(start?.streamSilenceSeconds ?? null);
  let form: IntegrationFormView | null = $state(null);
  let caCertificate = $state(start?.caCertificate ?? "");
  let key = $state("");
  let check: CheckView | null = $state(null);
  let problem = $state("");
  let busy = $state(false);
  let checking = $state(false);
  let savedAt: Date | null = $state(null);

  /** Reads the limits of the fields and, for a new integration, where it
   * starts; a stored integration keeps its values. */
  async function loadForm() {
    try {
      form = await integrationForm(protocol);

      if (start === null) {
        name = form.name;
        protocol = form.protocol;
        url = form.url;
        model = form.model;
        contextSize = form.contextSize;
        charactersPerToken = form.charactersPerToken;
        connectTimeoutSeconds = form.connectTimeoutSeconds;
        streamSilenceSeconds = form.streamSilenceSeconds;
      }
    } catch (error) {
      problem = sentence(error);
    }
  }

  void loadForm();

  /** A new integration whose protocol changed: every field still as the old
   * protocol started it takes the start of the new one; a field changed by
   * hand stays. */
  async function changeProtocol() {
    if (start !== null || form === null || protocol === null) {
      return;
    }

    try {
      const before = form;
      const after = await integrationForm(protocol);

      if (name === before.name) name = after.name;
      if (url === before.url) url = after.url;
      if (model === before.model) model = after.model;
      if (contextSize === before.contextSize) contextSize = after.contextSize;
      if (charactersPerToken === before.charactersPerToken) charactersPerToken = after.charactersPerToken;
      if (connectTimeoutSeconds === before.connectTimeoutSeconds) connectTimeoutSeconds = after.connectTimeoutSeconds;
      if (streamSilenceSeconds === before.streamSilenceSeconds) streamSilenceSeconds = after.streamSilenceSeconds;
      form = after;
    } catch (error) {
      problem = sentence(error);
    }
  }

  /** Runs one write: the buttons locked while it runs, the time of the
   * write shown when it succeeded. */
  async function write(work: () => Promise<number>) {
    problem = "";
    busy = true;

    try {
      const id = await work();
      savedAt = new Date();
      onsaved(id);
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  function save(event: SubmitEvent) {
    event.preventDefault();

    // The fields are required: the page sends nothing until each is given.
    if (
      protocol === null ||
      contextSize === null ||
      charactersPerToken === null ||
      connectTimeoutSeconds === null ||
      streamSilenceSeconds === null
    ) {
      return;
    }

    const fields = {
      id: integration?.id ?? 0,
      name,
      url,
      model,
      protocol,
      caCertificate,
      contextSize,
      charactersPerToken,
      connectTimeoutSeconds,
      streamSilenceSeconds,
    };

    void write(async () => {
      if (fields.id === 0) {
        return integrationCreate(fields);
      }

      await integrationSave(fields);
      return fields.id;
    });
  }

  function storeKey(event: SubmitEvent) {
    event.preventDefault();

    const id = integration?.id ?? 0;

    void write(async () => {
      await integrationSetKey(id, key);
      key = "";
      return id;
    });
  }

  async function checkConnection() {
    problem = "";
    check = null;
    checking = true;

    try {
      check = await integrationCheck(integration?.id ?? 0);
    } catch (error) {
      problem = sentence(error);
    } finally {
      checking = false;
    }
  }
</script>

<section class="card ai-config">
  <h2>{integration === null ? "A new integration" : integration.name}</h2>

  {#if form}
    <form onsubmit={save}>
      <div class="row">
        <label>Name <input bind:value={name} required /></label>
        <label>
          Protocol
          <select bind:value={protocol} onchange={changeProtocol}>
            <option value="openai_chat">OpenAI chat (Ollama, llama.cpp, vLLM)</option>
            <option value="anthropic_messages">Anthropic messages</option>
          </select>
        </label>
      </div>
      <label>Address <input class="mono" bind:value={url} required /></label>
      <div class="row">
        <label>Model <input class="mono" bind:value={model} required /></label>
        <label>
          Context, in tokens
          <span class="note">As the service of the model gives it.</span>
          <input type="number" min={form.minContext} step="1" bind:value={contextSize} required />
        </label>
        <label>
          Characters of a token
          <span class="note">The estimate a question is measured by; fewer is safer.</span>
          <input
            type="number"
            min="1"
            max={form.maxCharactersPerToken}
            step="1"
            bind:value={charactersPerToken}
            required
          />
        </label>
      </div>
      <div class="row">
        <label>
          Wait for a connection, in seconds
          <input
            type="number"
            min="1"
            max={form.maxConnectTimeoutSeconds}
            step="1"
            bind:value={connectTimeoutSeconds}
            required
          />
        </label>
        <label>
          Silence of an answer, in seconds
          <span class="note">A local model may think long before it answers.</span>
          <input
            type="number"
            min="1"
            max={form.maxStreamSilenceSeconds}
            step="1"
            bind:value={streamSilenceSeconds}
            required
          />
        </label>
      </div>
      <details>
        <summary>Own CA certificate (a server of the laboratory)</summary>
        <label>
          CA certificate (PEM)
          <span class="note">Empty: the trusted certificates of the system.</span>
          <textarea class="mono" rows="6" bind:value={caCertificate}></textarea>
        </label>
      </details>
      <div class="buttons">
        <button type="submit" disabled={busy}>{integration === null ? "Add the integration" : "Save"}</button>
        <button type="button" class="ghost" onclick={onclose}>Close</button>
      </div>
    </form>
  {/if}

  {#if integration !== null}
    <form onsubmit={storeKey}>
      <h3>
        API key
        {#if integration.hasKey}
          <span class="badge success">stored</span>
        {:else}
          <span class="badge warning">none</span>
        {/if}
      </h3>
      <p class="note">
        Kept only in your encrypted database and never shown again. A local model usually needs none; the Anthropic
        service always does.
      </p>
      <div class="row">
        <label>New key <input type="password" class="mono" autocomplete="off" bind:value={key} required /></label>
        <button type="submit" disabled={busy}>Store the key</button>
      </div>
    </form>

    <h3>Connection</h3>
    <p class="note">One short question. It belongs to no agent, so it is not kept in a journal.</p>
    <p><button type="button" disabled={checking || busy} onclick={checkConnection}>
      {checking ? "Checking…" : "Check the connection"}
    </button></p>

    {#if check}
      <dl class="facts">
        <dt>Ending</dt>
        <dd>
          <span class="badge" class:success={check.ending === "complete"} class:danger={check.ending !== "complete"}>
            {check.ending}
          </span>
        </dd>
        {#if check.error}<dt>Error</dt><dd class="problem">{check.error}</dd>{/if}
        <dt>Answer</dt><dd>{check.text || "—"}</dd>
        <dt>Model</dt><dd class="mono">{check.model}</dd>
        <dt>Tokens</dt><dd>{check.inputTokens} in, {check.outputTokens} out</dd>
        {#if check.contextWarning}<dt>Context</dt><dd class="note">{check.contextWarning}</dd>{/if}
      </dl>
    {/if}
  {/if}

  {#if problem}<p class="problem">{problem}</p>{/if}
  <SaveStatus {busy} {savedAt} />
</section>
