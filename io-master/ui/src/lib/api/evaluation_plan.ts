// The commands of the panel of the plan (src/panels/views/evaluation_plan/),
// one function per command, named as the command: `planGet` calls
// `plan_get`. The types mirror the views of those files; a change there
// needs the same change here.

import { invoke } from "@tauri-apps/api/core";

/** A standard (standards.rs, StandardView). */
export interface StandardView {
  id: number;
  title: string;
  version: string;
}

/** A plan in planning in the list (plans.rs, PlanSummaryView). */
export interface PlanSummaryView {
  id: number;
  name: string;
  workUnits: number;
  blocked: boolean;
}

/** A standard of the user and whether the plan has it (plans.rs). */
export interface StandardChoiceView {
  id: number;
  title: string;
  chosen: boolean;
}

/** A place of a document of the product (plans.rs). */
export interface DocumentView {
  id: number;
  name: string;
}

/** A verdict as the program names it. */
export type VerdictName = "pass" | "fail" | "inconclusive";

/** The data of a plan (plans.rs, PlanView). */
export interface PlanView {
  id: number;
  name: string;
  description: string;
  standards: StandardChoiceView[];
  documents: DocumentView[];
  verdicts: VerdictName[];
  blocked: boolean;
}

/** A work unit in the list of a plan (work_units.rs, WorkUnitRowView). */
export interface WorkUnitRowView {
  id: number;
  label: string;
  standard: string;
  requirement: string;
  functional: boolean;
  dependencies: number[];
  crossReference: boolean;
}

/** A control of a standard the plan may take (work_units.rs, ControlView). */
export interface ControlView {
  oscalKey: string;
  label: string;
  requirement: string;
}

/** A place of a proof of the laboratory (work_units.rs, ProofView). */
export interface ProofView {
  id: number;
  name: string;
}

/** A work unit for its form (work_units.rs, WorkUnitView). */
export interface WorkUnitView {
  id: number;
  label: string;
  requirement: string;
  guidance: string;
  dependencies: number[];
  documents: number[];
  proofs: ProofView[];
  aiAgent: number | null;
  /** Every agent, to choose the one that works on the unit. */
  agents: { id: number; name: string }[];
  crossReference: boolean;
}

/** Every standard. Called by EvaluationPlan.svelte. */
export function standardList(): Promise<StandardView[]> {
  return invoke("standard_list");
}

/** The dialog of the system, then the import; null when it was closed. Called by EvaluationPlan.svelte. */
export function standardImport(): Promise<number | null> {
  return invoke("standard_import");
}

/** The plans in planning. Called by EvaluationPlan.svelte. */
export function planList(): Promise<PlanSummaryView[]> {
  return invoke("plan_list");
}

/** A new plan; gives its id. Called by EvaluationPlan.svelte. */
export function planCreate(name: string, description: string): Promise<number> {
  return invoke("plan_create", { name, description });
}

/** The data of a plan. Called by PlanForm.svelte. */
export function planGet(id: number): Promise<PlanView> {
  return invoke("plan_get", { id });
}

/** Saves the data of a plan. Called by PlanForm.svelte. */
export function planSave(fields: {
  id: number;
  name: string;
  description: string;
  standards: number[];
  documents: number[];
  verdicts: VerdictName[];
}): Promise<void> {
  return invoke("plan_save", fields);
}

/** A new place of a document in a plan; gives its id. Called by PlanForm.svelte. */
export function planAddDocument(id: number, name: string): Promise<number> {
  return invoke("plan_add_document", { id, name });
}

/** The work units of a plan. Called by WorkUnits.svelte. */
export function workUnitList(plan: number): Promise<WorkUnitRowView[]> {
  return invoke("work_unit_list", { plan });
}

/** The controls of a standard the plan does not hold. Called by WorkUnits.svelte. */
export function workUnitControls(plan: number, standard: number): Promise<ControlView[]> {
  return invoke("work_unit_controls", { plan, standard });
}

/** Adds one work unit; gives its id. Called by WorkUnits.svelte. */
export function workUnitAdd(plan: number, standard: number, oscalKey: string): Promise<number> {
  return invoke("work_unit_add", { plan, standard, oscalKey });
}

/** Adds every control not held yet; gives how many. Called by WorkUnits.svelte. */
export function workUnitAddAll(plan: number, standard: number): Promise<number> {
  return invoke("work_unit_add_all", { plan, standard });
}

/** Removes a work unit from its plan. Called by WorkUnits.svelte. */
export function workUnitRemove(plan: number, unit: number): Promise<void> {
  return invoke("work_unit_remove", { plan, unit });
}

/** A work unit for its form. Called by WorkUnitForm.svelte. */
export function workUnitGet(unit: number): Promise<WorkUnitView> {
  return invoke("work_unit_get", { unit });
}

/** Saves the parameters of a work unit. Called by WorkUnitForm.svelte. */
export function workUnitSave(fields: {
  plan: number;
  unit: number;
  dependencies: number[];
  documents: number[];
  proofs: number[];
  aiAgent: number | null;
  guidance: string;
}): Promise<void> {
  return invoke("work_unit_save", fields);
}

/** A new place of a proof for a work unit; gives its id. Called by WorkUnitForm.svelte. */
export function workUnitAddProof(plan: number, unit: number, name: string): Promise<number> {
  return invoke("work_unit_add_proof", { plan, unit, name });
}
