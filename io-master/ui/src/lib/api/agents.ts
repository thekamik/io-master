// The commands of the panel of the agents (src/panels/views/agents/), one
// function per command, named as the command: `agentGet` calls `agent_get`.
// The types mirror the views of those files; a change there needs the same
// change here.

import { invoke } from "@tauri-apps/api/core";

import type { EndingName } from "./settings";

/** An agent in the list (agent.rs, AgentSummaryView). */
export interface AgentSummaryView {
  id: number;
  name: string;
  integration: string;
  knowledgeBases: number;
}

/** An integration or a knowledge base to choose (agent.rs, ChoiceView). */
export interface ChoiceView {
  id: number;
  name: string;
}

/** A term of a glossary (agent.rs, TermView). */
export interface TermView {
  term: string;
  meaning: string;
}

/** An agent whole, with what its editor offers (agent.rs, AgentView). */
export interface AgentView {
  id: number;
  name: string;
  integration: number;
  instructions: string;
  knowledgeBases: number[];
  glossary: TermView[];
  maxPassages: number;
  maxCharacters: number;
  integrations: ChoiceView[];
  bases: ChoiceView[];
}

/** The fields of an agent as the page sends them (agent.rs, AgentFields). */
export interface AgentFields {
  id: number;
  name: string;
  integration: number;
  instructions: string;
  knowledgeBases: number[];
  glossary: TermView[];
  maxPassages: number;
  maxCharacters: number;
}

/** A knowledge base in the list (knowledge.rs, BaseView). */
export interface BaseView {
  id: number;
  name: string;
  description: string;
  sources: number;
}

/** A source of a base, without its text (knowledge.rs, SourceView). */
export interface SourceView {
  id: number;
  name: string;
  characters: number;
}

/** An entry of the journal of an agent (journal.rs, JournalRowView). */
export interface JournalRowView {
  id: number;
  time: number;
  integration: string;
  model: string;
  workUnit: number | null;
  ending: EndingName;
  inputTokens: number;
  outputTokens: number;
}

/** An entry of the journal whole (journal.rs, JournalEntryView). */
export interface JournalEntryView {
  row: JournalRowView;
  request: string;
  answer: string;
}

/** Every agent. */
export function agentList(): Promise<AgentSummaryView[]> {
  return invoke("agent_list");
}

/** Makes a new agent of an integration; gives its id. */
export function agentCreate(name: string, integration: number): Promise<number> {
  return invoke("agent_create", { name, integration });
}

/** An agent whole, with every integration and base to choose. */
export function agentGet(id: number): Promise<AgentView> {
  return invoke("agent_get", { id });
}

/** Saves an agent. */
export function agentSave(fields: AgentFields): Promise<void> {
  return invoke("agent_save", { fields });
}

/** The passages an agent would give with a question. */
export function agentKnowledgeTest(id: number, question: string): Promise<string[]> {
  return invoke("agent_knowledge_test", { id, question });
}

/** Every knowledge base. */
export function knowledgeBaseList(): Promise<BaseView[]> {
  return invoke("knowledge_base_list");
}

/** Makes a new knowledge base; gives its id. */
export function knowledgeBaseCreate(name: string, description: string): Promise<number> {
  return invoke("knowledge_base_create", { name, description });
}

/** Saves the name and the description of a base. */
export function knowledgeBaseSave(id: number, name: string, description: string): Promise<void> {
  return invoke("knowledge_base_save", { id, name, description });
}

/** The sources of a base. */
export function knowledgeSourceList(base: number): Promise<SourceView[]> {
  return invoke("knowledge_source_list", { base });
}

/** Adds a file chosen in the dialog to a base; null when it was closed. */
export function knowledgeSourceAdd(base: number): Promise<number | null> {
  return invoke("knowledge_source_add", { base });
}

/** Replaces the text of a source by a file chosen; false when none was. */
export function knowledgeSourceReplace(id: number): Promise<boolean> {
  return invoke("knowledge_source_replace", { id });
}

/** Changes the name of a source. */
export function knowledgeSourceRename(id: number, name: string): Promise<void> {
  return invoke("knowledge_source_rename", { id, name });
}

/** The passages of a base that fit a question. */
export function knowledgeSearch(base: number, question: string): Promise<string[]> {
  return invoke("knowledge_search", { base, question });
}

/** One page of the journal of an agent, older than `before`. */
export function agentJournalList(agent: number, before: number | null): Promise<JournalRowView[]> {
  return invoke("agent_journal_list", { agent, before });
}

/** An entry of the journal whole. */
export function agentJournalGet(id: number): Promise<JournalEntryView> {
  return invoke("agent_journal_get", { id });
}
