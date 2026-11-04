// The commands of the panel Reports (src/panels/views/reports/), one
// function per command, named as the command: `reportPreview` calls
// `report_preview`. The types mirror the views of those files; a change
// there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

/** An evaluation to report on (report.rs, ReportSummaryView). */
export interface ReportSummaryView {
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

/** A template in the list (templates.rs, TemplateSummaryView). */
export interface TemplateSummaryView {
  id: number;
  name: string;
  extension: string;
}

/** A template whole (templates.rs, TemplateView). */
export interface TemplateView {
  id: number;
  name: string;
  extension: string;
  text: string;
  warnings: string[];
}

/** The markers of the evaluation, or the block of a list with the markers
 * of its elements (templates.rs, MarkerGroupView); `opening` and `closing`
 * are empty outside every block. */
export interface MarkerGroupView {
  opening: string;
  closing: string;
  markers: string[];
}

/** Every evaluation and report, the newest first. Called by ReportsTab.svelte. */
export function reportList(): Promise<ReportSummaryView[]> {
  return invoke("report_list");
}

/** The report of an evaluation by a template. Called by EvaluationReport.svelte. */
export function reportPreview(evaluation: number, template: number): Promise<string> {
  return invoke("report_preview", { evaluation, template });
}

/** Writes the report into a file chosen in the dialog of the system; gives
 * whether it was written. Called by EvaluationReport.svelte. */
export function reportExport(evaluation: number, template: number): Promise<boolean> {
  return invoke("report_export", { evaluation, template });
}

/** Every template, by their names. Called by TemplatesTab.svelte and EvaluationReport.svelte. */
export function reportTemplateList(): Promise<TemplateSummaryView[]> {
  return invoke("report_template_list");
}

/** A new template with the default text; gives its id. Called by TemplatesTab.svelte. */
export function reportTemplateCreate(name: string): Promise<number> {
  return invoke("report_template_create", { name });
}

/** A template with the warnings of its markers. Called by TemplateEditor.svelte. */
export function reportTemplateGet(id: number): Promise<TemplateView> {
  return invoke("report_template_get", { id });
}

/** Saves a template; gives the warnings of its markers. Called by TemplateEditor.svelte. */
export function reportTemplateSave(id: number, name: string, extension: string, text: string): Promise<string[]> {
  return invoke("report_template_save", { id, name, extension, text });
}

/** A new template of a file chosen in the dialog of the system; null when it
 * was closed. Called by TemplatesTab.svelte. */
export function reportTemplateImport(): Promise<number | null> {
  return invoke("report_template_import");
}

/** Writes a template into a file chosen in the dialog of the system; gives
 * whether it was written. Called by TemplateEditor.svelte. */
export function reportTemplateExport(id: number): Promise<boolean> {
  return invoke("report_template_export", { id });
}

/** The markers the editor offers, by block. Called by TemplateEditor.svelte. */
export function reportMarkers(): Promise<MarkerGroupView[]> {
  return invoke("report_markers");
}
