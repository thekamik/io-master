//! A document of the product (`ProductDocument`) given in an evaluation
//! (user, 2026-10-30): its file, as it was delivered, in the place the plan
//! reserved, with its version. It is frozen once given (its store): what the
//! work units are assessed against never changes.
//!
//! ```text
//! a file ──► document_file: .pdf, .txt or .md, within its limit
//!        ──► give_document: a place of the evaluation, the text readable (document_text) ──► update_product_document
//! a document ──► product_document_fields ──► the values of its markers in a report
//! ```
//!
//! Whether a file is a PDF is told by its first bytes (`%PDF-`), so that the
//! model needs no field of the kind.

use std::fs;
use std::path::Path;

use crate::infrastructure::pdf::poppler::{self, MAX_FILE};
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::evaluation::evaluation_in_progress;
use crate::objects::evaluation::product_document::product_document_model::ProductDocument;
use crate::objects::evaluation::product_document::product_document_store::{
    read_product_document, update_product_document,
};
use crate::objects::text::checked_text;

/// The largest text file of a document, in bytes.
const MAX_TEXT_FILE: u64 = 64 * 1024 * 1024;

/// The longest version of a document, in characters.
const MAX_VERSION: usize = 500;

/// The first bytes of every PDF.
const PDF_SIGNATURE: &[u8] = b"%PDF-";

/// The file `path` of a document of the product, as it is kept: a `.pdf`,
/// `.txt` or `.md` file within its limit.
///
/// Input: `path`, a file chosen in the dialog of the page.
/// Output: the bytes of the file; `DomainError::Refused` for another kind, a
/// file over its limit, or one that cannot be read.
///
/// Called by: `panels/views/evaluation/inputs.rs`: `evaluation_document_give`.
pub fn document_file(path: &Path) -> Result<Vec<u8>, DomainError> {
    let read = |error: std::io::Error| DomainError::Refused(format!("{}: {error}", path.display()));
    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or_default().to_lowercase();
    let size = fs::metadata(path).map_err(read)?.len();
    let limit = match extension.as_str() {
        "txt" | "md" => MAX_TEXT_FILE,
        "pdf" => MAX_FILE,
        _ => return Err(DomainError::Refused("only .pdf, .txt and .md files are documents".to_owned())),
    };

    ensure!(size <= limit, DomainError::Refused(format!("a .{extension} file over {limit} bytes")));

    fs::read(path).map_err(read)
}

/// The text of the file `content` of a document: a PDF through Poppler,
/// anything else as UTF-8.
///
/// Input: `content`, the bytes of the file from [`document_file`].
/// Output: its text ("1 Introduction ..."); `DomainError::Refused` for a
/// file that is neither a PDF nor UTF-8 or holds no text,
/// `DomainError::Pdf` for a PDF that cannot be read.
///
/// Called by: [`give_document`].
/// Calls: `infrastructure/pdf/poppler.rs`: `text`.
pub fn document_text(content: &[u8]) -> Result<String, DomainError> {
    let text = if content.starts_with(PDF_SIGNATURE) {
        poppler::text(content)?
    } else {
        String::from_utf8(content.to_vec())
            .map_err(|_| DomainError::Refused("the file is neither a PDF nor a text in UTF-8".to_owned()))?
    };

    ensure!(!text.trim().is_empty(), DomainError::Refused("the file holds no text".to_owned()));

    Ok(text)
}

/// Gives the document `document` of the evaluation `evaluation` its file
/// `content` and its version `version`.
///
/// Input: `evaluation`, the id of an evaluation in progress; `document`, the
/// id of one of its places of documents; `content`, the file from
/// [`document_file`]; `version`, the version typed ("1.2").
/// Output: nothing; `DomainError::Refused` unless the evaluation is in
/// progress, the place is one of its documents and the file holds a text;
/// its store refuses a document given already.
///
/// Called by: `panels/views/evaluation/inputs.rs`: `evaluation_document_give`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/evaluation.rs`: `evaluation_in_progress`;
/// `objects/text.rs`: `checked_text`; [`document_text`];
/// `objects/evaluation/product_document/product_document_store.rs`: `read_product_document`,
/// `update_product_document` (which indexes the text).
pub fn give_document(
    db: &UserDatabase,
    evaluation: i64,
    document: i64,
    content: Vec<u8>,
    version: &str,
) -> Result<(), DomainError> {
    let stored = evaluation_in_progress(db, evaluation)?;

    ensure!(
        stored.product_documents.contains(&document),
        DomainError::Refused("the document is not one of the evaluation".to_owned())
    );

    let version = checked_text(version, "the version of the document", MAX_VERSION)?;
    let text = document_text(&content)?;
    let place = read_product_document(db, document)?;
    let given = ProductDocument { content: Some(content), version: Some(version), ..place };

    Ok(update_product_document(db, &given, Some(&text))?)
}

/// The fields of a document of the product a report writes
/// (`{{product_document.name}}`), in the order of [`product_document_fields`]; its
/// file is no text of a report.
pub const PRODUCT_DOCUMENT_FIELDS: [&str; 3] = ["id", "name", "version"];

/// The fields of the document `document` as a report writes them, in the
/// order of [`PRODUCT_DOCUMENT_FIELDS`]; a version not given is empty.
///
/// Input: `document`, a document of an evaluation.
/// Output: its fields (`["4", "ST", "1.0"]`).
///
/// Called by: `objects/evaluation/report_template/report_template_rules/filling.rs`: `lists_of`.
pub fn product_document_fields(document: &ProductDocument) -> [String; 3] {
    [document.id.to_string(), document.name.clone(), document.version.clone().unwrap_or_default()]
}

#[cfg(test)]
mod tests {
    //! Tests of a document given in an evaluation: once, in a place of the
    //! evaluation, with a text.

    use super::*;

    use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
    use crate::test_support::database::database;
    use crate::test_support::evaluation::evaluation_needing_inputs;
    use crate::test_support::pdf::pdf;

    #[test]
    fn a_document_is_given_once_and_only_with_a_text() {
        let (_directory, db) = database();
        let id = evaluation_needing_inputs(&db);
        let document = read_evaluation_plan(&db, id).unwrap().product_documents[0];

        assert!(matches!(give_document(&db, id, document, vec![0xff, 0xfe], "1.0"), Err(DomainError::Refused(_))));
        assert!(matches!(give_document(&db, id, 999, b"text".to_vec(), "1.0"), Err(DomainError::Refused(_))));

        give_document(&db, id, document, pdf(&["The TOE is a router."]), " 1.3 ").unwrap();

        let given = read_product_document(&db, document).unwrap();

        assert_eq!(given.version.as_deref(), Some("1.3"));
        assert!(document_text(&given.content.unwrap()).unwrap().contains("The TOE is a router."));
        assert!(matches!(give_document(&db, id, document, b"other".to_vec(), "1.4"), Err(DomainError::Db(_))));
    }
}
