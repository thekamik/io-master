//! A document of the product: a document of the client (ST, IXIT, ...)
//! kept in the table `product_documents`, never with the standards
//! (`oscal_documents`; user, 2026-10-28: the two are never mixed). Its
//! place is reserved, with its id, when the plan is made; the file comes
//! with the copy of the plan the evaluator works on. The file itself is
//! kept in the database, encrypted with it, so that what was assessed
//! cannot change after the assessment.

/// A document of the product (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code, reason = "a new model (2026-10-28), not used by the program yet, only by db/ and its tests")]
pub struct ProductDocument {
    /// The id of the document in the database, given when its place is
    /// reserved in the plan.
    pub id: i64,

    /// What the document is ("ST", "IXIT"), given by the planner when
    /// the place is reserved.
    pub name: String,

    /// The file of the document, as it was delivered; `None` while the
    /// place is only reserved.
    pub content: Option<Vec<u8>>,

    /// The version of the document as the client gives it ("1.3");
    /// `None` while the place is only reserved.
    pub version: Option<String>,
}
