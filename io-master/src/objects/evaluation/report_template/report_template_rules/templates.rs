//! The life of a template of a report (user, 2026-11-03): a new one starts
//! from the default template of the program; it is changed in the editor,
//! imported from a file of text and written into one. A template whose
//! blocks are wrong is never kept; one with an unknown marker is kept with
//! a warning.
//!
//! ```text
//! a name ──► new_template ──► DEFAULT_TEMPLATE, "md" ──► create_report_template
//! the editor ──► change_template: a name, an extension, the blocks right ──► update_report_template ──► the warnings
//! a file ──► template_file (UTF-8, within its limit) ──► import_template ──► create_report_template
//! a template ──► write_template ──► a file
//! ```

use std::fs;
use std::path::Path;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::error::DomainError;
use crate::objects::evaluation::report_template::report_template_model::ReportTemplate;
use crate::objects::evaluation::report_template::report_template_rules::markers::template_warnings;
use crate::objects::evaluation::report_template::report_template_store::{
    create_report_template, read_report_template, update_report_template,
};
use crate::objects::text::{MAX_NAME, check_length, checked_text};

/// The longest text of a template, in characters.
pub const MAX_TEMPLATE: usize = 1_000_000;

/// The largest file of a template, in bytes (a character takes up to four).
const MAX_TEMPLATE_FILE: u64 = 4 * 1_000_000;

/// The longest extension of a template, in characters ("html").
const MAX_EXTENSION: usize = 10;

/// The text a new template starts from: a report in Markdown with every
/// part of an evaluation, for the evaluator to change.
const DEFAULT_TEMPLATE: &str = include_str!("../../../../../report_templates/default_report.md");

/// The extension of a report written in Markdown, of the default template.
const DEFAULT_EXTENSION: &str = "md";

/// The extension `extension` without its dot, in lower case ("md").
///
/// Input: `extension`, as typed or read from a file name (".MD", "html").
/// Output: the extension ("md"); `DomainError::Refused` when it is empty,
/// longer than [`MAX_EXTENSION`] or holds other than ASCII letters and digits.
///
/// Called by: [`change_template`], [`import_template`].
fn checked_extension(extension: &str) -> Result<String, DomainError> {
    let extension = extension.trim().trim_start_matches('.').to_ascii_lowercase();

    ensure!(
        !extension.is_empty() && extension.len() <= MAX_EXTENSION,
        DomainError::Refused(format!("the extension is missing or over {MAX_EXTENSION} characters"))
    );
    ensure!(
        extension.chars().all(|character| character.is_ascii_alphanumeric()),
        DomainError::Refused(format!("the extension {extension} holds other than letters and digits"))
    );

    Ok(extension)
}

/// A new template named `name`, with the text of the default template.
///
/// Input: `db`, the open database of the user; `name`, the name typed
/// ("ETR after the CEM").
/// Output: the id of the template (3); `DomainError::Refused` for a name
/// missing or too long, or for too many templates.
///
/// Called by: `panels/views/reports/templates.rs`: `report_template_create`.
/// Calls: `objects/text.rs`: `checked_text`; `objects/evaluation/report_template/report_template_store.rs`:
/// `create_report_template`.
pub fn new_template(db: &UserDatabase, name: &str) -> Result<i64, DomainError> {
    let template = ReportTemplate {
        id: 0,
        name: checked_text(name, "the name of the template", MAX_NAME)?,
        extension: DEFAULT_EXTENSION.to_owned(),
        text: DEFAULT_TEMPLATE.to_owned(),
    };

    Ok(create_report_template(db, &template)?)
}

/// Writes the template `template.id` anew, as the editor gives it.
///
/// Input: `db`, the open database of the user; `template`, its id that of a
/// stored template, its name, extension and text as typed.
/// Output: the warnings of its markers (`["an unknown marker {{work_unit.findngs}}: ..."]`);
/// `DomainError::Refused` for a name or an extension wrong, a text over
/// [`MAX_TEMPLATE`] or blocks not closed or nested; an error of the database.
///
/// Called by: `panels/views/reports/templates.rs`: `report_template_save`.
/// Calls: `objects/text.rs`: `checked_text`, `check_length`; [`checked_extension`];
/// `objects/evaluation/report_template/report_template_rules/markers.rs`: `template_warnings`;
/// `objects/evaluation/report_template/report_template_store.rs`: `update_report_template`.
pub fn change_template(db: &UserDatabase, template: &ReportTemplate) -> Result<Vec<String>, DomainError> {
    check_length(&template.text, "the template", MAX_TEMPLATE)?;

    let warnings = template_warnings(&template.text)?;
    let checked = ReportTemplate {
        id: template.id,
        name: checked_text(&template.name, "the name of the template", MAX_NAME)?,
        extension: checked_extension(&template.extension)?,
        text: template.text.clone(),
    };

    update_report_template(db, &checked)?;

    Ok(warnings)
}

/// The text of the file `path` of a template: UTF-8 within its limit.
///
/// Input: `path`, a file chosen in the dialog of the page.
/// Output: its text; `DomainError::Refused` for a file over its limit, not
/// in UTF-8, or one that cannot be read.
///
/// Called by: [`import_template`].
fn template_file(path: &Path) -> Result<String, DomainError> {
    let read = |error: std::io::Error| DomainError::Refused(format!("{}: {error}", path.display()));
    let size = fs::metadata(path).map_err(read)?.len();

    ensure!(size <= MAX_TEMPLATE_FILE, DomainError::Refused(format!("a template over {MAX_TEMPLATE_FILE} bytes")));

    let bytes = fs::read(path).map_err(read)?;

    String::from_utf8(bytes).map_err(|_| DomainError::Refused("the template is no text in UTF-8".to_owned()))
}

/// A new template of the file `path`: named by the file, with its
/// extension ("etr.html" → "etr", "html"; "txt" for a file without one).
///
/// Input: `db`, the open database of the user; `path`, a file chosen in the
/// dialog of the page.
/// Output: the id of the template (4); `DomainError::Refused` for a file
/// [`template_file`] refuses, a text over [`MAX_TEMPLATE`], blocks not closed
/// or nested, a name or an extension wrong, too many templates.
///
/// Called by: `panels/views/reports/templates.rs`: `report_template_import`.
/// Calls: [`template_file`], [`checked_extension`]; `objects/text.rs`: `checked_text`, `check_length`;
/// `objects/evaluation/report_template/report_template_rules/markers.rs`: `template_warnings`;
/// `objects/evaluation/report_template/report_template_store.rs`: `create_report_template`.
pub fn import_template(db: &UserDatabase, path: &Path) -> Result<i64, DomainError> {
    let text = template_file(path)?;

    check_length(&text, "the template", MAX_TEMPLATE)?;
    template_warnings(&text)?;

    let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or("txt");
    let template = ReportTemplate {
        id: 0,
        name: checked_text(name, "the name of the file", MAX_NAME)?,
        extension: checked_extension(extension)?,
        text,
    };

    Ok(create_report_template(db, &template)?)
}

/// Writes the text of the template `id` into the file `path`, replacing it.
///
/// Input: `db`, the open database of the user; `id`, the id of a template;
/// `path`, the file chosen in the dialog of the page.
/// Output: nothing; `DomainError::Refused` when the file cannot be written,
/// an error of the database.
///
/// Called by: `panels/views/reports/templates.rs`: `report_template_export`.
/// Calls: `objects/evaluation/report_template/report_template_store.rs`: `read_report_template`.
pub fn write_template(db: &UserDatabase, id: i64, path: &Path) -> Result<(), DomainError> {
    let template = read_report_template(db, id)?;

    fs::write(path, template.text).map_err(|error| DomainError::Refused(format!("{}: {error}", path.display())))
}

#[cfg(test)]
mod tests {
    //! Tests of the life of a template: a new one from the default, changed
    //! with its warnings, refused when its blocks are wrong; imported from a
    //! file and written into one.

    use super::*;

    use crate::test_support::TempDir;
    use crate::test_support::database::database;

    #[test]
    fn a_new_template_is_the_default_one_without_warnings() {
        let (_directory, db) = database();
        let id = new_template(&db, " ETR ").unwrap();
        let stored = read_report_template(&db, id).unwrap();

        assert_eq!((stored.name.as_str(), stored.extension.as_str()), ("ETR", "md"));
        assert_eq!(template_warnings(&stored.text).unwrap(), Vec::<String>::new());
        assert!(matches!(new_template(&db, " "), Err(DomainError::Refused(_))));
    }

    #[test]
    fn a_template_is_changed_with_its_warnings_and_refused_when_its_blocks_are_wrong() {
        let (_directory, db) = database();
        let id = new_template(&db, "ETR").unwrap();
        let changed = ReportTemplate {
            id,
            name: "Annex".to_owned(),
            extension: ".HTML".to_owned(),
            text: "<h1>{{evaluation_plan.prodct}}</h1>".to_owned(),
        };

        assert_eq!(change_template(&db, &changed).unwrap().len(), 1);
        assert_eq!(read_report_template(&db, id).unwrap().extension, "html");

        let unclosed = ReportTemplate { text: "{{#evaluation_plan.work_units}}".to_owned(), ..changed.clone() };

        assert!(matches!(change_template(&db, &unclosed), Err(DomainError::Refused(_))));
        assert!(matches!(
            change_template(&db, &ReportTemplate { extension: "t x".to_owned(), ..changed }),
            Err(DomainError::Refused(_))
        ));
        assert_eq!(read_report_template(&db, id).unwrap().text, "<h1>{{evaluation_plan.prodct}}</h1>");
    }

    #[test]
    fn a_template_is_imported_from_a_file_and_written_into_one() {
        let (_directory, db) = database();
        let folder = TempDir::new();
        let file = folder.path().join("annex.tex");

        fs::write(&file, "\\section{ {{evaluation_plan.product}} }").unwrap();

        let id = import_template(&db, &file).unwrap();
        let stored = read_report_template(&db, id).unwrap();

        assert_eq!((stored.name.as_str(), stored.extension.as_str()), ("annex", "tex"));

        let copy = folder.path().join("copy.tex");

        write_template(&db, id, &copy).unwrap();

        assert_eq!(fs::read_to_string(&copy).unwrap(), stored.text);

        fs::write(&file, [0xff, 0xfe]).unwrap();

        assert!(matches!(import_template(&db, &file), Err(DomainError::Refused(_))));
    }
}
