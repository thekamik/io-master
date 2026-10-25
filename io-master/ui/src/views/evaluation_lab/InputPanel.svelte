<!--
  The panel of an input of a task of the Evaluation lab (its commands
  evidence_*, input_chat_* and fragments_* in
  src/views/evaluation_lab/evaluation_lab.rs, step 52): what the plan
  expects; the evidence delivered (a file chosen in a dialog of the system,
  or a pasted text), kept as delivered; the chat with the AI about whether
  the evidence is ready; the fragments: exact quotes the program looked for
  in the evidence, proposed by the AI or written by the evaluator; only an
  accepted fragment is assessed. After a change it gives the screen back
  (onchanged).
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as evaluationLab from "../../lib/api/evaluation_lab";
  import { follow, idle, start } from "../../components/ai_work/ai_progress";
  import ConfirmButton from "../../components/confirm_button/ConfirmButton.svelte";
  import LabInstructions from "./LabInstructions.svelte";

  let {
    evaluation,
    screen,
    block,
    onchanged,
  }: {
    evaluation: string;
    screen: evaluationLab.LabTaskScreenView;
    block: evaluationLab.LabBlockView;
    onchanged: (screen: evaluationLab.LabTaskScreenView) => void;
  } = $props();

  const kinds = { pdf: "PDF", text: "text file", note: "pasted text" };

  let input = $derived(screen.inputs.find((candidate) => candidate.block === block.uuid));
  let progress = $state(idle());
  let note = $state({ open: false, name: "", text: "" });
  let own = $state({ open: false, evidence: "", page: "", quote: "", note: "" });
  let question = $state("");
  let instructions: evaluationLab.LabWork | null = $state(null);

  /** Runs a change and gives the screen back. */
  async function act(work: () => Promise<evaluationLab.LabTaskScreenView | null>) {
    progress.problem = "";

    try {
      const next = await work();

      if (next) {
        onchanged(next);
      }
    } catch (error) {
      progress.problem = sentence(error);
    }
  }

  /** Runs a work of the AI with its progress. */
  async function ask(what: string, work: (onEvent: (event: evaluationLab.AiEvent) => void) => Promise<evaluationLab.LabTaskScreenView>) {
    start(progress, what);
    await act(() => work((event) => follow(progress, event, what)));
    progress.running = false;
  }

  function addNote(event: SubmitEvent) {
    event.preventDefault();
    const written = { ...note };

    act(async () => {
      const next = await evaluationLab.evidenceAddNote(evaluation, block.uuid, written.name, written.text);
      note = { open: false, name: "", text: "" };
      return next;
    });
  }

  function addFragment(event: SubmitEvent) {
    event.preventDefault();
    const page = own.page.trim() === "" ? null : Number(own.page);
    const fragment = { evidence: own.evidence, page, quote: own.quote, note: own.note };

    act(async () => {
      const next = await evaluationLab.fragmentsAdd(evaluation, block.uuid, fragment);
      own = { open: false, evidence: "", page: "", quote: "", note: "" };
      return next;
    });
  }

  function send(event: SubmitEvent) {
    event.preventDefault();
    const text = question;

    ask("The AI is answering", async (onEvent) => {
      const next = await evaluationLab.inputChatSend(evaluation, block.uuid, text, onEvent);
      question = "";
      return next;
    });
  }

  /** The name of piece of evidence `uuid`. */
  function evidenceName(uuid: string): string {
    return input?.evidence.find((piece) => piece.uuid === uuid)?.name ?? "?";
  }
</script>

{#if instructions}<LabInstructions which={instructions} onclose={() => (instructions = null)} />{/if}

<h3>Input: {block.name}</h3>
{#if block.evidence}<p><strong>Expected:</strong> {block.evidence}</p>{/if}
{#if block.instructions}<p class="note">{block.instructions}</p>{/if}
{#if progress.problem}<p class="problem">{progress.problem}</p>{/if}
{#if progress.line}<p class="note">{progress.line}</p>{/if}

<h4>Evidence</h4>
{#each input?.evidence ?? [] as piece (piece.uuid)}
  <div class="link-row">
    <div>{piece.name} <span class="note">· {kinds[piece.kind]}{piece.kind === "pdf" ? `, ${piece.pages} pages` : ""}</span></div>
    <ConfirmButton onconfirm={() => act(() => evaluationLab.evidenceDelete(evaluation, screen.uuid, piece.uuid))} />
  </div>
{/each}
<div class="buttons">
  <button type="button" class="small" onclick={() => act(() => evaluationLab.evidenceAddFile(evaluation, block.uuid))}>Add a file…</button>
  <button type="button" class="small" onclick={() => (note.open = !note.open)}>Paste a text…</button>
</div>
{#if note.open}
  <form class="block-form" onsubmit={addNote}>
    <label>Name <input bind:value={note.name} placeholder="Interview with the developer, output of a tool, …" /></label>
    <label>Text <textarea bind:value={note.text} rows="6" required></textarea></label>
    <div class="buttons"><button type="submit">Add the text</button></div>
  </form>
{/if}

<h4>Fragments</h4>
<div class="buttons">
  {#if progress.running}
    <button type="button" class="small danger" onclick={() => evaluationLab.evaluationLabStop()}>Stop</button>
  {:else}
    <button type="button" class="small" disabled={(input?.evidence.length ?? 0) === 0} onclick={() => ask("The AI is reading the evidence", (onEvent) => evaluationLab.fragmentsPropose(evaluation, block.uuid, onEvent))}>Find fragments with AI</button>
  {/if}
  <button type="button" class="small" disabled={(input?.evidence.length ?? 0) === 0} onclick={() => (own.open = !own.open)}>Add a fragment…</button>
  <button type="button" class="small" onclick={() => (instructions = "evidence_fragments")}>Instructions…</button>
</div>
{#if own.open}
  <form class="block-form" onsubmit={addFragment}>
    <label>
      Evidence
      <select bind:value={own.evidence} required>
        {#each input?.evidence ?? [] as piece (piece.uuid)}<option value={piece.uuid}>{piece.name}</option>{/each}
      </select>
    </label>
    <label>Page <input bind:value={own.page} inputmode="numeric" placeholder="If known" /></label>
    <label>Quote (exact text) <textarea bind:value={own.quote} rows="3" required></textarea></label>
    <label>Note <textarea bind:value={own.note} rows="2"></textarea></label>
    <div class="buttons"><button type="submit">Add the fragment</button></div>
  </form>
{/if}
{#each input?.fragments ?? [] as fragment (fragment.uuid)}
  <div class="link-row" class:ai-proposal={!fragment.accepted}>
    <blockquote>{fragment.quote}</blockquote>
    <div class="note">
      {evidenceName(fragment.evidence)}{fragment.page ? `, page ${fragment.page}` : ""} ·
      {fragment.found ? "found in the evidence" : "NOT found in the evidence"} · {fragment.by_ai ? "proposed by the AI" : "yours"}
    </div>
    {#if fragment.note}<p class="note">AI note: {fragment.note}</p>{/if}
    <div class="buttons">
      {#if !fragment.accepted}
        <button type="button" class="small" onclick={() => act(() => evaluationLab.fragmentsAccept(evaluation, screen.uuid, fragment.uuid))}>Accept</button>
        <button type="button" class="small danger" onclick={() => act(() => evaluationLab.fragmentsRemove(evaluation, screen.uuid, fragment.uuid))}>Reject</button>
      {:else}
        <button type="button" class="small danger" onclick={() => act(() => evaluationLab.fragmentsRemove(evaluation, screen.uuid, fragment.uuid))}>Remove</button>
      {/if}
    </div>
  </div>
{/each}

<h4>Chat about the evidence</h4>
{#each input?.messages ?? [] as message, index (index)}
  <p class={message.speaker === "model" ? "chat-model" : "chat-evaluator"}><strong>{message.speaker === "model" ? "AI" : "You"}:</strong> {message.text}</p>
{/each}
<form class="block-form" onsubmit={send}>
  <textarea bind:value={question} rows="2" placeholder="Is the evidence complete for this requirement?"></textarea>
  <div class="buttons">
    {#if progress.running}
      <button type="button" class="small danger" onclick={() => evaluationLab.evaluationLabStop()}>Stop</button>
    {:else}
      <button type="submit" class="small" disabled={question.trim() === ""}>Send</button>
    {/if}
    <button type="button" class="small" onclick={() => act(() => evaluationLab.inputChatClear(evaluation, block.uuid))}>Clear the chat</button>
    <button type="button" class="small" onclick={() => (instructions = "input_chat")}>Instructions…</button>
  </div>
</form>
