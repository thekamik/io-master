// The commands of the view Evaluation lab (src/views/evaluation_lab/
// evaluation_lab.rs, step 52): a work package imported and carried out for
// one product. One function per command; the types are the views of the
// elements (their view.rs).

import { Channel, invoke } from "@tauri-apps/api/core";

import type { AiEvent } from "./components/ai_work";
import type { PartRowView } from "./components/part_rows";

export type { AiEvent };

// evaluation_list/view.rs
/** An evaluation as a row of the list, with its progress. */
export interface EvaluationSummaryView {
  uuid: string;
  project_title: string;
  package_name: string;
  product: string;
  tasks: number;
  done: number;
}

// package_import/view.rs
/** The product of a new evaluation. */
export interface ImportForm {
  product: string;
  product_version: string;
}

// lab_tasks/view.rs
export type TaskState = "todo" | "working" | "done";

export interface LabTaskView {
  uuid: string;
  label: string;
  title: string;
  state: TaskState;
  /** How far it is (step 59): its verdict, accepted fragments, outputs due and ready. */
  verdict: Verdict | null;
  fragments: number;
  outputs_due: number;
  outputs_ready: number;
}

/** The progress of the whole evaluation (step 59). */
export interface ProgressView {
  tasks: number;
  done: number;
  met: number;
  not_met: number;
  inconclusive: number;
  no_verdict: number;
  outputs_due: number;
  outputs_ready: number;
}

export interface EvaluationView {
  uuid: string;
  project_title: string;
  scope: string;
  package_name: string;
  evaluator: string;
  note: string;
  product: string;
  product_version: string;
  tasks: LabTaskView[];
  progress: ProgressView;
  /** The agent with knowledge of the evaluation (step 57.4); null: none. */
  agent: number | null;
  agents: AgentOptionView[];
}

/** An agent with knowledge that may be chosen. */
export interface AgentOptionView {
  id: number;
  name: string;
}

// lab_task/view.rs
export interface LabBlockView {
  uuid: string;
  kind: "requirement" | "input" | "output";
  x: number;
  y: number;
  name: string;
  evidence: string;
  instructions: string;
  target: string;
  format: "text" | "latex";
  condition: "always" | "not_met" | "met";
}

export interface EvidenceView {
  uuid: string;
  kind: "pdf" | "text" | "note";
  name: string;
  pages: number;
  added: number;
}

export interface MessageView {
  speaker: "evaluator" | "model";
  text: string;
}

export interface FragmentView {
  uuid: string;
  evidence: string;
  page: number | null;
  quote: string;
  note: string;
  found: boolean;
  accepted: boolean;
  by_ai: boolean;
}

export interface InputView {
  block: string;
  evidence: EvidenceView[];
  messages: MessageView[];
  fragments: FragmentView[];
}

export type Verdict = "met" | "not_met" | "inconclusive";

/** The four results and the verdict, as the page gets and sends them. */
export interface ResultsForm {
  findings: string;
  analysis: string;
  assessment: string;
  vulnerabilities: string;
  verdict: Verdict | null;
}

export interface OutputView {
  block: string;
  text: string;
  proposal: string | null;
  /** Whether its condition holds for the verdict set. */
  applies: boolean;
}

/** The screen of a task. */
export interface LabTaskScreenView {
  uuid: string;
  state: TaskState;
  requirement: { label: string; title: string; catalog_title: string; parts: PartRowView[] };
  related: { label: string; title: string }[];
  blocks: LabBlockView[];
  connections: { from: string; to: string }[];
  inputs: InputView[];
  results: ResultsForm;
  proposal: { results: ResultsForm; reason: string } | null;
  outputs: OutputView[];
  result_titles: string[];
  knowledge: KnowledgeView;
}

/** A passage of the knowledge of the agent of a task; chosen: it goes into the question. */
export interface ContextPassageView {
  id: number;
  source: string;
  page: number | null;
  label: string;
  text: string;
  chosen: boolean;
}

/** The agent with knowledge of a task: how it is chosen, the one in force, its passages. */
export interface KnowledgeView {
  choice: "evaluation" | "none" | "agent";
  agent: number | null;
  used: string | null;
  agents: AgentOptionView[];
  passages: ContextPassageView[];
}

// fragments/view.rs
/** A fragment written by the evaluator. */
export interface FragmentForm {
  evidence: string;
  page: number | null;
  quote: string;
  note: string;
}

/** The instructions of a work of the AI of the view. */
export type LabWork = "input_chat" | "evidence_fragments" | "assessment" | "output_draft";

/** A channel whose events go to `onEvent`. */
function channel(onEvent: (event: AiEvent) => void): Channel<AiEvent> {
  const made = new Channel<AiEvent>();

  made.onmessage = onEvent;

  return made;
}

/** Called by EvaluationList.svelte, when the view opens. */
export function evaluationListGet(): Promise<EvaluationSummaryView[]> {
  return invoke("evaluation_list_get");
}

/** Called by EvaluationList.svelte, "Delete". */
export function evaluationListDelete(uuid: string): Promise<EvaluationSummaryView[]> {
  return invoke("evaluation_list_delete", { uuid });
}

/** The evaluations after an import, and what the evaluator should know of it. */
export interface ImportedView {
  evaluations: EvaluationSummaryView[];
  notices: string[];
}

/** Called by EvaluationList.svelte, "Import a work package…"; null when no file was chosen. */
export function packageImportImport(form: ImportForm): Promise<ImportedView | null> {
  return invoke("package_import_import", { form });
}

/** Called by EvaluationScreen.svelte, when an evaluation opens. */
export function labTasksGet(evaluation: string): Promise<EvaluationView> {
  return invoke("lab_tasks_get", { evaluation });
}

/** Called by EvaluationScreen.svelte, the state of a task. */
export function labTasksState(evaluation: string, task: string, progress: TaskState): Promise<EvaluationView> {
  return invoke("lab_tasks_state", { evaluation, task, progress });
}

/** Called by EvaluationScreen.svelte, the agent with knowledge of the evaluation. */
export function labTasksAgent(evaluation: string, agent: number | null): Promise<EvaluationView> {
  return invoke("lab_tasks_agent", { evaluation, agent });
}

/** Called by LabTask.svelte, when a task opens. */
export function labTaskGet(evaluation: string, task: string): Promise<LabTaskScreenView> {
  return invoke("lab_task_get", { evaluation, task });
}

/** Called by InputPanel.svelte, "Add a file…"; null when no file was chosen. */
export function evidenceAddFile(evaluation: string, block: string): Promise<LabTaskScreenView | null> {
  return invoke("evidence_add_file", { evaluation, block });
}

/** Called by InputPanel.svelte, "Add the text". */
export function evidenceAddNote(evaluation: string, block: string, name: string, text: string): Promise<LabTaskScreenView> {
  return invoke("evidence_add_note", { evaluation, block, name, text });
}

/** Called by InputPanel.svelte, "Delete" of a piece of evidence. */
export function evidenceDelete(evaluation: string, task: string, evidence: string): Promise<LabTaskScreenView> {
  return invoke("evidence_delete", { evaluation, task, evidence });
}

/** Called by InputPanel.svelte, "Send"; the events go to `onEvent`. */
export function inputChatSend(
  evaluation: string,
  block: string,
  text: string,
  onEvent: (event: AiEvent) => void,
): Promise<LabTaskScreenView> {
  return invoke("input_chat_send", { evaluation, block, text, onEvent: channel(onEvent) });
}

/** Called by InputPanel.svelte, "Clear the chat". */
export function inputChatClear(evaluation: string, block: string): Promise<LabTaskScreenView> {
  return invoke("input_chat_clear", { evaluation, block });
}

/** Called by InputPanel.svelte, "Find fragments with AI"; the events go to `onEvent`. */
export function fragmentsPropose(evaluation: string, block: string, onEvent: (event: AiEvent) => void): Promise<LabTaskScreenView> {
  return invoke("fragments_propose", { evaluation, block, onEvent: channel(onEvent) });
}

/** Called by InputPanel.svelte, "Accept" of a fragment. */
export function fragmentsAccept(evaluation: string, task: string, fragment: string): Promise<LabTaskScreenView> {
  return invoke("fragments_accept", { evaluation, task, fragment });
}

/** Called by InputPanel.svelte, "Reject" and "Remove" of a fragment. */
export function fragmentsRemove(evaluation: string, task: string, fragment: string): Promise<LabTaskScreenView> {
  return invoke("fragments_remove", { evaluation, task, fragment });
}

/** Called by InputPanel.svelte, "Add the fragment". */
export function fragmentsAdd(evaluation: string, block: string, fragment: FragmentForm): Promise<LabTaskScreenView> {
  return invoke("fragments_add", { evaluation, block, fragment });
}

/** Called by RequirementPanel.svelte, "Assess with AI"; the events go to `onEvent`. */
export function assessmentPropose(evaluation: string, task: string, onEvent: (event: AiEvent) => void): Promise<LabTaskScreenView> {
  return invoke("assessment_propose", { evaluation, task, onEvent: channel(onEvent) });
}

/** Called by RequirementPanel.svelte, "Accept the proposal". */
export function assessmentAccept(evaluation: string, task: string): Promise<LabTaskScreenView> {
  return invoke("assessment_accept", { evaluation, task });
}

/** Called by RequirementPanel.svelte, "Reject the proposal". */
export function assessmentReject(evaluation: string, task: string): Promise<LabTaskScreenView> {
  return invoke("assessment_reject", { evaluation, task });
}

/** Called by RequirementPanel.svelte, "Save". */
export function assessmentSave(evaluation: string, task: string, results: ResultsForm): Promise<LabTaskScreenView> {
  return invoke("assessment_save", { evaluation, task, results });
}

/** Called by RequirementPanel.svelte, the agent with knowledge of the task. */
export function assessmentAgent(
  evaluation: string,
  task: string,
  choice: KnowledgeView["choice"],
  agent: number | null,
): Promise<LabTaskScreenView> {
  return invoke("assessment_agent", { evaluation, task, choice, agent });
}

/** Called by RequirementPanel.svelte, the box of a passage of the agent. */
export function assessmentContext(evaluation: string, task: string, passage: number, chosen: boolean): Promise<LabTaskScreenView> {
  return invoke("assessment_context", { evaluation, task, passage, chosen });
}

/** Called by OutputPanel.svelte, "Draft with AI"; the events go to `onEvent`. */
export function outputsDraft(evaluation: string, block: string, onEvent: (event: AiEvent) => void): Promise<LabTaskScreenView> {
  return invoke("outputs_draft", { evaluation, block, onEvent: channel(onEvent) });
}

/** Called by OutputPanel.svelte, "Accept the draft". */
export function outputsAccept(evaluation: string, block: string): Promise<LabTaskScreenView> {
  return invoke("outputs_accept", { evaluation, block });
}

/** Called by OutputPanel.svelte, "Reject the draft". */
export function outputsReject(evaluation: string, block: string): Promise<LabTaskScreenView> {
  return invoke("outputs_reject", { evaluation, block });
}

/** Called by OutputPanel.svelte, "Save". */
export function outputsSave(evaluation: string, block: string, text: string): Promise<LabTaskScreenView> {
  return invoke("outputs_save", { evaluation, block, text });
}

/** Called by EvaluationScreen.svelte, "Export the report…"; the number of files, null when no folder was chosen. */
export function reportExport(evaluation: string): Promise<number | null> {
  return invoke("report_export", { evaluation });
}

/** Called by the panels of a task, "Stop". */
export function evaluationLabStop(): Promise<void> {
  return invoke("evaluation_lab_stop");
}

/** Called by LabInstructions.svelte, when it opens. */
export function evaluationLabInstructionsGet(which: LabWork): Promise<string> {
  return invoke("evaluation_lab_instructions_get", { which });
}

/** Called by LabInstructions.svelte, "Save" and "Default" (an empty text). */
export function evaluationLabInstructionsSave(which: LabWork, text: string): Promise<string> {
  return invoke("evaluation_lab_instructions_save", { which, text });
}
