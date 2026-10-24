//! The data of a fragment written by the evaluator, as the page sends it.

use serde::Deserialize;

/// A fragment of a piece of evidence: an exact quote, its page if known, a
/// note.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FragmentForm {
    pub evidence: String,
    pub page: Option<u32>,
    pub quote: String,
    pub note: String,
}
