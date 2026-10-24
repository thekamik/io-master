<!--
  The screen of a task of a master project (step 51.2 of docs/todo.md; its
  commands task_scheme_* in src/views/evaluation_plan/evaluation_plan.rs):
  the scheme of the task drawn with Svelte Flow, the form of the block
  chosen (BlockForm), and the standard always beside it: the requirement as
  its catalog writes it and, when it has quotes, the page of the PDF.

  - A task opens with only the block of its requirement, inherited from
    OSCAL. "+ Input" and "+ Output" add blocks; a connection is drawn from
    the handle on the right of a block to the handle on the left of another (an input to the
    requirement, the requirement to an output; the program refuses the rest).
  - A block is moved by dragging; only the place where the drag ends is
    sent. A connection chosen can be removed.
  - When a block is added or removed, the drawing is fitted to the canvas
    (SchemeFit), so that the new block is seen.
  - "Propose with AI" asks the AI for inputs and corrections (step 51.3):
    the progress is shown as it comes; the proposals are dashed blocks and
    corrections in the form of their block, accepted or rejected there or
    all at once here. "Instructions…" opens the instructions of that AI.
  - Beside the standard, the related elements of the task (TaskLinks, step
    51.4): the work units and other elements the evaluator needs.
  - The program answers every change with the whole scheme as stored, so
    the drawing always shows what the program keeps.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { Background, Controls, MarkerType, SvelteFlow, type Connection, type Edge, type Node } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";

  import PartRows from "../../components/part_rows/PartRows.svelte";
  import PdfViewer from "../../components/pdf_viewer/PdfViewer.svelte";
  import { sentence } from "../../lib/api/errors";
  import * as evaluationPlan from "../../lib/api/evaluation_plan";
  import BlockForm from "./BlockForm.svelte";
  import BlockNode from "../../components/block_node/BlockNode.svelte";
  import AiWorkInstructions from "./AiWorkInstructions.svelte";
  import TaskLinks from "./TaskLinks.svelte";
  import SchemeFit from "./SchemeFit.svelte";

  let { task, onclose }: { task: string; onclose: () => void } = $props();

  let scheme = $state<evaluationPlan.SchemeView | null>(null);
  let nodes = $state.raw<Node[]>([]);
  let edges = $state.raw<Edge[]>([]);
  let selected: string | null = $state(null);
  let selectedConnection: { from: string; to: string } | null = $state(null);
  let quote: evaluationPlan.QuoteView | null = $state(null);
  let problem = $state("");
  /** The question to the AI while it runs: the characters written so far. */
  let asking: number | null = $state(null);
  /** What the last question gave, for the planner. */
  let report = $state("");
  let instructions = $state(false);

  const nodeTypes = { block: BlockNode };

  let block = $derived(scheme?.blocks.find((candidate) => candidate.uuid === selected) ?? null);
  let undecided = $derived(scheme?.blocks.some((candidate) => candidate.proposal !== null || candidate.correction) ?? false);

  onMount(async () => {
    try {
      show(await evaluationPlan.taskSchemeGet(task));
      quote = scheme?.requirement.quotes[0] ?? null;
    } catch (error) {
      problem = sentence(error);
    }
  });

  /** The drawing from the scheme as the program stores it. */
  function show(next: evaluationPlan.SchemeView) {
    scheme = next;

    if (selected !== null && !next.blocks.some((candidate) => candidate.uuid === selected)) {
      selected = null;
    }

    nodes = next.blocks.map((candidate) => ({
      id: candidate.uuid,
      type: "block",
      position: { x: candidate.x, y: candidate.y },
      data: { block: candidate, label: next.task.label, title: next.task.title, results: next.results },
      selected: candidate.uuid === selected,
    }));

    edges = next.connections.map((connection) => ({
      id: `${connection.from}>${connection.to}`,
      source: connection.from,
      target: connection.to,
      markerEnd: { type: MarkerType.ArrowClosed },
      selected: selectedConnection?.from === connection.from && selectedConnection?.to === connection.to,
    }));
  }

  /** Runs a change and shows the scheme the program gives back. */
  async function act(work: () => Promise<evaluationPlan.SchemeView>) {
    problem = "";

    try {
      show(await work());
    } catch (error) {
      problem = sentence(error);

      // Back to what is stored (a refused connection).
      if (scheme) {
        show(scheme);
      }
    }
  }

  /** A new block of `kind`: inputs on the left of the requirement, outputs on its right, one below another. */
  function add(kind: "input" | "output") {
    if (!scheme) {
      return;
    }

    const same = scheme.blocks.filter((candidate) => candidate.kind === kind);
    const lowest = Math.max(-130, ...same.map((candidate) => candidate.y));
    const x = kind === "input" ? -320 : 320;

    act(async () => {
      const next = await evaluationPlan.taskSchemeAddBlock(task, kind, x, lowest + 130);
      selected = next.blocks[next.blocks.length - 1]?.uuid ?? null;
      return next;
    });
  }

  /** A block saved or removed in its form (BlockForm): the scheme as stored, and no old problem left. */
  function changed(next: evaluationPlan.SchemeView) {
    problem = "";
    show(next);
  }

  function connect(connection: Connection) {
    act(() => evaluationPlan.taskSchemeConnect(connection.source, connection.target));
  }

  function moved({ nodes: dragged }: { nodes: Node[] }) {
    for (const node of dragged) {
      const stored = scheme?.blocks.find((candidate) => candidate.uuid === node.id);
      const x = Math.round(node.position.x);
      const y = Math.round(node.position.y);

      // The drawing is made again from `scheme`: it keeps the new place.
      if (stored) {
        stored.x = x;
        stored.y = y;
      }

      evaluationPlan.taskSchemeMoveBlock(node.id, x, y).catch((error) => (problem = sentence(error)));
    }
  }

  /** One event of the question to the AI. */
  function follow(event: evaluationPlan.AiEvent) {
    if (event.event === "writing") {
      asking = event.characters;
    } else if (event.event === "proposed") {
      const dropped = event.dropped.length > 0 ? ` Left out: ${event.dropped.join("; ")}.` : "";
      report = `The AI proposed ${event.added} input(s) and ${event.corrections} correction(s).${dropped}`;
    } else if (event.event === "failed") {
      problem = event.message;
    } else if (event.event === "end" && event.stopped) {
      report = "Stopped.";
    }
  }

  async function propose() {
    asking = 0;
    report = "";
    selected = null;
    await act(() => evaluationPlan.taskSchemePropose(task, follow));
    asking = null;
  }

  /** Accepts or rejects every proposal; the report of the question is done with then. */
  async function decideAll(accept: boolean) {
    await act(() => (accept ? evaluationPlan.taskSchemeAcceptAll(task) : evaluationPlan.taskSchemeRejectAll(task)));
    report = "";
  }

  function disconnect() {
    const connection = selectedConnection;
    selectedConnection = null;

    if (connection) {
      act(() => evaluationPlan.taskSchemeDisconnect(connection.from, connection.to));
    }
  }
</script>

<div class="editor-head">
  <button type="button" class="small" onclick={onclose}>← The project</button>
  {#if scheme}
    <h2><span class="object-id">{scheme.task.label}</span> {scheme.task.title}</h2>
  {/if}
</div>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if instructions}<AiWorkInstructions which="task_scheme" onclose={() => (instructions = false)} />{/if}

{#if scheme}
  <div class="scheme-body">
    <div class="scheme-canvas">
      <div class="buttons scheme-tools">
        <button type="button" class="small" onclick={() => add("input")}>+ Input</button>
        <button type="button" class="small" onclick={() => add("output")}>+ Output</button>
        {#if asking === null}
          <button type="button" class="small" onclick={propose}>Propose with AI</button>
        {:else}
          <button type="button" class="small danger" onclick={() => evaluationPlan.evaluationPlanStop()}>Stop</button>
        {/if}
        <button type="button" class="small" onclick={() => (instructions = true)}>Instructions…</button>
        {#if undecided}
          <button type="button" class="small" onclick={() => decideAll(true)}>Accept all</button>
          <button type="button" class="small danger" onclick={() => decideAll(false)}>Reject all</button>
        {/if}
        <p class="note scheme-note">
        {#if asking !== null}
          The AI is reading the requirement{asking > 0 ? `: ${asking} characters written` : "…"}
        {:else if report}
          {report}
        {:else}
          Connect two blocks by dragging from the handle on the right of one to the handle on the left of the other.
        {/if}
        </p>
      </div>
      <SvelteFlow
        bind:nodes
        bind:edges
        {nodeTypes}
        fitView
        fitViewOptions={{ maxZoom: 1 }}
        colorMode="dark"
        deleteKey={null}
        minZoom={0.2}
        connectionRadius={40}
        onconnect={connect}
        onnodedragstop={moved}
        onnodeclick={({ node }) => {
          selected = node.id;
          selectedConnection = null;
        }}
        onedgeclick={({ edge }) => {
          selectedConnection = { from: edge.source, to: edge.target };
          selected = null;
        }}
        onpaneclick={() => {
          selected = null;
          selectedConnection = null;
        }}
      >
        <Background />
        <Controls />
        <SchemeFit count={nodes.length} />
      </SvelteFlow>
    </div>

    <div class="scheme-side">
      {#if block}
        <div class="card"><BlockForm {block} onchanged={changed} /></div>
      {:else if selectedConnection}
        <div class="card">
          <h3>Connection</h3>
          <button type="button" class="danger" onclick={disconnect}>Remove the connection</button>
        </div>
      {/if}

      <div class="card scheme-standard">
        <h3>The standard</h3>
        <h4><span class="object-id">{scheme.requirement.label}</span> {scheme.requirement.title}</h4>
        {#if scheme.requirement.own}<p class="note">A requirement of the laboratory, outside the standards.</p>{/if}
        <PartRows rows={scheme.requirement.parts} />
        {#if scheme.requirement.quotes.length > 0}
          <div class="buttons">
            {#each scheme.requirement.quotes as candidate, index (index)}
              <button type="button" class="small" onclick={() => (quote = candidate)}>Page {candidate.page}</button>
            {/each}
          </div>
        {/if}
      </div>
      <TaskLinks {task} />
      {#if quote}
        <div class="scheme-pdf">
          <PdfViewer documentId={quote.document_id} page={quote.page} quote={quote.text} embedded />
        </div>
      {/if}
    </div>
  </div>
{/if}
