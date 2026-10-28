//! An AI agent made in the program: which model it asks, what it is told
//! to be, and which knowledge it draws on. A work unit names its agent
//! (`WorkUnit::ai_agent`). Kept in the table `ai_agents`.

/// An AI agent (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct Agent {
    /// The id of the agent in the database.
    pub id: i64,

    /// The name the user gave it ("CC evaluator, ADV").
    pub name: String,

    /// The integration it asks (`Integration`).
    pub integration: i64,

    /// Its instructions: who it is and how it answers.
    pub instructions: String,

    /// The knowledge bases it draws on (`KnowledgeBase`).
    pub knowledge_bases: Vec<i64>,

    /// Its glossary, used to widen a search in its knowledge bases.
    pub glossary: Vec<GlossaryEntry>,

    /// The most passages of its knowledge given with one question.
    pub max_passages: u32,

    /// The most characters of its knowledge given with one question.
    pub max_characters: u32,
}

/// A term of the glossary of an agent and what it means ("TSF" = "TOE
/// Security Functionality").
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct GlossaryEntry {
    pub term: String,
    pub meaning: String,
}
