// The commands of the view Evaluation plan, one function per command of its
// entry in the program (src/views/evaluation_plan/evaluation_plan.rs),
// named as the command: `oscalDocumentsGet` calls `oscal_documents_get`.
// The view works only on the documents of OSCAL: the master projects (each a
// list of tasks, every task a whole requirement) and the documents of OSCAL;
// the catalogs are made from the PDFs of a standard in the view Converter
// (converter.ts).
//
// The types mirror the view.rs of the elements (src/views/evaluation_plan/);
// a change there needs the same change here.

import { Channel, invoke } from "@tauri-apps/api/core";

import type { PartRowView } from "./components/part_rows";

// project_list/view.rs
/** A master project, as a row of the list. */
export interface ProjectSummaryView {
  uuid: string;
  title: string;
  catalogs: number;
  tasks: number;
}

// new_project/view.rs
/** A catalog a project can use (a finished one). */
export interface CatalogChoiceView {
  uuid: string;
  title: string;
  version: string;
}

// project_form/view.rs
/** A master project in its form. */
export interface ProjectView {
  uuid: string;
  title: string;
  description: string;
  catalogs: CatalogChoiceView[];
  available: CatalogChoiceView[];
}

/** What the page sends of a project. */
export interface ProjectForm {
  title: string;
  description: string;
  catalogs: string[];
}

// task_list/view.rs
/** A task: a whole requirement. */
export interface TaskView {
  uuid: string;
  catalog_uuid: string;
  catalog_title: string;
  control_id: string;
  label: string;
  title: string;
  /** The name of the work package the task is in. */
  package: string | null;
}

// standard_browser/view.rs
export interface QuoteView {
  document_id: number;
  page: number;
  text: string;
  found: boolean;
}

/** A requirement of a catalog of the project. */
export interface RequirementView {
  catalog_uuid: string;
  id: string;
  label: string;
  title: string;
  parts: PartRowView[];
  quotes: QuoteView[];
  in_project: boolean;
  /** For a requirement of the laboratory, its fields to be changed. */
  own: OwnRequirement | null;
}

// task_scheme/view.rs
export type BlockKind = "requirement" | "input" | "output";
export type OutputFormat = "text" | "latex";
export type OutputCondition = "always" | "not_met" | "met";

/** A block of the scheme of a task. */
export interface BlockView {
  uuid: string;
  kind: BlockKind;
  x: number;
  y: number;
  name: string;
  evidence: string;
  instructions: string;
  target: string;
  format: OutputFormat;
  condition: OutputCondition;
  /** The reason of the AI while the block is only proposed (drawn dashed). */
  proposal: string | null;
  /** The correction the AI proposes for the block, waiting for the planner. */
  correction: CorrectionView | null;
}

/** A correction of a block the AI proposes. */
export interface CorrectionView {
  name: string;
  evidence: string;
  instructions: string;
  reason: string;
}

// components/ai_work/view.rs
export type { AiEvent } from "./components/ai_work";
import type { AiEvent } from "./components/ai_work";

/** The instructions of a work of the AI of the view. */
export type AiWork = "task_scheme" | "requirement_choice" | "related_elements";

/** The fields of a block, as the page sends them. */
export interface BlockForm {
  name: string;
  evidence: string;
  instructions: string;
  target: string;
  format: OutputFormat;
  condition: OutputCondition;
}

/** The scheme of a task. */
export interface SchemeView {
  task: TaskView;
  requirement: RequirementView;
  blocks: BlockView[];
  connections: { from: string; to: string }[];
  /** The titles of the four text results of the requirement. */
  results: string[];
}

// standard_outputs/view.rs
/** An output of the template of a project ("Standard outputs"). */
export interface StandardOutputView {
  uuid: string;
  name: string;
  instructions: string;
  target: string;
  format: OutputFormat;
  condition: OutputCondition;
}

/** The fields of an output of the template, as the page sends them. */
export interface StandardOutputForm {
  name: string;
  instructions: string;
  target: string;
  format: OutputFormat;
  condition: OutputCondition;
}

// task_proposals/view.rs
/** A requirement the AI proposes as a task, with its reason. */
export interface TaskProposalView {
  catalog_uuid: string;
  catalog_title: string;
  control_id: string;
  label: string;
  title: string;
  reason: string;
}

// task_links/view.rs
/** An element of another catalog related to a task. */
export interface TaskLinkView {
  catalog_uuid: string;
  catalog_title: string;
  control_id: string;
  label: string;
  title: string;
  reason: string;
  /** Whether it is only proposed by the AI. */
  proposed: boolean;
}

// work_packages/view.rs
/** A work package: the tasks delegated to one evaluator. */
export interface WorkPackageView {
  uuid: string;
  name: string;
  evaluator: string;
  note: string;
  tasks: string[];
}

/** A work package, as the page sends it. */
export interface WorkPackageForm {
  name: string;
  evaluator: string;
  note: string;
  tasks: string[];
}

// own_requirement/view.rs
/** A requirement of the laboratory's own; its identifier is the planner's. */
export interface OwnRequirement {
  identifier: string;
  title: string;
  statement: string;
}

/** A requirement found by a search. */
export interface HitView {
  catalog_uuid: string;
  catalog_title: string;
  id: string;
  label: string;
  title: string;
}

// oscal_documents/view.rs
export type OscalModel =
  | "catalog"
  | "profile"
  | "mapping-collection"
  | "assessment-plan"
  | "assessment-results";

export interface OscalDocumentView {
  uuid: string;
  model: OscalModel;
  /** The name of the model for the evaluator ("Catalog", ...). */
  model_title: string;
  title: string;
  version: string;
  oscal_version: string;
  last_modified: string;
}

// ── When the view opens ──────────────────────────────────────────────

/** Called by ProjectList.svelte, when the view opens and after a change. */
export function projectListGet(): Promise<ProjectSummaryView[]> {
  return invoke("project_list_get");
}

/** Called by ProjectForm.svelte, when a project opens. */
export function projectFormGet(uuid: string): Promise<ProjectView> {
  return invoke("project_form_get", { uuid });
}

/** Called by TaskList.svelte, when a project opens and after a change. */
export function taskListGet(project: string): Promise<TaskView[]> {
  return invoke("task_list_get", { project });
}

/** Called by OscalDocuments.svelte, when the view opens and after a change. */
export function oscalDocumentsGet(): Promise<OscalDocumentView[]> {
  return invoke("oscal_documents_get");
}

// ── The actions of the elements ──────────────────────────────────────

/** Asks for a file in a dialog of the system; null when none was chosen. Called by OscalDocuments.svelte, "Import an OSCAL document". */
export function oscalDocumentsImport(): Promise<OscalDocumentView | null> {
  return invoke("oscal_documents_import");
}

/** Asks where to write the document; false when no place was chosen. Called by OscalDocuments.svelte, "Export". */
export function oscalDocumentsExport(
  uuid: string,
  fileName: string,
): Promise<boolean> {
  return invoke("oscal_documents_export", { uuid, fileName });
}

/** Called by OscalDocuments.svelte, "Delete". */
export function oscalDocumentsDelete(uuid: string): Promise<void> {
  return invoke("oscal_documents_delete", { uuid });
}

/** Called by NewProject.svelte, when its dialog opens. */
export function newProjectCatalogs(): Promise<CatalogChoiceView[]> {
  return invoke("new_project_catalogs");
}

/** Called by NewProject.svelte, "Make". */
export function newProjectCreate(project: ProjectForm): Promise<ProjectSummaryView> {
  return invoke("new_project_create", { project });
}

/** Called by ProjectList.svelte, "Delete". */
export function projectListDelete(uuid: string): Promise<void> {
  return invoke("project_list_delete", { uuid });
}

/** Called by ProjectForm.svelte, "Save". */
export function projectFormSave(uuid: string, project: ProjectForm): Promise<ProjectView> {
  return invoke("project_form_save", { uuid, project });
}

/** Called by StandardBrowser.svelte, "Add as a task". */
export function taskListAdd(project: string, catalog: string, control: string): Promise<TaskView[]> {
  return invoke("task_list_add", { project, catalog, control });
}

/** Called by TaskList.svelte, "✕". */
export function taskListRemove(task: string): Promise<TaskView[]> {
  return invoke("task_list_remove", { task });
}

/** Called by TaskList.svelte, "↑" and "↓". */
export function taskListMove(task: string, up: boolean): Promise<TaskView[]> {
  return invoke("task_list_move", { task, up });
}

/** Called by StandardBrowser.svelte, a requirement chosen. */
export function standardBrowserGet(project: string, catalog: string, id: string): Promise<RequirementView> {
  return invoke("standard_browser_get", { project, catalog, id });
}

/** Called by StandardBrowser.svelte, "Search". */
export function standardBrowserSearch(project: string, text: string): Promise<HitView[]> {
  return invoke("standard_browser_search", { project, text });
}

/** Called by OwnRequirement.svelte, "Add". */
export function ownRequirementAdd(project: string, requirement: OwnRequirement): Promise<TaskView[]> {
  return invoke("own_requirement_add", { project, requirement });
}

/** Called by OwnRequirement.svelte, "Save". */
export function ownRequirementSave(id: string, requirement: OwnRequirement): Promise<void> {
  return invoke("own_requirement_save", { id, requirement });
}

/** Called by StandardBrowser.svelte, "Delete" of a requirement of the laboratory. */
export function ownRequirementDelete(id: string): Promise<void> {
  return invoke("own_requirement_delete", { id });
}

/** Called by TaskScheme.svelte, when a task opens. */
export function taskSchemeGet(task: string): Promise<SchemeView> {
  return invoke("task_scheme_get", { task });
}

/** Called by TaskScheme.svelte, "+ Input" and "+ Output". */
export function taskSchemeAddBlock(task: string, kind: "input" | "output", x: number, y: number): Promise<SchemeView> {
  return invoke("task_scheme_add_block", { task, kind, x, y });
}

/** Called by BlockForm.svelte, "Save". */
export function taskSchemeSaveBlock(block: string, fields: BlockForm): Promise<SchemeView> {
  return invoke("task_scheme_save_block", { block, fields });
}

/** Called by TaskScheme.svelte, the end of a drag. */
export function taskSchemeMoveBlock(block: string, x: number, y: number): Promise<void> {
  return invoke("task_scheme_move_block", { block, x, y });
}

/** Called by BlockForm.svelte, "Remove". */
export function taskSchemeRemoveBlock(block: string): Promise<SchemeView> {
  return invoke("task_scheme_remove_block", { block });
}

/** Called by TaskScheme.svelte, a connection drawn. */
export function taskSchemeConnect(from: string, to: string): Promise<SchemeView> {
  return invoke("task_scheme_connect", { from, to });
}

/** Called by TaskScheme.svelte, a connection removed. */
export function taskSchemeDisconnect(from: string, to: string): Promise<SchemeView> {
  return invoke("task_scheme_disconnect", { from, to });
}

/** Called by StandardOutputs.svelte, when a project opens. */
export function standardOutputsGet(project: string): Promise<StandardOutputView[]> {
  return invoke("standard_outputs_get", { project });
}

/** Called by StandardOutputs.svelte, "Add". */
export function standardOutputsAdd(project: string, output: StandardOutputForm): Promise<StandardOutputView[]> {
  return invoke("standard_outputs_add", { project, output });
}

/** Called by StandardOutputs.svelte, "Save". */
export function standardOutputsSave(output: string, fields: StandardOutputForm): Promise<StandardOutputView[]> {
  return invoke("standard_outputs_save", { output, fields });
}

/** Called by StandardOutputs.svelte, "Delete". */
export function standardOutputsDelete(output: string): Promise<StandardOutputView[]> {
  return invoke("standard_outputs_delete", { output });
}

/** Called by TaskScheme.svelte, "Propose with AI"; the events go to `onEvent`. */
export function taskSchemePropose(task: string, onEvent: (event: AiEvent) => void): Promise<SchemeView> {
  const channel = new Channel<AiEvent>();

  channel.onmessage = onEvent;

  return invoke("task_scheme_propose", { task, onEvent: channel });
}

/** Called by TaskScheme.svelte, TaskProposals.svelte, TaskLinks.svelte, "Stop". */
export function evaluationPlanStop(): Promise<void> {
  return invoke("evaluation_plan_stop");
}

/** Called by BlockForm.svelte, "Accept" of a proposal. */
export function taskSchemeAccept(block: string): Promise<SchemeView> {
  return invoke("task_scheme_accept", { block });
}

/** Called by BlockForm.svelte, "Reject" of a proposal. */
export function taskSchemeReject(block: string): Promise<SchemeView> {
  return invoke("task_scheme_reject", { block });
}

/** Called by TaskScheme.svelte, "Accept all". */
export function taskSchemeAcceptAll(task: string): Promise<SchemeView> {
  return invoke("task_scheme_accept_all", { task });
}

/** Called by TaskScheme.svelte, "Reject all". */
export function taskSchemeRejectAll(task: string): Promise<SchemeView> {
  return invoke("task_scheme_reject_all", { task });
}

/** Called by AiWorkInstructions.svelte, when it opens. */
export function aiWorkInstructionsGet(which: AiWork): Promise<string> {
  return invoke("ai_work_instructions_get", { which });
}

/** Called by AiWorkInstructions.svelte, "Save" and "Default" (an empty text). */
export function aiWorkInstructionsSave(which: AiWork, text: string): Promise<string> {
  return invoke("ai_work_instructions_save", { which, text });
}

/** Called by TaskProposals.svelte, when a project opens and after a change. */
export function taskProposalsGet(project: string): Promise<TaskProposalView[]> {
  return invoke("task_proposals_get", { project });
}

/** Called by TaskProposals.svelte, "Propose tasks with AI"; the events go to `onEvent`. */
export function taskProposalsPropose(
  project: string,
  catalog: string,
  onEvent: (event: AiEvent) => void,
): Promise<TaskProposalView[]> {
  const channel = new Channel<AiEvent>();

  channel.onmessage = onEvent;

  return invoke("task_proposals_propose", { project, catalog, onEvent: channel });
}

/** Called by TaskProposals.svelte, "Accept". */
export function taskProposalsAccept(project: string, catalog: string, control: string): Promise<TaskProposalView[]> {
  return invoke("task_proposals_accept", { project, catalog, control });
}

/** Called by TaskProposals.svelte, "Reject". */
export function taskProposalsReject(project: string, catalog: string, control: string): Promise<TaskProposalView[]> {
  return invoke("task_proposals_reject", { project, catalog, control });
}

/** Called by TaskProposals.svelte, "Accept all". */
export function taskProposalsAcceptAll(project: string): Promise<void> {
  return invoke("task_proposals_accept_all", { project });
}

/** Called by TaskProposals.svelte, "Reject all". */
export function taskProposalsRejectAll(project: string): Promise<void> {
  return invoke("task_proposals_reject_all", { project });
}

/** Called by TaskLinks.svelte, when a task opens. */
export function taskLinksGet(task: string): Promise<TaskLinkView[]> {
  return invoke("task_links_get", { task });
}

/** Called by TaskLinks.svelte, "Find with AI"; the events go to `onEvent`. */
export function taskLinksPropose(task: string, onEvent: (event: AiEvent) => void): Promise<TaskLinkView[]> {
  const channel = new Channel<AiEvent>();

  channel.onmessage = onEvent;

  return invoke("task_links_propose", { task, onEvent: channel });
}

/** Called by TaskLinks.svelte, "Accept". */
export function taskLinksAccept(task: string, catalog: string, control: string): Promise<TaskLinkView[]> {
  return invoke("task_links_accept", { task, catalog, control });
}

/** Called by TaskLinks.svelte, "Reject" and "Remove". */
export function taskLinksRemove(task: string, catalog: string, control: string): Promise<TaskLinkView[]> {
  return invoke("task_links_remove", { task, catalog, control });
}

/** Called by WorkPackages.svelte, when a project opens and after a change. */
export function workPackagesGet(project: string): Promise<WorkPackageView[]> {
  return invoke("work_packages_get", { project });
}

/** Called by WorkPackages.svelte, "Make". */
export function workPackagesAdd(project: string, workPackage: WorkPackageForm): Promise<WorkPackageView[]> {
  return invoke("work_packages_add", { project, package: workPackage });
}

/** Called by WorkPackages.svelte, "Save". */
export function workPackagesSave(workPackage: string, fields: WorkPackageForm): Promise<WorkPackageView[]> {
  return invoke("work_packages_save", { package: workPackage, fields });
}

/** Called by WorkPackages.svelte, "Delete". */
export function workPackagesDelete(workPackage: string): Promise<WorkPackageView[]> {
  return invoke("work_packages_delete", { package: workPackage });
}

/** Called by WorkPackages.svelte, "Export…"; false when no place was chosen. */
export function workPackagesExport(workPackage: string): Promise<boolean> {
  return invoke("work_packages_export", { package: workPackage });
}
