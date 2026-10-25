// The progress of a work of the AI (the component ai_work of the page): what
// the page shows while the AI writes, and what it gave. Presentation only;
// the panels of the Evaluation lab (InputPanel, RequirementPanel,
// OutputPanel) and the proving ground of the view Knowledge (RAG) keep one each.

import type { AiEvent } from "../../lib/api/components/ai_work";

/** What a panel shows of a work of the AI. */
export interface Progress {
  running: boolean;
  line: string;
  problem: string;
}

/** A progress with no work. */
export function idle(): Progress {
  return { running: false, line: "", problem: "" };
}

/** Starts a work: the line says what the AI does. */
export function start(progress: Progress, what: string): void {
  progress.running = true;
  progress.line = `${what}…`;
  progress.problem = "";
}

/** One event of the work, written into `progress`; `what` says what the AI does. */
export function follow(progress: Progress, event: AiEvent, what: string): void {
  if (event.event === "part") {
    progress.line = `${what}, part ${event.index} of ${event.count}…`;
  } else if (event.event === "writing") {
    progress.line = `${what}: ${event.characters} characters written`;
  } else if (event.event === "proposed") {
    const dropped = event.dropped.length > 0 ? ` Left out: ${event.dropped.join("; ")}.` : "";
    progress.line = `Done: ${event.added} proposed, to be decided by you.${dropped}`;
  } else if (event.event === "failed") {
    progress.problem = event.message;
  } else {
    progress.running = false;
    if (event.stopped) {
      progress.line = "Stopped.";
    }
  }
}
