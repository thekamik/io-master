// The component "block of a scheme" (ui/src/components/block_node/): a block
// as the drawing of a scheme shows it, in the Evaluation plan (where it is
// edited) and in the Evaluation lab (where it is read only). The data comes
// from the scheme of each view.

/** A block on the drawing. The proposals of the AI exist only in the plan. */
export interface DrawnBlock {
  uuid: string;
  kind: "requirement" | "input" | "output";
  x: number;
  y: number;
  name: string;
  format: "text" | "latex";
  condition: "always" | "not_met" | "met";
  proposal?: string | null;
  correction?: unknown;
}

/** What a node of the drawing carries (Svelte Flow asks for an object of named fields). */
export interface BlockNodeData extends Record<string, unknown> {
  block: DrawnBlock;
  /** The label and the title of the requirement of the task. */
  label: string;
  title: string;
  /** The titles of the four results, shown in the block of the requirement. */
  results: string[];
  /** A short line under the block (the Evaluation lab: its evidence, whether an output waits). */
  note?: string;
}
