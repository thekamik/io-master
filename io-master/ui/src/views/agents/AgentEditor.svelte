<!--
  One agent (its commands agent_get, agent_save and agent_knowledge_test in
  src/panels/views/agents/agent.rs): its name, its integration, its
  instructions, its knowledge bases (shared with other agents), its
  glossary and its limits; the test of the knowledge it would give with a
  question (nothing is sent to an AI service); its journal (AgentJournal).
  The rows of the glossary are edited in the page and sent whole with
  "Save"; empty rows are left out by the program.
-->
<script lang="ts">
  import SaveStatus from "../../components/save_status/SaveStatus.svelte";
  import { agentGet, agentKnowledgeTest, agentSave, type AgentView, type TermView } from "../../lib/api/agents";
  import { sentence } from "../../lib/api/errors";
  import AgentJournal from "./AgentJournal.svelte";

  let { agent, onback }: { agent: number; onback: () => void } = $props();

  let view: AgentView | null = $state(null);
  let name = $state("");
  let integration = $state(0);
  let instructions = $state("");
  let bases: number[] = $state([]);
  let glossary: TermView[] = $state([]);
  let maxPassages = $state(8);
  let maxCharacters = $state(12000);
  let question = $state("");
  let passages: string[] | null = $state(null);
  let problem = $state("");
  let busy = $state(false);
  let savedAt: Date | null = $state(null);

  async function load(id: number) {
    problem = "";

    try {
      view = await agentGet(id);
      name = view.name;
      integration = view.integration;
      instructions = view.instructions;
      bases = [...view.knowledgeBases];
      glossary = view.glossary.map((entry) => ({ ...entry }));
      maxPassages = view.maxPassages;
      maxCharacters = view.maxCharacters;
    } catch (error) {
      problem = sentence(error);
    }
  }

  $effect(() => {
    void load(agent);
  });

  function toggle(id: number, on: boolean) {
    bases = on ? [...bases, id] : bases.filter((chosen) => chosen !== id);
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      await agentSave({ id: agent, name, integration, instructions, knowledgeBases: bases, glossary, maxPassages, maxCharacters });
      savedAt = new Date();
      await load(agent);
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  async function test(event: SubmitEvent) {
    event.preventDefault();
    problem = "";

    try {
      passages = await agentKnowledgeTest(agent, question);
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<p><button type="button" class="ghost" onclick={onback}>← Agents</button></p>

{#if view}
  <section class="card ai-config">
    <h2>{view.name}</h2>
    <form onsubmit={save}>
      <div class="row">
        <label>Name <input bind:value={name} required /></label>
        <label>
          Integration
          <select bind:value={integration}>
            {#each view.integrations as one (one.id)}
              <option value={one.id}>{one.name}</option>
            {/each}
          </select>
        </label>
      </div>
      <label>
        Instructions
        <span class="note">Who the agent is and how it answers; given to the model with every question.</span>
        <textarea rows="6" bind:value={instructions}></textarea>
      </label>

      <h3>Knowledge bases</h3>
      {#if view.bases.length === 0}
        <p class="note">No knowledge base yet: make one in the tab Knowledge bases.</p>
      {:else}
        <div class="choices">
          {#each view.bases as base (base.id)}
            <label class="choice">
              <input
                type="checkbox"
                checked={bases.includes(base.id)}
                onchange={(event) => toggle(base.id, event.currentTarget.checked)}
              />
              {base.name}
            </label>
          {/each}
        </div>
      {/if}

      <h3>Glossary</h3>
      <p class="note">A term the question names widens the search by its meaning ("TSF" also finds "TOE Security Functionality").</p>
      {#each glossary as entry, index (index)}
        <div class="row">
          <label>Term <input bind:value={entry.term} /></label>
          <label>Meaning <input bind:value={entry.meaning} /></label>
          <button type="button" class="ghost" onclick={() => (glossary = glossary.filter((_, at) => at !== index))}>
            Take out
          </button>
        </div>
      {/each}
      <p><button type="button" onclick={() => (glossary = [...glossary, { term: "", meaning: "" }])}>A new term</button></p>

      <h3>Limits of one question</h3>
      <div class="row">
        <label>Passages, at most <input type="number" min="1" max="50" bind:value={maxPassages} required /></label>
        <label>
          Characters, at most
          <input type="number" min="1500" max="40000" step="500" bind:value={maxCharacters} required />
        </label>
      </div>

      <div class="buttons"><button type="submit" disabled={busy}>Save</button></div>
    </form>
    <SaveStatus {busy} {savedAt} />
  </section>

  <section class="card">
    <h2>Test of the knowledge</h2>
    <p class="note">
      The passages the agent would give with a question, as saved: from its bases, widened by its glossary, within its
      limits. Nothing is sent to the AI service.
    </p>
    <form onsubmit={test}>
      <label>Question <input bind:value={question} required placeholder="secure boot of the TOE" /></label>
      <button type="submit">Show the knowledge</button>
    </form>
    {#if passages !== null}
      {#if passages.length === 0}
        <p class="note">No passage fits the question.</p>
      {:else}
        <p class="note">
          {passages.length} passages, {passages.reduce((sum, passage) => sum + passage.length, 0)} characters.
        </p>
        {#each passages as passage, index (index)}
          <pre class="passage">{passage}</pre>
        {/each}
      {/if}
    {/if}
  </section>

  <AgentJournal {agent} />
{/if}

{#if problem}<p class="problem">{problem}</p>{/if}

<style>
  h3 {
    margin-top: 1.25rem;
  }

  pre.passage {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 0.85rem;
  }
</style>
