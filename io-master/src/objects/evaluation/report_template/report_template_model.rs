//! A template of a report (user, 2026-11-03): a text of any format the
//! evaluator writes (Markdown, HTML, LaTeX, plain text), with markers where
//! the fields of an evaluation go (`{{work_unit.findings}}`), kept in the
//! table `report_templates`. A report is a template filled with an
//! evaluation; it is no model of its own.

/// A template of a report (user, 2026-11-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportTemplate {
    /// The id of the template in the database.
    pub id: i64,

    /// The name the evaluator gives it ("ETR after the CEM").
    pub name: String,

    /// The extension a report written from it is saved with, without the
    /// dot ("md", "html", "tex").
    pub extension: String,

    /// The text of the template, with its markers and blocks.
    pub text: String,
}
