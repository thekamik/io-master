//! The rules of the knowledge (`KnowledgeBase`, `Source`): the bases and
//! their sources made and changed, a base searched. A base is shared: every agent that names it searches
//! it. A source is the text of one file (`.txt`, `.md` or `.pdf`); it is
//! never removed (no delete, user 2026-10-28), but its text may be replaced
//! by a newer file and its name changed.
//!
//! ```text
//! a file ──► source_text: .txt, .md as UTF-8; .pdf through infrastructure/pdf/poppler.rs, by parts of pages
//!        ──► add_source / replace_source_text ──► rag_store.rs cuts the text into passages and indexes them
//! ```

use std::fs;
use std::path::Path;

use crate::infrastructure::pdf::poppler::{self, MAX_FILE};
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::rag::rag_model::{KnowledgeBase, Source};
use crate::objects::ai::rag::rag_store::{
    FoundPassage, MAX_BASES, MAX_FOUND, MAX_PASSAGE_CHARS, MAX_SOURCE_CHARS, MAX_SOURCES, MAX_STOP_WORD,
    MAX_STOP_WORDS, MIN_PASSAGE_CHARS, PASSAGE_CHARS, STOP_WORDS, create_knowledge_base, create_source,
    read_knowledge_base, read_knowledge_bases, read_passages, read_source, read_source_records, update_knowledge_base,
    update_source, update_source_name,
};
use crate::objects::error::DomainError;
use crate::objects::text::checked_text;

/// The longest name of a base or a source, in characters.
const MAX_NAME: usize = 200;

/// The longest description of a base, in characters.
const MAX_DESCRIPTION: usize = 2000;

/// The largest text file read as a source, in bytes: four bytes at most a
/// character of the longest text of a source.
const MAX_TEXT_FILE: u64 = 64 * 1024 * 1024;

/// Makes a new knowledge base, its passages and stop words those of the
/// program (`PASSAGE_CHARS`, `STOP_WORDS`), to be tuned in the base.
///
/// Input: `name`, the name typed ("Common Criteria"); `description`, what it
/// holds, may be empty.
/// Output: the id of the new base (`4`); `DomainError::Refused` for a name
/// refused, a description over [`MAX_DESCRIPTION`] or `MAX_BASES` bases
/// already, an error of the database.
///
/// Called by: `panels/views/agents/knowledge.rs`: `knowledge_base_create`.
/// Calls: `objects/text.rs`: `checked_text`; `objects/ai/rag/rag_store.rs`: `read_knowledge_bases`,
/// `create_knowledge_base`.
pub fn add_base(db: &UserDatabase, name: &str, description: &str) -> Result<i64, DomainError> {
    let name = checked_text(name, "the name of the base", MAX_NAME)?;

    ensure!(read_knowledge_bases(db)?.len() < MAX_BASES, DomainError::Refused(format!("{MAX_BASES} bases already")));
    ensure!(
        description.chars().count() <= MAX_DESCRIPTION,
        DomainError::Refused(format!("a description over {MAX_DESCRIPTION} characters"))
    );

    let mut stop_words = Vec::with_capacity(STOP_WORDS.len());

    for word in STOP_WORDS {
        stop_words.push(word.to_owned());
    }

    let base = KnowledgeBase {
        id: 0,
        name,
        description: description.trim().to_owned(),
        passage_characters: u32::try_from(PASSAGE_CHARS).map_err(|error| DomainError::Refused(error.to_string()))?,
        stop_words,
    };

    Ok(create_knowledge_base(db, &base)?)
}

/// The stop words as typed, each trimmed and in lower case, an empty one or
/// one given twice left out.
///
/// Input: `words`, the stop words of a base as the page sends them.
/// Output: the words to keep (`["the", "shall"]`); `DomainError::Refused` for
/// more than `MAX_STOP_WORDS` or one over `MAX_STOP_WORD` characters.
///
/// Called by: [`change_base`].
fn checked_stop_words(words: &[String]) -> Result<Vec<String>, DomainError> {
    ensure!(words.len() <= MAX_STOP_WORDS, DomainError::Refused(format!("more than {MAX_STOP_WORDS} stop words")));

    let mut kept: Vec<String> = Vec::with_capacity(words.len());

    for word in words {
        let word = word.trim().to_lowercase();

        ensure!(
            word.chars().count() <= MAX_STOP_WORD,
            DomainError::Refused(format!("a stop word over {MAX_STOP_WORD} characters"))
        );

        if !word.is_empty() && !kept.contains(&word) {
            kept.push(word);
        }
    }

    Ok(kept)
}

/// Writes the base `base.id` anew: its name, description, length of a
/// passage (its sources indexed anew when it changes) and stop words.
///
/// Input: `base`, the base as changed, its id that of a stored base.
/// Output: nothing; `DomainError::Refused` for a name or a description
/// refused, a passage out of `MIN_PASSAGE_CHARS` to `MAX_PASSAGE_CHARS`, stop
/// words refused, an error of the database for a base missing.
///
/// Called by: `panels/views/agents/knowledge.rs`: `knowledge_base_save`.
/// Calls: `objects/text.rs`: `checked_text`; `objects/ai/rag/rag_store.rs`: `read_knowledge_base`,
/// `update_knowledge_base`; [`checked_stop_words`].
pub fn change_base(db: &UserDatabase, base: &KnowledgeBase) -> Result<(), DomainError> {
    let name = checked_text(&base.name, "the name of the base", MAX_NAME)?;

    read_knowledge_base(db, base.id)?;
    ensure!(
        base.description.chars().count() <= MAX_DESCRIPTION,
        DomainError::Refused(format!("a description over {MAX_DESCRIPTION} characters"))
    );

    ensure!(
        (MIN_PASSAGE_CHARS..=MAX_PASSAGE_CHARS).contains(&base.passage_characters),
        DomainError::Refused(format!("a passage: from {MIN_PASSAGE_CHARS} to {MAX_PASSAGE_CHARS} characters"))
    );

    let changed = KnowledgeBase {
        id: base.id,
        name,
        description: base.description.trim().to_owned(),
        passage_characters: base.passage_characters,
        stop_words: checked_stop_words(&base.stop_words)?,
    };

    Ok(update_knowledge_base(db, &changed)?)
}

/// The text of the file `path` for a source: a `.txt` or `.md` file as
/// UTF-8, a `.pdf` through Poppler.
///
/// Input: `path`, a file chosen in the dialog of the page.
/// Output: its text ("The TOE shall ..."); `DomainError::Refused` for
/// another kind of file, a file over its limit, a text that is no UTF-8,
/// holds nothing or is over `MAX_SOURCE_CHARS`; `DomainError::Pdf` for a PDF
/// that cannot be read.
///
/// Called by: `panels/views/agents/knowledge.rs`: `chosen_text`.
/// Calls: `infrastructure/pdf/poppler.rs`: `text`.
pub fn source_text(path: &Path) -> Result<String, DomainError> {
    let read = |error: std::io::Error| DomainError::Refused(format!("{}: {error}", path.display()));
    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or_default().to_lowercase();
    let size = fs::metadata(path).map_err(read)?.len();
    let text = match extension.as_str() {
        "txt" | "md" => {
            ensure!(size <= MAX_TEXT_FILE, DomainError::Refused(format!("a text file over {MAX_TEXT_FILE} bytes")));
            String::from_utf8(fs::read(path).map_err(read)?)
                .map_err(|_| DomainError::Refused("the file is no text in UTF-8".to_owned()))?
        }
        "pdf" => {
            ensure!(size <= MAX_FILE, DomainError::Refused(format!("a PDF over {MAX_FILE} bytes")));
            poppler::text(&fs::read(path).map_err(read)?)?
        }
        _ => return Err(DomainError::Refused("only .txt, .md and .pdf files are sources".to_owned())),
    };

    ensure!(!text.trim().is_empty(), DomainError::Refused("the file holds no text".to_owned()));
    ensure!(
        text.chars().count() <= MAX_SOURCE_CHARS,
        DomainError::Refused(format!("a text over {MAX_SOURCE_CHARS} characters"))
    );

    Ok(text)
}

/// Adds the text `text` to the base `base` as a new source named `name`.
/// The database indexes its passages.
///
/// Input: `base`, the id of a stored base; `name`, the name of the source
/// ("CEM 3.1 R5.pdf"); `text`, its text from [`source_text`].
/// Output: the id of the new source (`12`); `DomainError::Refused` for a name
/// refused or `MAX_SOURCES` sources in the base already, an error of the
/// database.
///
/// Called by: `panels/views/agents/knowledge.rs`: `knowledge_source_add`.
/// Calls: `objects/text.rs`: `checked_text`; `objects/ai/rag/rag_store.rs`: `read_knowledge_base`,
/// `read_source_records`, `create_source`.
pub fn add_source(db: &UserDatabase, base: i64, name: &str, text: String) -> Result<i64, DomainError> {
    let name = checked_text(name, "the name of the source", MAX_NAME)?;

    read_knowledge_base(db, base)?;
    ensure!(
        read_source_records(db, base)?.len() < MAX_SOURCES,
        DomainError::Refused(format!("{MAX_SOURCES} sources in the base already"))
    );

    Ok(create_source(db, &Source { id: 0, knowledge_base: base, name, text })?)
}

/// Replaces the text of the source `id` (a newer version of its file); the
/// database indexes it anew.
///
/// Called by: `panels/views/agents/knowledge.rs`: `knowledge_source_replace`.
/// Calls: `objects/ai/rag/rag_store.rs`: `read_source`, `update_source`.
pub fn replace_source_text(db: &UserDatabase, id: i64, text: String) -> Result<(), DomainError> {
    let source = read_source(db, id)?;

    Ok(update_source(db, &Source { text, ..source })?)
}

/// Changes the name of the source `id`.
///
/// Input: `id`, the id of a stored source; `name`, the new name typed.
/// Output: nothing; `DomainError::Refused` for a name refused, an error of
/// the database for a source missing.
///
/// Called by: `panels/views/agents/knowledge.rs`: `knowledge_source_rename`.
/// Calls: `objects/text.rs`: `checked_text`; `objects/ai/rag/rag_store.rs`: `update_source_name`.
pub fn rename_source(db: &UserDatabase, id: i64, name: &str) -> Result<(), DomainError> {
    let name = checked_text(name, "the name of the source", MAX_NAME)?;

    Ok(update_source_name(db, id, &name)?)
}

/// The passages of the base `base` that fit `question` best, each with the
/// name of its source and the passage that follows it: the test of a base,
/// at most [`MAX_FOUND`].
///
/// Input: `base`, the id of a stored base; `question`, the question typed.
/// Output: the passages (`[(7, "CEM.pdf", "The evaluator shall ...",
/// None)]`); an error of the database for a base missing.
///
/// Called by: `panels/views/agents/knowledge.rs`: `knowledge_search`.
/// Calls: `objects/ai/rag/rag_store.rs`: `read_knowledge_base`, `read_passages`.
pub fn search_base(db: &UserDatabase, base: i64, question: &str) -> Result<Vec<FoundPassage>, DomainError> {
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
        assert!(matches!(source_text(&image), Err(DomainError::Refused(_))));
        assert!(matches!(source_text(&empty), Err(DomainError::Refused(_))));
    }
}
