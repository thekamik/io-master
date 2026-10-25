// The commands of the view Knowledge (RAG) (src/views/knowledge/knowledge.rs,
// step 57): the knowledge bases of the AI, their sources and the search of
// their passages; the agents made of them and their proving ground. One
// function per command; the types are the views of the elements (their
// view.rs).

import { Channel, invoke } from "@tauri-apps/api/core";

import type { AiEvent } from "./components/ai_work";

export type { AiEvent };

// knowledge_list/view.rs
/** A knowledge base as a row of the list. */
export interface BaseSummaryView {
  id: number;
  name: string;
  description: string;
  sources: number;
  passages: number;
}

/** The name and the description of a base. */
export interface BaseForm {
  name: string;
  description: string;
}

// knowledge_base/view.rs
export interface SourceView {
  id: number;
  kind: "pdf" | "text" | "catalog" | "note";
  name: string;
  passages: number;
}

export interface BaseScreenView {
  id: number;
  name: string;
  description: string;
  sources: SourceView[];
}

export interface CatalogChoiceView {
  uuid: string;
  title: string;
  version: string;
}

// knowledge_search/view.rs
/** A passage found; score: BM25, the lower, the better it fits. */
export interface HitView {
  id: number;
  source: string;
  page: number | null;
  label: string;
  text: string;
  score: number;
}

// agent_list/view.rs
export interface AgentSummaryView {
  id: number;
  name: string;
  bases: number;
}

// agent_form/view.rs
/** A knowledge base in the form of an agent: ticked when the agent draws on it. */
export interface BaseChoiceView {
  id: number;
  name: string;
  passages: number;
  chosen: boolean;
}

export interface AgentFormView {
  id: number;
  name: string;
  instructions: string;
  glossary: string;
  passages: number;
  characters: number;
  bases: BaseChoiceView[];
  max_passages: number;
  min_characters: number;
  max_characters: number;
}

/** The form of an agent as it is saved. */
export interface AgentForm {
  name: string;
  instructions: string;
  glossary: string;
  passages: number;
  characters: number;
  bases: number[];
}

// proving_ground/view.rs
export interface QuoteView {
  passage: number;
  quote: string;
  found: boolean;
}

export interface TrialView {
  question: string;
  passages: HitView[];
  answer: string;
  quotes: QuoteView[];
}

/** A channel of Tauri that hands every event of a work of the AI to `onEvent`. */
function channel(onEvent: (event: AiEvent) => void): Channel<AiEvent> {
  const made = new Channel<AiEvent>();

  made.onmessage = onEvent;

  return made;
}

/** Called by KnowledgeList.svelte, when the view opens. */
export function knowledgeListGet(): Promise<BaseSummaryView[]> {
  return invoke("knowledge_list_get");
}

/** Called by KnowledgeList.svelte, "Make". */
export function knowledgeListCreate(form: BaseForm): Promise<BaseScreenView> {
  return invoke("knowledge_list_create", { form });
}

/** Called by KnowledgeList.svelte, "Delete". */
export function knowledgeListDelete(id: number): Promise<BaseSummaryView[]> {
  return invoke("knowledge_list_delete", { id });
}

/** Called by KnowledgeBase.svelte, when a base opens. */
export function knowledgeBaseGet(id: number): Promise<BaseScreenView> {
  return invoke("knowledge_base_get", { id });
}

/** Called by KnowledgeBase.svelte, "Save". */
export function knowledgeBaseSave(id: number, form: BaseForm): Promise<BaseScreenView> {
  return invoke("knowledge_base_save", { id, form });
}

/** Called by KnowledgeBase.svelte, "Add a file…"; null when no file was chosen. */
export function knowledgeBaseAddFile(id: number): Promise<BaseScreenView | null> {
  return invoke("knowledge_base_add_file", { id });
}

/** Called by KnowledgeBase.svelte, "Add the catalog". */
export function knowledgeBaseAddCatalog(id: number, catalog: string): Promise<BaseScreenView> {
  return invoke("knowledge_base_add_catalog", { id, catalog });
}

/** Called by KnowledgeBase.svelte, "Add the note". */
export function knowledgeBaseAddNote(id: number, name: string, text: string): Promise<BaseScreenView> {
  return invoke("knowledge_base_add_note", { id, name, text });
}

/** Called by KnowledgeBase.svelte, "Remove" of a source. */
export function knowledgeBaseRemove(id: number, source: number): Promise<BaseScreenView> {
  return invoke("knowledge_base_remove", { id, source });
}

/** Called by KnowledgeBase.svelte, when a base opens. */
export function knowledgeBaseCatalogs(): Promise<CatalogChoiceView[]> {
  return invoke("knowledge_base_catalogs");
}

/** Called by KnowledgeSearch.svelte, "Search". */
export function knowledgeSearchRun(base: number, text: string): Promise<HitView[]> {
  return invoke("knowledge_search_run", { base, text });
}

/** Called by AgentList.svelte, when the tab Agents opens. */
export function agentListGet(): Promise<AgentSummaryView[]> {
  return invoke("agent_list_get");
}

/** Called by AgentList.svelte, "Make". */
export function agentListCreate(name: string): Promise<AgentFormView> {
  return invoke("agent_list_create", { name });
}

/** Called by AgentList.svelte, "Delete". */
export function agentListDelete(id: number): Promise<AgentSummaryView[]> {
  return invoke("agent_list_delete", { id });
}

/** Called by AgentForm.svelte, when an agent opens. */
export function agentFormGet(id: number): Promise<AgentFormView> {
  return invoke("agent_form_get", { id });
}

/** Called by AgentForm.svelte, "Save". */
export function agentFormSave(id: number, form: AgentForm): Promise<AgentFormView> {
  return invoke("agent_form_save", { id, form });
}

/** Called by AgentForm.svelte, "Export…"; false when no place was chosen. */
export function agentFormExport(id: number): Promise<boolean> {
  return invoke("agent_form_export", { id });
}

/** Called by AgentList.svelte, "Import an agent…"; null when no file was chosen. */
export function agentListImport(): Promise<AgentFormView | null> {
  return invoke("agent_list_import");
}

/** Called by ProvingGround.svelte, "Show the passages". */
export function provingGroundFind(agent: number, question: string): Promise<HitView[]> {
  return invoke("proving_ground_find", { agent, question });
}

/** Called by ProvingGround.svelte, when an agent opens and after a question. */
export function provingGroundGet(agent: number): Promise<TrialView | null> {
  return invoke("proving_ground_get", { agent });
}

/** Called by ProvingGround.svelte, "Ask the agent". */
export function provingGroundAsk(agent: number, question: string, onEvent: (event: AiEvent) => void): Promise<void> {
  return invoke("proving_ground_ask", { agent, question, onEvent: channel(onEvent) });
}

/** Called by ProvingGround.svelte, "Stop". */
export function knowledgeStop(): Promise<void> {
  return invoke("knowledge_stop");
}
