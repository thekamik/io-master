//! The table of contents of a standard as the list of schemes of a
//! test project (step 27 of `docs/todo.md`): the AI reads the pages of
//! the table of contents and proposes the chapters; the program finds
//! where each chapter starts and stores them as *proposed* schemes,
//! which the user corrects and accepts.
//!
//! The task `contents` of the AI (`ai/api/tasks.rs`), asked in the role
//! *text analysis*:
//!
//! ```text
//! question()   from the open project: the pages of its document, as one
//!              part, and the instructions of the task (prompt())
//!   ai/api/completions.rs   the model answers over the connection of the role
//! answer()     read() the answer: one JSON object with "chapters"
//!              locate() the first page of every chapter, found in the document
//!              apply() every chapter stored as a proposed scheme of the
//!              open project with its pages (written through by db/)
//! ```
//!
//! **Pages.** A table of contents gives the page numbers printed in
//! the standard, which often differ from the pages of the PDF (a cover
//! and a preface count apart). The program therefore looks for the
//! heading of each chapter ("9.1 Identification and authentication")
//! in the text of the pages after the table of contents; where a
//! heading is not found, the printed number is moved by the offset the
//! found headings show. A chapter ends on the page where the next one
//! starts (that page may hold the end of the one and the start of the
//! other); the last chapter runs to the end of the document. The user
//! can correct every range.

use serde_json::Value;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use crate::model::standards::Document;
use crate::model::test_projects::{PageSpan, ProposalState, SchemeFields};
use crate::standards;
use crate::standards::quote::normalize;
use crate::test_projects::error::StoreError;
use crate::test_projects::schemes;
use crate::test_projects::workspace::OpenProject;

use crate::ai::api::completions::{self, Turn};
use crate::model::test_projects::rules::SchemeError;

use super::parts::{PART_RANGE, Part, parts};

/// The instructions to the model when the user has not changed them
/// (the task `contents` of `ai/api/tasks.rs`; changed ones are kept in
/// `db/ai/instructions/`).
pub const DEFAULT_INSTRUCTIONS: &str = r#"You help to map a standard for an evaluation. The standard will be split into chapters; each chapter becomes a scheme of its own, in which its requirements, their inputs and outputs are drawn later.

Below is the table of contents of the standard. Propose the chapters that hold requirements (criteria, activities, tests) of the product or of the evaluation. Rules:
1. Choose the level of the table at which a chapter holds a coherent group of requirements, usually the second level (for example "9.1", not "9" and not "9.1.5"). Leave out the introduction, the scope, the references, the terms, the annexes without requirements and the bibliography.
2. Copy the number and the title of each chapter exactly as the table gives them.
3. Give the page number the table shows for each chapter, or null when it shows none.
4. Keep the order of the table.

Answer with one JSON object and nothing else, in this form:
{"chapters": [{"number": "9.1", "title": "...", "page": 41}]}"#;

/// A chapter as the model proposed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub number: String,
    pub title: String,
    /// The page printed in the table of contents, if it gives one.
    pub printed_page: Option<u32>,
}

/// What a table of contents gave.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppliedChapters {
    /// New proposed schemes.
    pub added: usize,
    /// Chapters whose number a scheme of the project has already.
    pub known: usize,
    /// Chapters whose first page was not found.
    pub without_pages: usize,
    /// What was dropped, and why.
    pub dropped: Vec<String>,
}

/// The message to the model for the table of contents in `part` of
/// `document`.
pub fn prompt(instructions: &str, document: &Document, part: &Part) -> String {
    format!(
        "{}\n\nThe table of contents of {}, pages {} to {}.\n\n{}",
        instructions.trim(),
        document.fields.title,
        part.first_page,
        part.last_page,
        part.text
    )
}

/// The JSON object in an answer: the answer itself, or the first
/// object in it (a model may put it in a code block, or add words).
fn json_object(answer: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(answer.trim())
        && value.is_object()
    {
        return Some(value);
    }

    let start = answer.find('{')?;
    let end = answer.rfind('}')?;

    serde_json::from_str::<Value>(answer.get(start..=end)?).ok().filter(Value::is_object)
}

/// Reads the answer of the model. A chapter without a title is
/// dropped; the rest is kept in the order of the answer.
pub fn read(answer: &str) -> Result<(Vec<Chapter>, Vec<String>), String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;
    let text = |item: &Value, name: &str| item.get(name).and_then(Value::as_str).unwrap_or_default().trim().to_owned();

    let mut chapters = Vec::new();
    let mut dropped = Vec::new();

    for (index, item) in value.get("chapters").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let title = text(item, "title");

        if title.is_empty() {
            dropped.push(format!("chapter {}: no title", index + 1));
            continue;
        }

        chapters.push(Chapter {
            number: text(item, "number"),
            title,
            printed_page: item.get("page").and_then(Value::as_u64).map(|page| page as u32).filter(|page| *page > 0),
        });
    }

    Ok((chapters, dropped))
}

/// The heading of a chapter as it is looked for in a page: the number
/// and the beginning of the title, in the normal form of `quote.rs`.
fn heading(chapter: &Chapter) -> String {
    let title: String = normalize(&chapter.title).chars().take(40).collect();

    normalize(&format!("{} {}", chapter.number, title))
}

/// The first page of every chapter in `document`, looked for after the
/// page `after` (the end of the table of contents); `None` where it is
/// not found (see the top of this file).
pub fn locate(db: &UserDb, document: &Document, after: u32, chapters: &[Chapter]) -> Result<Vec<Option<u32>>, DbError> {
    let mut pages = Vec::new();

    for number in after + 1..=document.page_count {
        if let Some(text) = standards::read::page(db, document.id, number)? {
            pages.push((number, normalize(&text)));
        }
    }

    // Each heading after the start of the chapter before it: a chapter
    // named again in a running head or a reference later is not taken.
    let mut found: Vec<Option<u32>> = Vec::with_capacity(chapters.len());
    let mut from = after + 1;

    for chapter in chapters {
        let wanted = heading(chapter);
        let page = pages
            .iter()
            .filter(|(number, _)| *number >= from)
            .find(|(_, text)| !chapter.number.is_empty() && text.contains(&wanted))
            .map(|(number, _)| *number);

        if let Some(page) = page {
            from = page;
        }

        found.push(page);
    }

    // The offset between printed and real pages, from the chapters
    // found; the most common one wins.
    let mut offsets: Vec<i64> = chapters
        .iter()
        .zip(&found)
        .filter_map(|(chapter, page)| Some(i64::from((*page)?) - i64::from(chapter.printed_page?)))
        .collect();
    offsets.sort_unstable();

    let offset = offsets.chunk_by(|a, b| a == b).max_by_key(|run| run.len()).and_then(|run| run.first().copied());

    Ok(chapters
        .iter()
        .zip(found)
        .map(|(chapter, page)| {
            page.or_else(|| {
                let guess = i64::from(chapter.printed_page?) + offset?;
                (guess > i64::from(after) && guess <= i64::from(document.page_count)).then_some(guess as u32)
            })
        })
        .collect())
}

/// The pages of every chapter: from its first page to the first page
/// of the next chapter with one (that page may hold both), the last
/// one to the end of the document.
pub fn spans(document: &Document, starts: &[Option<u32>]) -> Vec<Option<PageSpan>> {
    starts
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let first = (*start)?;
            let next = starts[index + 1..].iter().flatten().copied().find(|page| *page >= first);

            Some(PageSpan {
                document_id: document.id,
                first_page: first,
                last_page: next.unwrap_or(document.page_count).max(first),
            })
        })
        .collect()
}

/// Stores the chapters as proposed schemes of the open project, at the end of
/// its list, with the pages of `spans`; a chapter whose number a
/// scheme of the project has already is left out.
pub fn apply(
    db: &UserDb,
    open: &mut OpenProject,
    chapters: &[Chapter],
    spans: &[Option<PageSpan>],
) -> Result<AppliedChapters, StoreError> {
    let mut applied = AppliedChapters::default();
    let numbers: Vec<String> = open.data().schemes.iter().map(|scheme| normalize(&scheme.fields.number)).collect();

    for (chapter, span) in chapters.iter().zip(spans) {
        if !chapter.number.is_empty() && numbers.contains(&normalize(&chapter.number)) {
            applied.known += 1;
            continue;
        }

        let fields = SchemeFields {
            number: chapter.number.clone(),
            name: chapter.title.clone(),
            pages: *span,
            state: ProposalState::Proposed,
        };

        match schemes::create::scheme(db, open, &fields) {
            Ok(_) => {
                applied.added += 1;

                if span.is_none() {
                    applied.without_pages += 1;
                }
            }
            Err(StoreError::Scheme(error)) => {
                applied.dropped.push(format!("{} {}: {error}", chapter.number, chapter.title).trim().to_owned())
            }
            Err(error) => return Err(error),
        }
    }

    Ok(applied)
}

/// The question about the table of contents on the pages
/// `first_page` to `last_page` of `document`, a document of the open
/// project: the whole table of contents as one part.
#[derive(Debug, Clone)]
pub struct Question {
    pub document: Document,
    pub part: Part,
    pub turns: Vec<Turn>,
}

/// The question for the open project, with the instructions of the
/// task.
pub fn question(
    db: &UserDb,
    open: &OpenProject,
    document: i64,
    first_page: u32,
    last_page: u32,
    instructions: &str,
) -> Result<Question, StoreError> {
    if !open.data().project.fields.documents.contains(&document) {
        return Err(SchemeError::Invalid("document", "read only a document of the project".to_owned()).into());
    }

    let stored = standards::read::get(db, document)?.ok_or(StoreError::Scheme(SchemeError::NotFound("document")))?;
    let part = parts(db, &[(document, first_page, last_page)], PART_RANGE.1)?
        .into_iter()
        .next()
        .ok_or_else(|| SchemeError::Invalid("pages", "the pages of the table of contents hold no text".to_owned()))?;
    let turns = completions::single(prompt(instructions, &stored, &part));

    Ok(Question { document: stored, part, turns })
}

/// What the answer about a table of contents came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answered {
    /// The chapters were applied as proposed schemes.
    Applied(AppliedChapters),
    /// The answer could not be read; the reason, for the user.
    Unreadable(String),
}

/// Reads the answer of the model to `question`, finds the pages of the
/// chapters and applies them to the open project.
pub fn answer(db: &UserDb, open: &mut OpenProject, question: &Question, text: &str) -> Result<Answered, StoreError> {
    let (chapters, dropped) = match read(text) {
        Ok(read) => read,
        Err(reason) => return Ok(Answered::Unreadable(reason)),
    };

    let starts = locate(db, &question.document, question.part.last_page, &chapters)?;
    let spans = spans(&question.document, &starts);
    let mut applied = apply(db, open, &chapters, &spans)?;
    applied.dropped.splice(0..0, dropped);

    Ok(Answered::Applied(applied))
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "contents_tests.rs"]
mod tests;
