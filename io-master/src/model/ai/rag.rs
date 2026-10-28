//! The knowledge for the AI (RAG, retrieval-augmented generation): the
//! texts an agent searches (earlier evaluations, notes, anything). At a
//! question the program finds the pieces of the texts of the agent that
//! fit it and gives only them to the model with the question, since a
//! whole text would not fit into one question.
//!
//! The model is only the text. Its index (the text cut into passages of
//! a bounded length, searched by FTS5 of SQLite) is made from it by
//! `db/` whenever the text is stored, and made again when it changes:
//! it is no data of its own, as the index of a table is none.
//!
//! ```text
//! Agent (sources) ──► Source ──text──► the index of its passages (db/)
//! ```

/// A source of knowledge for the AI (user, 2026-10-28). Kept in the
/// table `rag_sources`; an agent names the sources it searches
/// (`Agent::sources`), and one source may serve several agents.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct Source {
    /// The id of the source in the database.
    pub id: i64,

    /// The name of the source ("Evaluation of router X, 2025").
    pub name: String,

    /// The text of the source.
    pub text: String,
}
