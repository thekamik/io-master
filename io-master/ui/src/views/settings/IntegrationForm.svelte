<!--
  One integration with an AI service (its commands integration_create,
  integration_save, integration_set_key and integration_check in
  src/panels/views/settings/integrations.rs): its data, its API key and the
  check of the connection. The key is only written: the program never gives
  it back, the form shows only whether one is stored. A stored integration
  whose address changed needs its key given anew (the key is bound to its
  address). Tells the tab after a write (onsaved, with the id).
-->
<script lang="ts">
  import { untrack } from "svelte";

  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
  import { sentence } from "../../lib/api/errors";
  import {
    integrationCheck,
    integrationCreate,
    integrationSave,
    integrationSetKey,
    type CheckView,
    type IntegrationView,
    type ProtocolName,
  } from "../../lib/api/settings";

  let {
    integration,
    onsaved,
    onclose,
  }: { integration: IntegrationView | null; onsaved: (id: number) => void; onclose: () => void } = $props();

  /** What a new integration of each protocol is most often. */
  const EXAMPLES: Record<ProtocolName, { url: string; model: string }> = {
    openai_chat: { url: "http://127.0.0.1:11434/v1/chat/completions", model: "llama3.1" },
    anthropic_messages: { url: "https://api.anthropic.com/v1/messages", model: "claude-opus-5-5" },
  };

  // The form starts from the integration given; it is made anew for another
  // one (the {#key} of the tab), so the first value is enough (untrack: read
  // once, on purpose).
  const start = untrack(() => integration);

  let name = $state(start?.name ?? "");
  let protocol: ProtocolName = $state(start?.protocol ?? "openai_chat");
  let url = $state(start?.url ?? "");
  let model = $state(start?.model ?? "");
  let contextSize = $state(start?.contextSize ?? 32768);
  let caCertificate = $state(start?.caCertificate ?? "");
  let key = $state("");
  let check: CheckView | null = $state(null);
  let problem = $state("");
  let busy = $state(false);
  let checking = $state(false);
  let savedAt: Date | null = $state(null);

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

    const fields = { id: integration?.id ?? 0, name, url, model, protocol, caCertificate, contextSize };

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

  <form onsubmit={save}>
    <div class="row">
      <label>Name <input bind:value={name} required placeholder="Ollama on this computer" /></label>
      <label>
        Protocol
        <select bind:value={protocol}>
          <option value="openai_chat">OpenAI chat (Ollama, llama.cpp, vLLM)</option>
          <option value="anthropic_messages">Anthropic messages</option>
        </select>
      </label>
    </div>
    <label>Address <input class="mono" bind:value={url} required placeholder={EXAMPLES[protocol].url} /></label>
    <div class="row">
      <label>Model <input class="mono" bind:value={model} required placeholder={EXAMPLES[protocol].model} /></label>
      <label>
        Context, in tokens
        <input type="number" min="512" step="1" bind:value={contextSize} required />
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
      </dl>
    {/if}
  {/if}

  {#if problem}<p class="problem">{problem}</p>{/if}
  <SaveStatus {busy} {savedAt} />
</section>
