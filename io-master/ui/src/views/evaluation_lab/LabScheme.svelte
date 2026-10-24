<!--
  The scheme of a task of the Evaluation lab, read only (step 52): the
  drawing of the plan (the component BlockNode), frozen; a block is chosen
  to open its panel (onchoose). Under every input the number of pieces of
  its evidence, under every output whether its condition holds. Presentation
  only.
-->
<script lang="ts">
  import { Background, Controls, MarkerType, SvelteFlow, type Edge, type Node } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";

  import BlockNode from "../../components/block_node/BlockNode.svelte";
  import type { BlockNodeData } from "../../lib/api/components/block_node";
  import type { LabTaskScreenView } from "../../lib/api/evaluation_lab";

  let {
    screen,
    chosen,
    onchoose,
  }: { screen: LabTaskScreenView; chosen: string | null; onchoose: (block: string | null) => void } = $props();

  const nodeTypes = { block: BlockNode };

  /** The line under a block: its evidence, or whether an output waits. */
  function note(uuid: string, kind: string): string | undefined {
    if (kind === "input") {
      const input = screen.inputs.find((candidate) => candidate.block === uuid);
      const accepted = input?.fragments.filter((fragment) => fragment.accepted).length ?? 0;

      return `${input?.evidence.length ?? 0} evidence · ${accepted} fragment(s) accepted`;
    }

    if (kind === "output") {
      const output = screen.outputs.find((candidate) => candidate.block === uuid);

      if (!output?.applies) {
        return "waits for its verdict";
      }

      return output.text ? "text accepted" : "to be written";
    }

    return undefined;
  }

  let nodes = $derived<Node[]>(
    screen.blocks.map((block) => {
      const data: BlockNodeData = {
        block,
        label: screen.requirement.label,
        title: screen.requirement.title,
        results: screen.result_titles,
        note: note(block.uuid, block.kind),
      };

      return { id: block.uuid, type: "block", position: { x: block.x, y: block.y }, data, selected: block.uuid === chosen };
    }),
  );

  let edges = $derived<Edge[]>(
    screen.connections.map((connection) => ({
      id: `${connection.from}>${connection.to}`,
      source: connection.from,
      target: connection.to,
      markerEnd: { type: MarkerType.ArrowClosed },
    })),
  );
</script>

<SvelteFlow
  {nodes}
  {edges}
  {nodeTypes}
  fitView
  fitViewOptions={{ maxZoom: 1 }}
  colorMode="dark"
  nodesDraggable={false}
  nodesConnectable={false}
  deleteKey={null}
  minZoom={0.2}
  onnodeclick={({ node }) => onchoose(node.id)}
  onpaneclick={() => onchoose(null)}
>
  <Background />
  <Controls showLock={false} />
</SvelteFlow>
