//! The knowledge for the AI (RAG, retrieval-augmented generation): bases
//! of sources cut into passages. At a question the program finds the
//! passages that fit it and gives them to the model with the question.
//! The passages are searched in the database (FTS5) and never read into
//! memory all at once, so a base and a source name what they belong to
//! by its id, and hold no list of what belongs to them.
//!
//! ```text
//! KnowledgeBase  ◄── Source (knowledge_base)  ◄── Passage (source)
//! ```

/// A knowledge base (proposal, 2026-10-28). Kept in the table
/// `knowledge_bases`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct KnowledgeBase {
    /// The id of the base in the database.
    pub id: i64,

    /// The name of the base ("Common Criteria 2022").
    pub name: String,

    /// What the base is for.
    pub description: String,
}

/// A source of a knowledge base: a document whose text is cut into
/// passages. Kept in the table `knowledge_sources`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct Source {
    /// The id of the source in the database.
    pub id: i64,

    /// The base it belongs to (`KnowledgeBase`).
    pub knowledge_base: i64,

    /// What the source is.
    pub kind: SourceKind,

    /// The name of the source ("CC Part 3, R5").
    pub name: String,

    /// The standard it was made from (`OscalDocument`) when its kind is
    /// `Oscal`; `None` for every other kind.
    pub oscal_document: Option<i64>,
}

/// What a source is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub enum SourceKind {
    /// A PDF, its text read page by page.
    Pdf,
    /// A text file.
    Text,
    /// A standard of the program (an OSCAL catalog), one requirement
    /// after the other.
    Oscal,
    /// A note written by the user (an interpretation of the scheme).
    Note,
}

/// A passage of a source: a piece of its text of a bounded length, with
/// the place it comes from. Kept in the table `knowledge_passages`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct Passage {
    /// The id of the passage in the database.
    pub id: i64,

    /// The source it belongs to (`Source`).
    pub source: i64,

    /// The page of a PDF it stands on; `None` for a source without pages.
    pub page: Option<u32>,

    /// The label of the requirement it holds ("ADV_ARC.1-1") when the
    /// source is a standard; empty for every other kind.
    pub label: String,

    /// The text of the passage.
    pub text: String,
}
