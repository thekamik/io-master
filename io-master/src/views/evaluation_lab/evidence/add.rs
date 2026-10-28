//! Evidence delivered for an input (step 52 of docs/history.md): a PDF (its
//! text read page by page with Poppler), a text file, or a text pasted by
//! the evaluator. The bytes are kept exactly as delivered; the text is what
//! the AI reads and where the quotes of the fragments are looked for.

use std::path::Path;

use crate::app::error::AppError;
use crate::db_old::evaluation as db_evaluation;
use crate::db_old::oscal::ids::new_uuid;
use crate::db_old::user::database::UserDb;
use crate::model_old::evaluation::{
    EVIDENCE_NAME_MAX, Evidence, EvidenceKind, MAX_PAGES, NOTE_MAX, PDF_MAX, TEXT_FILE_MAX,
};
use crate::model_old::task_scheme::BlockKind;
use crate::model_old::text::{Field, check_text};
use crate::model_old::time::now;
use crate::pdf::poppler;

/// A file read for evidence: its name, its kind, its bytes and its pages.
pub struct ReadFile {
    pub name: String,
    pub kind: EvidenceKind,
    pub content: Vec<u8>,
    pub pages: Vec<String>,
}

/// Reads the file at `path` as evidence: a PDF by its ending (at most
/// `PDF_MAX` bytes, `MAX_PAGES` pages read), any other file as UTF-8 text
/// (at most `TEXT_FILE_MAX` bytes).
///
/// Called by: `evidence_add_file` (`evaluation_lab.rs`), on a thread for blocking work.
/// Calls: `poppler::info`, `poppler::text_of_pages`.
pub fn read_file(path: &Path) -> Result<ReadFile, AppError> {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let is_pdf = path.extension().is_some_and(|ending| ending.eq_ignore_ascii_case("pdf"));
    let size =
        std::fs::metadata(path).map_err(|error| AppError::Failed(format!("the file was not read: {error}")))?.len();

    ensure!(
        size <= if is_pdf { PDF_MAX } else { TEXT_FILE_MAX },
        AppError::Invalid("the file is too large to be taken as evidence".to_owned())
    );

    let content = std::fs::read(path).map_err(|error| AppError::Failed(format!("the file was not read: {error}")))?;

    if is_pdf {
        let info = poppler::info(&content).map_err(|error| AppError::Invalid(error.to_string()))?;
        let pages = poppler::text_of_pages(&content, 1, info.pages.min(MAX_PAGES))
            .map_err(|error| AppError::Invalid(error.to_string()))?;

        return Ok(ReadFile { name, kind: EvidenceKind::Pdf, content, pages });
    }

    let text = String::from_utf8(content.clone())
        .map_err(|_| AppError::Invalid("the file is neither a PDF nor a text in UTF-8".to_owned()))?;

    Ok(ReadFile { name, kind: EvidenceKind::Text, content, pages: vec![text] })
}

/// Stores `file` as evidence of input `block` of evaluation `evaluation`;
/// refused when the block is no input. The task goes in progress.
///
/// Called by: `evidence_add_file` (`evaluation_lab.rs`).
/// Calls: [`store`].
pub fn add_file(db: &UserDb, evaluation: &str, block: &str, file: ReadFile) -> Result<String, AppError> {
    store(db, evaluation, block, file.kind, &file.name, file.pages, &file.content)
}

/// Stores the text `text` pasted by the evaluator, named `name`, as
/// evidence of input `block`; at most `NOTE_MAX` characters.
///
/// Called by: `evidence_add_note` (`evaluation_lab.rs`).
/// Calls: [`store`].
pub fn add_note(db: &UserDb, evaluation: &str, block: &str, name: &str, text: &str) -> Result<String, AppError> {
    ensure!(!text.trim().is_empty(), AppError::Invalid("the text is empty".to_owned()));
    check_text(Field::Text, text, NOTE_MAX, true).map_err(|error| AppError::Invalid(error.to_string()))?;

    store(db, evaluation, block, EvidenceKind::Note, name, vec![text.to_owned()], text.as_bytes())
}

/// Stores a piece of evidence of input `block`; gives the task it belongs to.
///
/// Called by: [`add_file`], [`add_note`].
/// Calls: `db_evaluation::read::block`, `create::evidence`, `update::task_started`.
fn store(
    db: &UserDb,
    evaluation: &str,
    block: &str,
    kind: EvidenceKind,
    name: &str,
    pages: Vec<String>,
    content: &[u8],
) -> Result<String, AppError> {
    let Some((task, found)) = db_evaluation::read::block(db, evaluation, block)? else {
        return Err(AppError::Invalid("the input does not exist".to_owned()));
    };

    ensure!(found.kind == BlockKind::Input, AppError::Invalid("evidence is delivered to an input".to_owned()));

    let name = if name.trim().is_empty() { "Evidence".to_owned() } else { name.trim().to_owned() };
    check_text(Field::Name, &name, EVIDENCE_NAME_MAX, false).map_err(|error| AppError::Invalid(error.to_string()))?;

    let evidence = Evidence { uuid: new_uuid()?, block: block.to_owned(), kind, name, pages, added: now() };

    db_evaluation::create::evidence(db, evaluation, &task, &evidence, content)?;
    db_evaluation::update::task_started(db, evaluation, &task)?;

    Ok(task)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `add.rs`.

    use super::*;

    use crate::test_support::TempDir;
    use crate::test_support::db::{lab_evaluation, lab_package};
    use crate::test_support::standards::{database, pdf};

    #[test]
    fn a_pdf_is_read_by_page_and_a_text_as_one_page() {
        let directory = TempDir::new();
        let document = directory.path().join("st.pdf");
        std::fs::write(&document, pdf(&["The TOE shall.", "Page two."])).unwrap();
        let notes = directory.path().join("notes.txt");
        std::fs::write(&notes, "Interview notes.").unwrap();

        let read = read_file(&document).unwrap();
        assert_eq!((read.kind, read.pages.len()), (EvidenceKind::Pdf, 2));
        assert!(read.pages[0].contains("The TOE shall."));

        let read = read_file(&notes).unwrap();
        assert_eq!(
            (read.kind, read.name.as_str(), read.pages),
            (EvidenceKind::Text, "notes.txt", vec!["Interview notes.".to_owned()])
        );

        let binary = directory.path().join("image.bin");
        std::fs::write(&binary, [0xFF, 0xFE, 0x00]).unwrap();
        assert!(read_file(&binary).is_err());
    }

    #[test]
    fn a_note_is_kept_for_an_input_only_and_starts_the_task() {
        let test = database();
        let package = lab_package("c");
        db_evaluation::create::evaluation(&test.db, &lab_evaluation(&package), &package).unwrap();

        assert_eq!(add_note(&test.db, "e", "i", " Interview ", "Anna owns the keys.").unwrap(), "t");
        assert_eq!(db_evaluation::read::evidence_of(&test.db, "e", "i").unwrap()[0].name, "Interview");
        assert_eq!(
            db_evaluation::read::task(&test.db, "e", "t").unwrap().unwrap().state,
            crate::model_old::evaluation::TaskState::Working
        );
        assert!(add_note(&test.db, "e", "o", "x", "text").is_err(), "an output takes no evidence");
        assert!(add_note(&test.db, "e", "i", "x", " ").is_err());
    }
}
