// The commands of the panel of the evaluation (src/panels/views/evaluation/),
// one function per command, named as the command: `evaluationGet` calls
// `evaluation_get`. The types mirror the views of those files; a change
// there needs the same change here.

import { Channel, invoke } from "@tauri-apps/api/core";

/** An evaluation in the list (evaluations.rs, EvaluationSummaryView). */
export interface EvaluationSummaryView {
  id: number;
  name: string;
  product: string;
  client: string;
  evaluator: string;
  /** Seconds since 1970 (UTC). */
  started: number | null;
  state: "evaluation" | "report";
  workUnits: number;
}

/** An agent to choose (evaluations.rs, AgentChoiceView). */
export interface AgentChoiceView {
  id: number;
  name: string;
}

/** A document of the product, without its file (evaluations.rs, DocumentInputView). */
export interface DocumentInputView {
  id: number;
  name: string;
  /** null while it is not given. */
  version: string | null;
  bytes: number;
  neededBy: number;
}

/** A proof of the laboratory (evaluations.rs, ProofInputView). */
export interface ProofInputView {
  id: number;
  name: string;
  /** null while it is not given. */
  proof: string | null;
  performed: number | null;
  tester: string | null;
  equipment: string | null;
  neededBy: number;
}

/** An evaluation whole (evaluations.rs, EvaluationView). */
export interface EvaluationView {
  summary: EvaluationSummaryView;
  aiAgent: number | null;
  agents: AgentChoiceView[];
  documents: DocumentInputView[];
  proofs: ProofInputView[];
}

/** Every evaluation and report, the newest first. Called by Evaluations.svelte. */
export function evaluationList(): Promise<EvaluationSummaryView[]> {
  return invoke("evaluation_list");
}

/** Begins an evaluation on a copy of a plan; gives its id. Called by Evaluations.svelte. */
export function evaluationCreate(
  plan: number,
  product: string,
  client: string,
  evaluator: string,
  agent: number,
): Promise<number> {
  return invoke("evaluation_create", { plan, product, client, evaluator, agent });
}

/** One evaluation whole. Called by EvaluationEditor.svelte. */
export function evaluationGet(id: number): Promise<EvaluationView> {
  return invoke("evaluation_get", { id });
}

/** Gives an evaluation its agent. Called by EvaluationEditor.svelte. */
export function evaluationSetAgent(id: number, agent: number): Promise<void> {
  return invoke("evaluation_set_agent", { id, agent });
}

/** Gives a document its file, chosen in the dialog of the system, and its
 * version; gives whether a file was chosen. Called by EvaluationEditor.svelte. */
export function evaluationDocumentGive(id: number, document: number, version: string): Promise<boolean> {
  return invoke("evaluation_document_give", { id, document, version });
}

/** Gives a proof its text and the record of its test. Called by EvaluationEditor.svelte. */
export function evaluationProofGive(
  id: number,
  proof: number,
  text: string,
  performed: number,
  tester: string,
  equipment: string,
): Promise<void> {
  return invoke("evaluation_proof_give", { id, proof, text, performed, tester, equipment });
}

/** A verdict as the program names it. */
export type VerdictName = "pass" | "fail" | "inconclusive";

/** A name and how many it counts (run.rs, CountView). */
export interface CountView {
  name: string;
  count: number;
}

/** What an assessment would ask (run.rs, AssessPreviewView). */
export interface AssessPreviewView {
  questions: number;
  agents: CountView[];
  /** The documents and proofs needed and not given: the assessment does not start. */
  missing: CountView[];
}

/** One step of an assessment (run.rs, AssessStepView). */
export interface AssessStepView {
  number: number;
  total: number;
  label: string;
  /** null when it was not assessed; the notes tell why. */
  verdict: VerdictName | null;
  notes: string[];
}

/** How an assessment ended (run.rs, AssessSummaryView). */
export interface AssessSummaryView {
  assessed: number;
  failed: number;
  stopped: boolean;
}

/** A work unit in the list of an evaluation (work_units.rs, UnitRowView). */
export interface UnitRowView {
  id: number;
  label: string;
  functional: boolean;
  agent: string;
  /** The agent is the one of the evaluation. */
  general: boolean;
  verdict: VerdictName | null;
  author: "ai" | "user" | null;
}

/** A name with its id and what it tells (work_units.rs, NamedView). */
export interface NamedView {
  id: number;
  name: string;
  detail: string | null;
}

/** A quote (work_units.rs, QuoteView). */
export interface QuoteView {
  document: number;
  text: string;
}

/** The last question of the AI about a work unit (work_units.rs, JournalView). */
export interface JournalView {
  time: number;
  model: string;
  request: string;
  answer: string;
  ending: string;
}

/** One work unit of an evaluation (work_units.rs, UnitView). */
export interface UnitView {
  id: number;
  label: string;
  standard: string;
  requirement: string;
  guidance: string;
  dependencies: NamedView[];
  documents: NamedView[];
  proofs: string[] | null;
  agent: string;
  findings: string;
  analysis: string;
  vulnerabilityNotes: string;
  notes: string;
  quotes: QuoteView[];
  verdict: VerdictName | null;
  author: "ai" | "user" | null;
  verdicts: VerdictName[];
  journal: JournalView | null;
}

/** The results of a work unit as the evaluator sends them (work_units.rs, ResultsFields). */
export interface ResultsFields {
  findings: string;
  analysis: string;
  vulnerabilityNotes: string;
  notes: string;
  quotes: QuoteView[];
  /** Empty for none. */
  verdict: VerdictName | "";
}

/** What an assessment would ask, counted now. Called by EvaluationUnits.svelte. */
export function evaluationAssessPreview(id: number, again: boolean): Promise<AssessPreviewView> {
  return invoke("evaluation_assess_preview", { id, again });
}

/** Assesses the work units with the AI; `onstep` gets each step as it ends.
 * Called by EvaluationUnits.svelte. */
export function evaluationAssess(
  id: number,
  again: boolean,
  onstep: (step: AssessStepView) => void,
): Promise<AssessSummaryView> {
  const onStep = new Channel<AssessStepView>();

  onStep.onmessage = onstep;

  return invoke("evaluation_assess", { id, again, onStep });
}

/** Asks the assessment to stop after its present work unit. Called by EvaluationUnits.svelte. */
export function evaluationAssessStop(): Promise<void> {
  return invoke("evaluation_assess_stop");
}

/** The work units of an evaluation in the order of the work. Called by EvaluationUnits.svelte. */
export function evaluationUnitList(id: number): Promise<UnitRowView[]> {
  return invoke("evaluation_unit_list", { id });
}

/** One work unit of an evaluation. Called by EvaluationUnit.svelte. */
export function evaluationUnitGet(id: number, unit: number): Promise<UnitView> {
  return invoke("evaluation_unit_get", { id, unit });
}

/** Saves the results of a work unit as the evaluator gives them. Called by EvaluationUnit.svelte. */
export function evaluationUnitSave(id: number, unit: number, fields: ResultsFields): Promise<void> {
  return invoke("evaluation_unit_save", { id, unit, fields });
}
