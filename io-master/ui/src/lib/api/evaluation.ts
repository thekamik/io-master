// The commands of the panel of the evaluation (src/panels/views/evaluation/),
// one function per command, named as the command: `evaluationGet` calls
// `evaluation_get`. The types mirror the views of those files; a change
// there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

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
