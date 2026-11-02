//! The knowledge for the AI (RAG, retrieval-augmented generation): the
//! texts an agent searches (earlier evaluations, notes, anything),
//! grouped into knowledge bases, as files into folders. At a question
//! the program finds the pieces of the texts that fit it and gives only
//! them to the model with the question, since a whole text would not
//! fit into one question.
//!
//! A base groups its sources so that several agents share them: an agent
//! names its bases (`Agent::knowledge_bases`), and a source added to a
//! base reaches every agent of the base.
//!
//! The model is only the text. Its index (the text cut into passages of
//! a bounded length, searched by FTS5 of SQLite) is made from it by
//! `rag_store.rs` whenever the text is stored, and made again when it changes:
//! it is no data of its own, as the index of a table is none.
//!
//! ```text
//! Agent "ADV" ──┐
//!               ├──► KnowledgeBase ◄── Source (knowledge_base) ──text──► the index of its passages (rag_store.rs)
//! Agent "ATE" ──┘
//! ```

/// A knowledge base: a named group of sources (user, 2026-10-28). Kept
/// in the table `knowledge_bases`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeBase {
    /// The id of the base in the database.
    pub id: i64,

    /// The name of the base ("CEM and its interpretations").
    pub name: String,

    /// What the base is for.
    pub description: String,

    /// The longest passage its sources are cut into for the index, in
    /// characters (user, 2026-11-02); its sources are indexed anew when it
    /// changes.
    pub passage_characters: u32,

    /// The words a search of the base passes over, in lower case ("the",
    /// "shall"): words in almost every passage, which would find passages by
    /// themselves instead of by the terms that matter.
    pub stop_words: Vec<String>,
}

/// A source of knowledge: one text of a base (user, 2026-10-28). Kept in
/// the table `knowledge_sources`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// The id of the source in the database.
    pub id: i64,

    /// The base it belongs to (`KnowledgeBase`).
    pub knowledge_base: i64,

    /// The name of the source ("Evaluation of router X, 2025").
    pub name: String,

    /// The text of the source.
    pub text: String,
}
