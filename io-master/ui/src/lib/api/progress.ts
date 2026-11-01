// The progress of the long work of the AI (src/panels/views/progress.rs),
// which the commands evaluation_assess_progress and plan_fill_progress give:
// the page asks for it while the work runs, and when a panel is opened
// again. The types mirror the views of that file; a change there needs the
// same change here.

import type { VerdictName } from "./evaluation";

/** A work unit not taken, or taken with notes (ProblemView). */
export interface ProblemView {
  label: string;
  notes: string[];
}

/** The progress of the last long work on one plan or evaluation (ProgressView). */
export interface ProgressView {
  /** It runs; otherwise it ended and the counts are its summary. */
  running: boolean;
  /** The user asked it to stop. */
  stopping: boolean;
  total: number;
  assessed: number;
  failed: number;
  /** The last work unit gone through; empty before the first. */
  label: string;
  /** null without a verdict. */
  lastVerdict: VerdictName | null;
  problems: ProblemView[];
}

/** How often the page asks for the progress while a work runs, in milliseconds. */
export const PROGRESS_INTERVAL = 500;
