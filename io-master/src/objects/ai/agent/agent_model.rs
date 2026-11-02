//! An AI agent made in the program: which model it asks, what it is told
//! to be, which knowledge it draws on, and its role with the parameters of
//! its work (`objects/ai/agent_roles/agent_roles_model.rs`). A work unit names
//! its agent (`WorkUnit::ai_agent`). Kept in the table `ai_agents`.

use crate::objects::ai::agent_roles::agent_roles_model::AgentRole;

/// An AI agent (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    /// The id of the agent in the database.
    pub id: i64,

    /// The name the user gave it ("CC evaluator, ADV").
    pub name: String,

    /// The integration it asks (`Integration`).
    pub integration: i64,

    /// Its instructions: who it is and how it answers.
    pub instructions: String,

    /// The knowledge bases it searches (`KnowledgeBase` of `objects/ai/rag/rag_model.rs`).
    pub knowledge_bases: Vec<i64>,

    /// Its glossary, used to widen a search in its knowledge bases.
    pub glossary: Vec<GlossaryEntry>,

    /// The most passages of its knowledge given with one question.
    pub max_passages: u32,

    /// The most characters of its knowledge given with one question.
    pub max_characters: u32,

    /// The longest answer it asks for, in tokens, asked whole: a question
    /// that does not fit beside it into the context of its integration is
    /// refused (user, 2026-11-02).
    pub max_answer_tokens: u32,

    /// How its message to the model is laid out (the system prompt):
    /// `{instructions}`, `{knowledge}`, `{task}` (user, 2026-11-02).
    pub system_template: String,

    /// How its knowledge is introduced, when it gives any: `{passages}`.
    pub knowledge_template: String,

    /// Each passage of its knowledge: `{source}`, `{text}`.
    pub passage_template: String,

    /// What it is made for, with the parameters of that work (user,
    /// 2026-11-02); given when it is made, never changed.
    pub role: AgentRole,
}

/// A term of the glossary of an agent and what it means ("TSF" = "TOE
/// Security Functionality").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryEntry {
    pub term: String,
    pub meaning: String,
}
