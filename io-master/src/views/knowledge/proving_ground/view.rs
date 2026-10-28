//! The data of the proving ground of an agent, as the page gets it: the
//! last question, the passages it was given, the answer and its quotes.

use serde::Serialize;

use crate::model_old::knowledge::Trial;

use super::super::knowledge_search::view::{self as search_view, HitView};

/// A quote of the answer: the passage it names and whether it was found there.
#[derive(Debug, Serialize)]
pub struct QuoteView {
    pub passage: usize,
    pub quote: String,
    pub found: bool,
}

/// The last trial of an agent.
#[derive(Debug, Serialize)]
pub struct TrialView {
    pub question: String,
    pub passages: Vec<HitView>,
    pub answer: String,
    pub quotes: Vec<QuoteView>,
}

/// `trial` as the page shows it.
///
/// Called by: `proving_ground_get` (`knowledge.rs`).
pub fn trial(trial: Trial) -> TrialView {
    let mut quotes = Vec::with_capacity(trial.quotes.len());

    for quote in trial.quotes {
        quotes.push(QuoteView { passage: quote.passage, quote: quote.quote, found: quote.found });
    }

    TrialView { question: trial.question, passages: search_view::all(trial.passages), answer: trial.answer, quotes }
}
