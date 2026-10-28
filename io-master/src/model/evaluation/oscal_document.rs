//! A document of a standard: an OSCAL catalog (the CEM, a protection
//! profile, ...) kept in the table `oscal_documents`, never with the
//! documents of the product (`product_documents`; user, 2026-10-28: the
//! two are never mixed). The document itself is kept in the database, as
//! its OSCAL JSON, not in a file outside it.

/// A document of a standard (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct OscalDocument {
    /// The id of the document in the database.
    pub id: i64,

    /// The OSCAL document itself, as its JSON.
    pub doc: String,

    /// The version of the standard ("2022 R1").
    pub version: String,
}
