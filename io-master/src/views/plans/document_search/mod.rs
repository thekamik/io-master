//! The element "search" of the view Evaluation plan: the text of the
//! documents of standards searched (page: `ui/src/views/plans/DocumentSearch.svelte`).
//!
//! ```text
//! action   command               parameters                 data back
//! search   document_search_run   text, document or null     the hits: document and its title, page, the text around
//! ```
//!
//! Searched in the one form of `model/standards/quote.rs` (spaces, divided
//! words and case do not matter); at most `MAX_HITS` hits. Opening a hit
//! in the viewer is presentation.

pub mod commands;
pub mod search;
pub mod view;
