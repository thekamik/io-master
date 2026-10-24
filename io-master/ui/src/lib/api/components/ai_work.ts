// The component "work of the AI" (src/components/ai_work/): the events of
// one question of a work of the AI, in the views Evaluation plan and
// Evaluation lab.

/** An event of a work of the AI. */
export type AiEvent =
  | { event: "part"; index: number; count: number }
  | { event: "writing"; characters: number }
  | { event: "proposed"; added: number; corrections: number; dropped: string[] }
  | { event: "failed"; message: string }
  | { event: "end"; stopped: boolean };
