//! Help for the tests: a database of an evaluator in a temporary
//! directory, a small standard of three pages loaded into it, and a
//! small but real PDF for the tests that run `pdftotext`.

use crate::db::encryption::database_key::DatabaseKey;
use crate::db_old::standards as db_standards;
use crate::db_old::user::database::{self as user_db, UserDb};
use crate::model_old::standards::{DocumentFields, sha256_hex};
use crate::security::random::fill_random;
use crate::test_support::TempDir;

/// The pages of the small standard of the tests.
pub const PAGES: [&str; 3] = [
    "Part 3: Security assurance components\n1 Scope\nThis part defines the assurance requirements.",
    "ADV_FSP.4 Complete functional specification\nThe developer shall provide a functional specifi-\ncation.\n\
     ADV_FSP.4.1E The evaluator shall confirm that the information provided meets all requirements.",
    "ATE_IND.2 Independent testing - sample\nThe developer shall provide the TOE for testing.\n\
     ATE_IND.2.1E The evaluator shall test a subset of the TSF.",
];

/// A database of an evaluator; the directory lives as long as the
/// returned value.
pub struct TestDb {
    /// Held so that the folder lives as long as the database.
    #[allow(dead_code)]
    pub directory: TempDir,
    pub db: UserDb,
}

pub fn database() -> TestDb {
    let directory = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let db = user_db::create(directory.path(), 1, &key).unwrap();

    TestDb { directory, db }
}

/// The fields of the small standard.
pub fn fields() -> DocumentFields {
    DocumentFields { title: "Common Criteria".to_owned(), part: "Part 3".to_owned(), version: "CC:2022 R1".to_owned() }
}

/// Loads the small standard (with `content` as its file, so that
/// different calls make different files); returns its number.
pub fn load(db: &UserDb, content: &[u8]) -> i64 {
    let pages: Vec<String> = PAGES.iter().map(|page| (*page).to_owned()).collect();

    db_standards::create::document(db, &fields(), "cc-part3.pdf", &sha256_hex(content), content, &pages).unwrap()
}

/// A small but real PDF with one page per text (Helvetica; a line
/// break in a text starts a new line), for the tests that run
/// `pdftotext`.
pub fn pdf(pages: &[&str]) -> Vec<u8> {
    let font = 3 + pages.len() * 2;
    let kids: Vec<String> = (0..pages.len()).map(|index| format!("{} 0 R", 3 + index * 2)).collect();

    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), pages.len()),
    ];

    for (index, text) in pages.iter().enumerate() {
        let lines: Vec<String> = text.split('\n').map(|line| format!("({line}) Tj 0 -14 Td")).collect();
        let content = format!("BT /F1 12 Tf 72 720 Td {} ET", lines.join(" "));

        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 {font} 0 R >> >> \
             /Contents {} 0 R >>",
            4 + index * 2
        ));
        objects.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
    }

    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned());

    let mut file = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();

    for (number, object) in objects.iter().enumerate() {
        offsets.push(file.len());
        file.push_str(&format!("{} 0 obj\n{object}\nendobj\n", number + 1));
    }

    let xref = file.len();
    file.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));

    for offset in offsets {
        file.push_str(&format!("{offset:010} 00000 n \n"));
    }

    file.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R /Info << /Title (Test standard) >> >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    ));

    file.into_bytes()
}
