// What the page shows while the AI works for the editor of a catalog: the
// line of progress and the log, from the events of the work
// (src/views/converter/catalog_editor/ai_builder/view.rs, BuildEvent). Used
// by the two elements that start a work of the AI: AiBuilder.svelte (the
// manual editing) and ConversionWizard.svelte (the conversion step by step).

import type { BuildEvent } from "../../../lib/api/converter/catalog_editor";

/** The progress of a work of the AI. */
export interface BuildProgress {
  progress: string;
  log: { done: boolean; text: string }[];
}

/** Applies `event` to `state`. */
export function applyBuildEvent(state: BuildProgress, event: BuildEvent): void {
  switch (event.event) {
    case "part":
      state.progress = `Part ${event.index} of ${event.count}: ${event.document}, pages ${event.first_page}–${event.last_page}…`;
      break;
    case "writing":
      state.progress = state.progress.replace(/ — .* characters$/, "") + ` — ${event.characters} characters`;
      break;
    case "proposed":
      state.log.push({ done: true, text: `Proposed: ${event.added}.` });
      for (const dropped of event.dropped) {
        state.log.push({ done: false, text: `Dropped: ${dropped}` });
      }
      break;
    case "failed":
      state.log.push({ done: false, text: event.message });
      break;
    case "end":
      state.progress = event.stopped ? "Stopped." : "";
      break;
  }
}
