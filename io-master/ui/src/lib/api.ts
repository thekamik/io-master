// The commands of the program (src/tauri_ui/commands.rs), as typed
// functions for the pages. Every call to Rust goes through this
// file, so the page and the program meet in one place.
//
// The types below mirror the views of src/tauri_ui/serializer/;
// a change there needs the same change here.

import { Channel, invoke } from "@tauri-apps/api/core";

/* ---------------------------------------------------------------
 * Errors
 * ------------------------------------------------------------- */

/** An error of a command (serializer/error_view.rs). */
export interface UiError {
  kind:
    | "not_installed"
    | "already_installed"
    | "not_logged_in"
    | "wrong_credentials"
    | "invalid"
    | "ai_unavailable"
    | "ai_refused"
    | "failed";
  message: string;
  /** For a field of an AI connection that breaks a rule: its name. */
  field: string | null;
}

/** Whether a rejected call carries an error of a command. */
export function isUiError(error: unknown): error is UiError {
  return typeof error === "object" && error !== null && "kind" in error && "message" in error;
}

/**
 * The sentence to show for a rejected call. The messages of the
 * program are written in lower case without a full stop, so that
 * they can be put inside other sentences; shown alone, they get
 * both.
 */
export function sentence(error: unknown): string {
  const text = isUiError(error) ? error.message : String(error);
  const capital = text.charAt(0).toUpperCase() + text.slice(1);

  return /[.!?]$/.test(capital) ? capital : `${capital}.`;
}

/* ---------------------------------------------------------------
 * The program and the account (serializer/app_view.rs)
 * ------------------------------------------------------------- */

export interface PageView {
  id: string;
  title: string;
}

export interface WarningView {
  level: "danger" | "warning";
  text: string;
}

export interface AppView {
  installed: boolean;
  role: "workstation" | "master" | null;
  user: string | null;
  pages: PageView[];
  warnings: WarningView[];
  /** The version of the program: "0.4.0". */
  version: string;
  /** The folder of the installation. */
  data_dir: string;
}

export function appState(): Promise<AppView> {
  return invoke("app_state");
}

export function setUp(fields: {
  name: string;
  password: string;
  passwordRepeat: string;
  role: string;
  roleConfirmed: boolean;
}): Promise<AppView> {
  return invoke("set_up", fields);
}

export function logIn(name: string, password: string): Promise<AppView> {
  return invoke("log_in", { name, password });
}

export function logOut(): Promise<AppView> {
  return invoke("log_out");
}

export function changePassword(current: string, password: string, passwordRepeat: string): Promise<void> {
  return invoke("change_password", { current, password, passwordRepeat });
}

/* ---------------------------------------------------------------
 * AI config (serializer/ai_config_view.rs)
 * ------------------------------------------------------------- */

/** The fields of a connection, as the page edits and sends them. */
export interface ConnectionFields {
  name: string;
  url: string;
  model: string;
  ca_certificate: string;
  headers: string;
  body: string;
  /** The JSON of one message in {{messages}}, with {{role}} and {{text}}. */
  message_template: string;
  user_role: string;
  model_role: string;
  system_role: string;
  system_prompt: string;
  answer: "stream" | "whole";
  text_path: string;
  stop_path: string;
  cut_reasons: string;
  refused_reasons: string;
  error_path: string;
  model_path: string;
  input_tokens_path: string;
  output_tokens_path: string;
  done_marker: string;
}

/** A connection as the program shows it; never with the key. */
export interface ConnectionView extends ConnectionFields {
  /** null: the template of a new connection. */
  id: number | null;
  has_key: boolean;
  /** A key is stored, but no header uses {{api_key}}. */
  key_unused: boolean;
}

/** Ready fields of a well-known service; choosing one only fills the form. */
export interface PresetView {
  id: string;
  title: string;
  description: string;
  fields: ConnectionView;
}

export interface AiConfigView {
  connections: ConnectionView[];
  /** The connection the chat uses. */
  active: number | null;
  template: ConnectionView;
  presets: PresetView[];
}

export interface SavedView {
  config: AiConfigView;
  id: number;
  notice: string;
}

/** The report of "Check connection"; the key masked everywhere. */
export interface CheckView {
  works: boolean;
  outcome: string;
  url: string;
  headers: { name: string; value: string }[];
  body: string;
  status: number | null;
  raw: string;
  text: string;
  model: string | null;
  input_tokens: number;
  output_tokens: number;
}

export function aiConfig(): Promise<AiConfigView> {
  return invoke("ai_config");
}

/** Saves a connection; `id` null: a new one. */
export function saveAiConnection(id: number | null, connection: ConnectionFields): Promise<SavedView> {
  return invoke("save_ai_connection", { id, connection });
}

export function deleteAiConnection(id: number): Promise<AiConfigView> {
  return invoke("delete_ai_connection", { id });
}

/** Makes a connection the one the chat uses. */
export function activateAiConnection(id: number): Promise<AiConfigView> {
  return invoke("activate_ai_connection", { id });
}

export function storeAiKey(id: number, apiKey: string): Promise<AiConfigView> {
  return invoke("store_ai_key", { id, apiKey });
}

export function deleteAiKey(id: number): Promise<AiConfigView> {
  return invoke("delete_ai_key", { id });
}

/** The instruction for a chatbot that helps to fill one field of the form. */
export function aiFieldInstruction(field: string, connection: ConnectionFields): Promise<string> {
  return invoke("ai_field_instruction", { field, connection });
}

/** Sends one short message over a stored connection. */
export function checkAiConnection(id: number): Promise<CheckView> {
  return invoke("check_ai_connection", { id });
}

/* ---------------------------------------------------------------
 * The scan (serializer/scan_view.rs)
 * ------------------------------------------------------------- */

export interface ColumnView {
  key: string;
  title: string;
}

export interface ScanView {
  settings: {
    all_types: boolean;
    types: string[];
    columns: string[];
    summary: string;
  };
  choices: {
    types: string[];
    columns: ColumnView[];
  };
  table: { columns: ColumnView[]; rows: string[][] } | null;
  problem: string | null;
}

export function scanPorts(): Promise<ScanView> {
  return invoke("scan_ports");
}

export function saveScanSettings(allTypes: boolean, types: string[], columns: string[]): Promise<ScanView> {
  return invoke("save_scan_settings", { allTypes, types, columns });
}

/* ---------------------------------------------------------------
 * The chat (serializer/chat_view.rs)
 * ------------------------------------------------------------- */

export interface ChatSummaryView {
  id: number;
  title: string;
  created: string;
}

export interface MessageView {
  speaker: "evaluator" | "model";
  who: string;
  text: string;
  created: string;
  notice: string | null;
}

export interface ChatView {
  id: number;
  title: string;
  messages: MessageView[];
}

/** An event of a reply being written. */
export type ReplyEvent =
  | { event: "chat"; id: number; title: string }
  | { event: "text"; text: string }
  | { event: "end"; ending: "complete" | "cut" | "stopped" | "refused" | "failed"; notice: string | null };

export function listChats(): Promise<ChatSummaryView[]> {
  return invoke("list_chats");
}

export function openChat(chatId: number): Promise<ChatView> {
  return invoke("open_chat", { chatId });
}

export function deleteChat(chatId: number): Promise<void> {
  return invoke("delete_chat", { chatId });
}

/**
 * Sends a message (`chatId` null: a new chat). The events of the
 * reply arrive in `onEvent` while it is written; the promise ends
 * when the reply is stored.
 */
export function sendMessage(chatId: number | null, text: string, onEvent: (event: ReplyEvent) => void): Promise<void> {
  const channel = new Channel<ReplyEvent>();

  channel.onmessage = onEvent;

  return invoke("send_message", { chatId, text, onEvent: channel });
}

/** Stops the reply being written. */
export function stopReply(): Promise<void> {
  return invoke("stop_reply");
}

/* ---------------------------------------------------------------
 * Create evaluation scheme: documents of standards
 * (serializer/standard_view.rs)
 * ------------------------------------------------------------- */

export interface DocumentView {
  id: number;
  title: string;
  part: string;
  version: string;
  file_name: string;
  sha256: string;
  page_count: number;
}

export interface DocumentFields {
  title: string;
  part: string;
  version: string;
}

/** An event of loading documents (also of an import). */
export type LoadEvent =
  | { event: "file"; name: string; index: number; count: number }
  | { event: "pages"; done: number; total: number }
  | { event: "loaded"; document: DocumentView }
  | { event: "failed"; name: string; message: string }
  | { event: "end"; stopped: boolean };

export interface HitView {
  document_id: number;
  page: number;
  snippet: string;
}

export function listDocuments(): Promise<DocumentView[]> {
  return invoke("list_documents");
}

/** Opens the dialog of the system for PDFs and loads them; the promise ends with the loading. */
export function loadDocuments(onEvent: (event: LoadEvent) => void): Promise<void> {
  const channel = new Channel<LoadEvent>();

  channel.onmessage = onEvent;

  return invoke("load_documents", { onEvent: channel });
}

/** Stops the long work running (loading, an analysis). */
export function stopWork(): Promise<void> {
  return invoke("stop_work");
}

export function updateDocument(id: number, document: DocumentFields): Promise<DocumentView> {
  return invoke("update_document", { id, document });
}

export function deleteDocument(id: number): Promise<void> {
  return invoke("delete_document", { id });
}

export function documentPage(id: number, page: number): Promise<string> {
  return invoke("document_page", { id, page });
}

export function searchDocuments(text: string, document: number | null): Promise<HitView[]> {
  return invoke("search_documents", { text, document });
}

/** The PDF of a document, as bytes, for the viewer. */
export function documentPdf(id: number): Promise<ArrayBuffer> {
  return invoke("document_pdf", { id });
}

/* ---------------------------------------------------------------
 * Create evaluation scheme: test projects and their schemes
 * (serializer/project_view.rs, serializer/scheme_view.rs)
 * ------------------------------------------------------------- */

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
