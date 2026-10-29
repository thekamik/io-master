//! The knowledge bases and their sources, as the panel of the agents makes
//! and changes them. A base is shared: every agent that names it searches
//! it. A source is the text of one file (`.txt`, `.md` or `.pdf`); it is
//! never removed (no delete, user 2026-10-28), but its text may be replaced
//! by a newer file and its name changed.
//!
//! ```text
//! a file ──► source_text: .txt, .md as UTF-8; .pdf through pdf/poppler.rs, by parts of pages
//!        ──► add_source / replace_source_text ──► db/ cuts the text into passages and indexes them
//! ```

use std::fs;
use std::path::Path;

use crate::db::ai::rag::{
    MAX_BASES, MAX_FOUND, MAX_SOURCE_CHARS, MAX_SOURCES, create_knowledge_base, create_source, read_knowledge_base,
    read_knowledge_bases, read_passages, read_source, read_sources, update_knowledge_base, update_source,
};
use crate::db::database::Database;
use crate::model::ai::rag::{KnowledgeBase, Source};
use crate::panels::logic::agents::error::AgentError;
use crate::pdf::poppler::{MAX_FILE, PART, info, text_of_pages};

/// The longest name of a base or a source, in characters.
const MAX_NAME: usize = 200;

/// The longest description of a base, in characters.
const MAX_DESCRIPTION: usize = 2000;

/// The largest text file read as a source, in bytes: four bytes at most a
/// character of the longest text of a source.
const MAX_TEXT_FILE: u64 = 64 * 1024 * 1024;

/// A name trimmed, refused when it is empty or over [`MAX_NAME`].
///
/// Called by: [`add_base`], [`change_base`], [`add_source`],
/// [`rename_source`].
fn checked_name(name: &str, what: &str) -> Result<String, AgentError> {
    let name = name.trim();

    ensure!(!name.is_empty(), AgentError::Refused(format!("{what} without a name")));
    ensure!(name.chars().count() <= MAX_NAME, AgentError::Refused(format!("a name over {MAX_NAME} characters")));

    Ok(name.to_owned())
}

/// Makes a new knowledge base; gives its id.
///
/// Called by: `panels/views/agents/knowledge.rs` (a new base).
/// Calls: [`checked_name`], `read_knowledge_bases`, `create_knowledge_base`.
pub fn add_base(db: &Database, name: &str, description: &str) -> Result<i64, AgentError> {
    let name = checked_name(name, "a knowledge base")?;

    ensure!(read_knowledge_bases(db)?.len() < MAX_BASES, AgentError::Refused(format!("{MAX_BASES} bases already")));
    ensure!(
        description.chars().count() <= MAX_DESCRIPTION,
        AgentError::Refused(format!("a description over {MAX_DESCRIPTION} characters"))
    );

    Ok(create_knowledge_base(db, &KnowledgeBase { id: 0, name, description: description.trim().to_owned() })?)
}

/// Writes the name and the description of the base `base.id` anew.
///
/// Called by: `panels/views/agents/knowledge.rs` (a base saved).
/// Calls: [`checked_name`], `read_knowledge_base`, `update_knowledge_base`.
pub fn change_base(db: &Database, base: &KnowledgeBase) -> Result<(), AgentError> {
    let name = checked_name(&base.name, "a knowledge base")?;

    read_knowledge_base(db, base.id)?;
    ensure!(
        base.description.chars().count() <= MAX_DESCRIPTION,
        AgentError::Refused(format!("a description over {MAX_DESCRIPTION} characters"))
    );

    Ok(update_knowledge_base(
        db,
        &KnowledgeBase { id: base.id, name, description: base.description.trim().to_owned() },
    )?)
}

/// The text of the PDF `pdf`, read by parts of [`PART`] pages, the pages
/// parted by an empty line.
///
/// Called by: [`source_text`].
/// Calls: `poppler::info`, `poppler::text_of_pages`.
fn pdf_text(pdf: &[u8]) -> Result<String, AgentError> {
    let pages = info(pdf)?.pages;
    let mut text = String::new();
    let mut first = 1;

    // Each turn reads one part; MAX_PAGES / PART parts at most.
    for _ in 0..=pages / PART {
        if first > pages {
            break;
        }

        let last = (first + PART - 1).min(pages);

        for page in text_of_pages(pdf, first, last)? {
            text.push_str(&page);
            text.push_str("\n\n");
        }

        first = last + 1;
    }

    Ok(text)
}

/// The text of the file `path` for a source: a `.txt` or `.md` file as
/// UTF-8, a `.pdf` through Poppler; refused for another kind, a file over
/// its limit, a text that is no UTF-8 or that holds nothing.
///
/// Called by: `panels/views/agents/knowledge.rs` (a file chosen in the
/// dialog).
/// Calls: [`pdf_text`].
pub fn source_text(path: &Path) -> Result<String, AgentError> {
    let read = |error: std::io::Error| AgentError::Refused(format!("{}: {error}", path.display()));
    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or_default().to_lowercase();
    let size = fs::metadata(path).map_err(read)?.len();
    let text = match extension.as_str() {
        "txt" | "md" => {
            ensure!(size <= MAX_TEXT_FILE, AgentError::Refused(format!("a text file over {MAX_TEXT_FILE} bytes")));
            String::from_utf8(fs::read(path).map_err(read)?)
                .map_err(|_| AgentError::Refused("the file is no text in UTF-8".to_owned()))?
        }
        "pdf" => {
            ensure!(size <= MAX_FILE, AgentError::Refused(format!("a PDF over {MAX_FILE} bytes")));
            pdf_text(&fs::read(path).map_err(read)?)?
        }
        _ => return Err(AgentError::Refused("only .txt, .md and .pdf files are sources".to_owned())),
    };

    ensure!(!text.trim().is_empty(), AgentError::Refused("the file holds no text".to_owned()));
    ensure!(
        text.chars().count() <= MAX_SOURCE_CHARS,
        AgentError::Refused(format!("a text over {MAX_SOURCE_CHARS} characters"))
    );

    Ok(text)
}

/// Adds the text `text` to the base `base` as a new source named `name`;
/// gives its id. The database indexes its passages.
///
/// Called by: `panels/views/agents/knowledge.rs` (a file added).
/// Calls: [`checked_name`], `read_sources`, `create_source`.
pub fn add_source(db: &Database, base: i64, name: &str, text: String) -> Result<i64, AgentError> {
    let name = checked_name(name, "a source")?;

    read_knowledge_base(db, base)?;
    ensure!(
        read_sources(db, base)?.len() < MAX_SOURCES,
        AgentError::Refused(format!("{MAX_SOURCES} sources in the base already"))
    );

    Ok(create_source(db, &Source { id: 0, knowledge_base: base, name, text })?)
}

/// Replaces the text of the source `id` (a newer version of its file); the
/// database indexes it anew.
///
/// Called by: `panels/views/agents/knowledge.rs` (a file chosen anew).
/// Calls: `read_source`, `update_source`.
pub fn replace_source_text(db: &Database, id: i64, text: String) -> Result<(), AgentError> {
    let source = read_source(db, id)?;

    Ok(update_source(db, &Source { text, ..source })?)
}

/// Changes the name of the source `id`.
///
/// Called by: `panels/views/agents/knowledge.rs` (a source renamed).
/// Calls: [`checked_name`], `read_source`, `update_source`.
pub fn rename_source(db: &Database, id: i64, name: &str) -> Result<(), AgentError> {
    let name = checked_name(name, "a source")?;
    let source = read_source(db, id)?;

    Ok(update_source(db, &Source { name, ..source })?)
}

/// The passages of the base `base` that fit `question` best: the test of a
/// base, at most [`MAX_FOUND`].
///
/// Called by: `panels/views/agents/knowledge.rs` (the search in a base).
/// Calls: `read_knowledge_base`, `read_passages`.
pub fn search_base(db: &Database, base: i64, question: &str) -> Result<Vec<String>, AgentError> {
    read_knowledge_base(db, base)?;

    Ok(read_passages(db, &[base], question, MAX_FOUND)?)
}

#[cfg(test)]
mod tests {
    //! Tests of the knowledge: bases, sources from files of each kind, the
    //! text replaced, the search.

    use super::*;

    use crate::test_support::TempDir;
    use crate::test_support::database::database;
    use crate::test_support::pdf::pdf;

    #[test]
    fn a_base_gets_sources_from_files_and_finds_their_passages() {
        let (_directory, db) = database();
        let folder = TempDir::new();
        let base = add_base(&db, " Earlier evaluations ", "Reports").unwrap();
        let notes = folder.path().join("notes.md");

        fs::write(&notes, "# Router X\n\nThe secure boot was verified.").unwrap();

        let id = add_source(&db, base, "notes.md", source_text(&notes).unwrap()).unwrap();

        assert_eq!(read_knowledge_base(&db, base).unwrap().name, "Earlier evaluations");
        assert_eq!(search_base(&db, base, "secure boot").unwrap().len(), 1);

        replace_source_text(&db, id, "Only the update mechanism.".to_owned()).unwrap();
        rename_source(&db, id, "notes, version 2").unwrap();

        assert!(search_base(&db, base, "secure boot").unwrap().is_empty());
        assert_eq!(read_source(&db, id).unwrap().name, "notes, version 2");
    }

    #[test]
    fn a_pdf_is_read_and_another_kind_or_an_empty_file_refused() {
        let folder = TempDir::new();
        let report = folder.path().join("report.pdf");
        let image = folder.path().join("image.png");
        let empty = folder.path().join("empty.txt");

        fs::write(&report, pdf(&["Annex A", "Annex B"])).unwrap();
        fs::write(&image, [0u8; 4]).unwrap();
        fs::write(&empty, "  \n").unwrap();

        let text = source_text(&report).unwrap();

        assert!(text.contains("Annex A") && text.contains("Annex B"));
        assert!(matches!(source_text(&image), Err(AgentError::Refused(_))));
        assert!(matches!(source_text(&empty), Err(AgentError::Refused(_))));
    }
}
