// The test projects and their schemes (serializer/project_view.rs,
// serializer/scheme_view.rs; tauri_ui/commands/projects.rs, schemes.rs).
//
// The types mirror the views of src/tauri_ui/serializer/; a change
// there needs the same change here.

import { Channel, invoke } from "@tauri-apps/api/core";
import { type DocumentView, type LoadEvent } from "./documents";

export type TileKind = "input" | "requirement" | "output";

export interface SourceView {
  document_id: number;
  page: number;
  quote: string;
  /** Whether the quote is on its page; set by the program. */
  found: boolean;
}

/** The fields of a tile, as the page shows and sends them. */
export interface TileFields {
  kind: TileKind;
  identifier: string;
  title: string;
  description: string;
  place: string;
  state: "proposed" | "accepted";
  external: { document: string; place: string } | null;
  sections: string[];
  instructions: string;
  x: number;
  y: number;
  sources: SourceView[];
}

export interface TileView extends TileFields {
  id: number;
}

export interface LinkView {
  id: number;
  from: number;
  to: number;
  meaning: "needed_by" | "reported_in" | "provides" | "precedes";
}

export interface FindingView {
  kind: string;
  tile: number;
  message: string;
}

export interface SectionView {
  name: string;
  title: string;
  description: string;
}

/** A test project in the list. */
export interface ProjectView {
  id: number;
  name: string;
  version: string;
  description: string;
  author: string;
  documents: number[];
  ready: boolean;
}

export interface ProjectFields {
  name: string;
  version: string;
  description: string;
  author: string;
  documents: number[];
}

/** The pages of one document a scheme covers. */
export interface PageSpan {
  document_id: number;
  first_page: number;
  last_page: number;
}

/** The fields of a scheme (a chapter of the standard), as the page sends them. */
export interface SchemeFields {
  number: string;
  name: string;
  pages: PageSpan | null;
  state: "proposed" | "accepted";
}

/** A scheme in the list of its project, with what is in it. */
export interface SchemeSummaryView extends SchemeFields {
  id: number;
  tiles: number;
  requirements: number;
  findings: number;
  proposed: number;
}

/** An open project: its fields and the list of its schemes. */
export interface ProjectDataView {
  project: ProjectView;
  schemes: SchemeSummaryView[];
  documents: DocumentView[];
  findings: number;
  can_undo: boolean;
}

/** A tile of another scheme linked to the scheme shown: drawn as a shadow. */
export interface ShadowView {
  id: number;
  scheme: number;
  scheme_number: string;
  scheme_name: string;
  kind: TileKind;
  identifier: string;
  title: string;
}

/** An open scheme: everything the drawing needs. */
export interface SchemeDataView {
  project: ProjectView;
  scheme: SchemeSummaryView;
  schemes: SchemeSummaryView[];
  tiles: TileView[];
  shadows: ShadowView[];
  links: LinkView[];
  findings: FindingView[];
  documents: DocumentView[];
  sections: SectionView[];
  can_undo: boolean;
}

export interface PageRange {
  document_id: number;
  first_page: number;
  last_page: number;
}

/** An event of a work with AI (an analysis, a table of contents). */
export type AnalysisEvent =
  | { event: "scheme"; index: number; count: number; number: string; name: string }
  | { event: "part"; index: number; count: number; document: string; first_page: number; last_page: number }
  | { event: "writing"; characters: number }
  | { event: "applied"; added: number; joined: number; links: number; dropped: string[] }
  | { event: "chapters"; added: number; known: number; without_pages: number; dropped: string[] }
  | { event: "failed"; message: string }
  | { event: "end"; stopped: boolean };

/** The name of a scheme as the lists show it: "9.1 Identification". */
export function schemeName(scheme: { number: string; name: string }): string {
  return `${scheme.number} ${scheme.name}`.trim();
}

export function listProjects(): Promise<ProjectView[]> {
  return invoke("list_projects");
}

export function createProject(project: ProjectFields): Promise<ProjectDataView> {
  return invoke("create_project", { project });
}

export function openProject(id: number): Promise<ProjectDataView> {
  return invoke("open_project", { id });
}

export function updateProject(id: number, project: ProjectFields): Promise<ProjectDataView> {
  return invoke("update_project", { id, project });
}

export function setProjectReady(id: number, ready: boolean): Promise<ProjectDataView> {
  return invoke("set_project_ready", { id, ready });
}

export function deleteProject(id: number): Promise<void> {
  return invoke("delete_project", { id });
}

/** Only the fields of a scheme the program takes (it refuses any other). */
export function schemeFields(scheme: SchemeFields): SchemeFields {
  return {
    number: scheme.number,
    name: scheme.name,
    pages: scheme.pages
      ? { document_id: scheme.pages.document_id, first_page: scheme.pages.first_page, last_page: scheme.pages.last_page }
      : null,
    state: scheme.state,
  };
}

export function createScheme(project: number, scheme: SchemeFields): Promise<ProjectDataView> {
  return invoke("create_scheme", { project, scheme: schemeFields(scheme) });
}

export function updateScheme(id: number, scheme: SchemeFields): Promise<ProjectDataView> {
  return invoke("update_scheme", { id, scheme: schemeFields(scheme) });
}

export function acceptSchemes(project: number, ids: number[]): Promise<ProjectDataView> {
  return invoke("accept_schemes", { project, ids });
}

export function deleteSchemes(project: number, ids: number[]): Promise<ProjectDataView> {
  return invoke("delete_schemes", { project, ids });
}

/** Reads the table of contents on `range` with AI: its chapters become proposed schemes. */
export function proposeSchemes(
  project: number,
  range: PageRange,
  onEvent: (event: AnalysisEvent) => void,
): Promise<ProjectDataView> {
  const channel = new Channel<AnalysisEvent>();

  channel.onmessage = onEvent;

  return invoke("propose_schemes", { project, range, onEvent: channel });
}

export function openScheme(id: number): Promise<SchemeDataView> {
  return invoke("open_scheme", { id });
}

/** Only the fields the program takes (it refuses any other). */
export function tileFields(tile: TileFields): TileFields {
  return {
    kind: tile.kind,
    identifier: tile.identifier,
    title: tile.title,
    description: tile.description,
    place: tile.place,
    state: tile.state,
    external: tile.external ? { document: tile.external.document, place: tile.external.place } : null,
    sections: [...tile.sections],
    instructions: tile.instructions,
    x: tile.x,
    y: tile.y,
    sources: tile.sources.map((source) => ({ ...source })),
  };
}

export function addTile(scheme: number, tile: TileFields): Promise<SchemeDataView> {
  return invoke("add_tile", { scheme, tile: tileFields(tile) });
}

export function updateTile(id: number, tile: TileFields): Promise<SchemeDataView> {
  return invoke("update_tile", { id, tile: tileFields(tile) });
}

export function moveTile(id: number, x: number, y: number): Promise<void> {
  return invoke("move_tile", { id, x, y });
}

export function deleteTiles(scheme: number, ids: number[]): Promise<SchemeDataView> {
  return invoke("delete_tiles", { scheme, ids });
}

export function acceptTiles(scheme: number, ids: number[]): Promise<SchemeDataView> {
  return invoke("accept_tiles", { scheme, ids });
}

export function addLink(scheme: number, from: number, to: number): Promise<SchemeDataView> {
  return invoke("add_link", { scheme, from, to });
}

export function deleteLink(scheme: number, id: number): Promise<SchemeDataView> {
  return invoke("delete_link", { scheme, id });
}

export function undoScheme(scheme: number): Promise<SchemeDataView> {
  return invoke("undo_scheme", { scheme });
}

/** Moves tiles to the scheme `to`; gives the scheme `scheme` shown back. */
export function moveTiles(scheme: number, ids: number[], to: number): Promise<SchemeDataView> {
  return invoke("move_tiles", { scheme, ids, to });
}

/** Asks where to save, and writes the file of the project; false when the dialog was closed. */
export function exportProject(id: number, withPdfs: boolean): Promise<boolean> {
  return invoke("export_project", { id, withPdfs });
}

/** Asks for a file and imports the project; null when the dialog was closed. */
export function importProject(onEvent: (event: LoadEvent) => void): Promise<ProjectDataView | null> {
  const channel = new Channel<LoadEvent>();

  channel.onmessage = onEvent;

  return invoke("import_project", { onEvent: channel });
}

/** Which instructions of the AI: the analysis of a chapter, or reading a table of contents. */
export type Instructions = "chapter" | "contents";

export function analysisInstructions(which: Instructions): Promise<string> {
  return invoke("analysis_instructions", { which });
}

/** Stores the instructions; empty text brings the default back. */
export function saveAnalysisInstructions(which: Instructions, text: string): Promise<string> {
  return invoke("save_analysis_instructions", { which, text });
}

/** Analyses pages of the documents for one scheme with AI (no ranges: its own pages); "Stop" is stopWork(). */
export function analyseScheme(
  scheme: number,
  ranges: PageRange[],
  partSize: number,
  onEvent: (event: AnalysisEvent) => void,
): Promise<SchemeDataView> {
  const channel = new Channel<AnalysisEvent>();

  channel.onmessage = onEvent;

  return invoke("analyse_scheme", { scheme, ranges, partSize, onEvent: channel });
}

/** Analyses several schemes of a project, each on its own pages; "Stop" is stopWork(). */
export function analyseSchemes(
  project: number,
  ids: number[],
  partSize: number,
  onEvent: (event: AnalysisEvent) => void,
): Promise<ProjectDataView> {
  const channel = new Channel<AnalysisEvent>();

  channel.onmessage = onEvent;

  return invoke("analyse_schemes", { project, ids, partSize, onEvent: channel });
}
