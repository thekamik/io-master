// The commands of the view Knowledge (RAG) (src/views/knowledge/knowledge.rs,
// step 57): the knowledge bases of the AI, their sources and the search of
// their passages. One function per command; the types are the views of the
// elements (their view.rs).

import { invoke } from "@tauri-apps/api/core";

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
