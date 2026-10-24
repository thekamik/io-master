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

import { invoke } from "@tauri-apps/api/core";

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
