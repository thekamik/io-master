//! The sources given to a knowledge base (step 57.2 of docs/todo.md): a PDF
//! (its text read page by page with Poppler), a text file, a catalog of
//! OSCAL of the installation (its groups and requirements one after the
//! other), or a note written by the evaluator. Each is cut into passages
//! (`model/knowledge.rs`, `passages`) and stored with them; the file itself
//! is not kept, only its text.

use std::path::Path;

use crate::ai::tasks::candidates::requirement_text;
use crate::app::error::AppError;
use crate::db::knowledge as db_knowledge;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::model_old::knowledge::{
    MAX_PAGES, MAX_PASSAGES, NAME_MAX, NOTE_MAX, PDF_MAX, Passage, SourceKind, TEXT_FILE_MAX, passages,
};
use crate::model_old::oscal::catalog::Catalog;
use crate::model_old::oscal::catalog::tree::flat_parts;
use crate::model_old::text::{Field, check_text};
use crate::pdf::poppler;

/// A file read as a source: its name, its kind and its passages.
pub struct ReadSource {
    pub name: String,
    pub kind: SourceKind,
    pub passages: Vec<Passage>,
}

/// Reads the file at `path` as a source: a PDF by its ending (at most
/// `PDF_MAX` bytes, `MAX_PAGES` pages, cut into passages page by page), any
/// other file as UTF-8 text (at most `TEXT_FILE_MAX` bytes).
///
/// Called by: `knowledge_base_add_file` (`knowledge.rs`), on a thread for blocking work.
/// Calls: `poppler::info`, `poppler::text_of_pages`, `passages`.
pub fn read_file(path: &Path) -> Result<ReadSource, AppError> {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let is_pdf = path.extension().is_some_and(|ending| ending.eq_ignore_ascii_case("pdf"));
    let size =
        std::fs::metadata(path).map_err(|error| AppError::Failed(format!("the file was not read: {error}")))?.len();

    ensure!(
        size <= if is_pdf { PDF_MAX } else { TEXT_FILE_MAX },
        AppError::Invalid("the file is too large for a source of a knowledge base".to_owned())
    );

    let content = std::fs::read(path).map_err(|error| AppError::Failed(format!("the file was not read: {error}")))?;

    if !is_pdf {
        let text = String::from_utf8(content)
            .map_err(|_| AppError::Invalid("the file is neither a PDF nor a text in UTF-8".to_owned()))?;

        return Ok(ReadSource { name, kind: SourceKind::Text, passages: passages(None, "", &text) });
    }

    let info = poppler::info(&content).map_err(|error| AppError::Invalid(error.to_string()))?;
    let pages = poppler::text_of_pages(&content, 1, info.pages.min(MAX_PAGES))
        .map_err(|error| AppError::Invalid(error.to_string()))?;
    let mut found = Vec::new();

    for (number, text) in (1..).zip(pages) {
        found.extend(passages(Some(number), "", &text));
        ensure!(found.len() <= MAX_PASSAGES, AppError::Invalid("the file has too many passages".to_owned()));
    }

    Ok(ReadSource { name, kind: SourceKind::Pdf, passages: found })
}

/// Stores `source` read from a file in base `base`.
///
/// Called by: `knowledge_base_add_file` (`knowledge.rs`).
/// Calls: [`store`].
pub fn add_file(db: &UserDb, base: i64, source: &ReadSource) -> Result<(), AppError> {
    store(db, base, source.kind, &source.name, &source.passages)
}

/// Gives base `base` the catalog `uuid` of the installation: every group
/// (its title and parts) and every requirement (`requirement_text`) one
/// passage or more, labelled with the label of the requirement.
///
/// Called by: `knowledge_base_add_catalog` (`knowledge.rs`).
/// Calls: `db_oscal::read::catalog`, [`catalog_passages`], [`store`].
pub fn add_catalog(db: &UserDb, base: i64, uuid: &str) -> Result<(), AppError> {
    let catalog = db_oscal::read::catalog(db, uuid)?;
    let found = catalog_passages(&catalog)?;

    store(db, base, SourceKind::Catalog, &catalog.metadata.title, &found)
}

/// The passages of `catalog`, in the order of the document.
///
/// Called by: [`add_catalog`].
/// Calls: `locations`, `requirement_text`, `flat_parts`, `passages`.
fn catalog_passages(catalog: &Catalog) -> Result<Vec<Passage>, AppError> {
    let mut found = Vec::new();

    for location in catalog.locations()? {
        if let Some(control) = catalog.control_at(&location) {
            let label = control.label().unwrap_or(&control.id);
            let text = requirement_text(label, control).map_err(AppError::Invalid)?;
            found.extend(passages(None, label, &text));
        } else if let Some(group) = catalog.group_at(&location.groups) {
            let mut text = format!("{}\n\n", group.title);

            for (_, part) in flat_parts(&group.parts)? {
                text.push_str(&format!("{}\n\n", part.prose.as_deref().unwrap_or_default()));
            }

            found.extend(passages(None, &group.title, &text));
        }

        ensure!(found.len() <= MAX_PASSAGES, AppError::Invalid("the catalog has too many passages".to_owned()));
    }

    Ok(found)
}

/// Gives base `base` the note `text` of the evaluator, named `name`; at
/// most `NOTE_MAX` characters.
///
/// Called by: `knowledge_base_add_note` (`knowledge.rs`).
/// Calls: `passages`, [`store`].
pub fn add_note(db: &UserDb, base: i64, name: &str, text: &str) -> Result<(), AppError> {
    ensure!(!text.trim().is_empty(), AppError::Invalid("the note is empty".to_owned()));
    check_text(Field::Text, text, NOTE_MAX, true).map_err(|error| AppError::Invalid(error.to_string()))?;

    store(db, base, SourceKind::Note, name, &passages(None, "", text))
}

/// Stores a source named `name` (a default name when it is empty) with its
/// passages; refused when it has none.
///
/// Called by: [`add_file`], [`add_catalog`], [`add_note`].
/// Calls: `db_knowledge::create::source`.
fn store(db: &UserDb, base: i64, kind: SourceKind, name: &str, found: &[Passage]) -> Result<(), AppError> {
    ensure!(!found.is_empty(), AppError::Invalid("the source has no text".to_owned()));

    let name = if name.trim().is_empty() { "Source".to_owned() } else { name.trim().to_owned() };
    check_text(Field::Name, &name, NAME_MAX, false).map_err(|error| AppError::Invalid(error.to_string()))?;

    db_knowledge::create::source(db, base, kind, &name, found)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `add.rs`.

    use super::*;

    use crate::test_support::TempDir;
    use crate::test_support::db::basic_catalog;
    use crate::test_support::standards::{database, pdf};

    #[test]
    fn a_pdf_is_read_into_passages_with_their_page() {
        let directory = TempDir::new();
        let document = directory.path().join("cc.pdf");
        std::fs::write(&document, pdf(&["The TSF shall audit.", "Keys are generated."])).unwrap();

        let read = read_file(&document).unwrap();

        assert_eq!((read.kind, read.name.as_str(), read.passages.len()), (SourceKind::Pdf, "cc.pdf", 2));
        assert_eq!(read.passages[1].page, Some(2));
        assert!(read.passages[1].text.contains("Keys are generated."));
    }

    #[test]
    fn a_catalog_gives_its_groups_and_requirements() {
        let test = database();
        let catalog = basic_catalog();
        crate::db::oscal::create::document(&test.db, &catalog).unwrap();
        let base = db_knowledge::create::base(&test.db, "ISO", "").unwrap();

        add_catalog(&test.db, base, catalog.uuid()).unwrap();

        let sources = db_knowledge::read::sources(&test.db, base).unwrap();
        assert_eq!(sources[0].kind, SourceKind::Catalog);
        let hits = db_knowledge::read::search(&test.db, base, "\"roles\"", 10).unwrap();
        assert!(hits.iter().any(|hit| hit.passage.label == "1.1.1"));
    }

    #[test]
    fn an_empty_note_is_refused_and_a_note_kept() {
        let test = database();
        let base = db_knowledge::create::base(&test.db, "CC", "").unwrap();

        assert!(add_note(&test.db, base, "Empty", " \n ").is_err());
        add_note(&test.db, base, "", "The scheme reads AVA_VAN.2 strictly.").unwrap();

        assert_eq!(db_knowledge::read::sources(&test.db, base).unwrap()[0].name, "Source");
    }
}
