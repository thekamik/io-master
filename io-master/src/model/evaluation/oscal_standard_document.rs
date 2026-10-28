//! An OSCAL document of a standard: the catalog of a standard (the CEM,
//! a protection profile, ...) kept in the table `oscal_documents`, never with the
//! documents of the product (`product_documents`; user, 2026-10-28: the
//! two are never mixed). The document itself is kept in the database, as
//! its OSCAL JSON, not in a file outside it.

/// An OSCAL document of a standard (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code, reason = "a new model (2026-10-28), not used by the program yet, only by db/ and its tests")]
pub struct OscalStandardDocument {
    /// The id of the document in the database.
    pub id: i64,

    /// The title of the standard ("CEM:2022 Revision 1"), as its
    /// document gives it, for the lists.
    pub title: String,

    /// The OSCAL document itself, as its JSON.
    pub doc: String,

    /// The version of the standard ("2022 R1").
    pub version: String,
}
