//! Reading the text of a PDF, page by page, with the tools of
//! Poppler (the package `poppler-utils`):
//!
//! ```text
//! pdfinfo fd://0                          the number of pages, the title
//! pdftotext -enc UTF-8 -f A -l B fd://0 -     the text of pages A to B,
//!                                         each page ended by a form feed
//! ```
//!
//! Poppler reads standards best of the free tools (tables, columns,
//! divided words). It is called as a separate program, never through
//! a shell, and the PDF is given on its standard input (`fd://0`):
//! no file name reaches a command line, and a PDF that comes from an
//! imported document is never written to the disk unencrypted.
//!
//! A PDF may have hundreds of pages, so [`text`] reads it in parts of
//! [`PART`] pages ([`text_of_pages`]), each part one call of `pdftotext`.

use std::fmt;
use std::io::{ErrorKind, Write};
use std::process::{Command, Output, Stdio};

/// How many pages are read at once.
pub const PART: u32 = 20;

/// The largest PDF that is read, in bytes.
pub const MAX_FILE: u64 = 256 * 1024 * 1024;

/// The most pages a document may have.
pub const MAX_PAGES: u32 = 10_000;

/// What `pdfinfo` tells about a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfInfo {
    pub pages: u32,
    /// The title in the metadata of the file, if it has one.
    pub title: Option<String>,
}

/// Why the text of a file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfError {
    /// `pdfinfo` or `pdftotext` is not installed.
    ToolMissing(&'static str),
    /// The file is larger than [`MAX_FILE`].
    TooLarge,
    /// The tool could not read the file; holds what it said.
    Unreadable(String),
}

impl fmt::Display for PdfError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfError::ToolMissing(tool) => {
                write!(
                    formatter,
                    "{tool} is not installed; install the package poppler-utils (Debian) or poppler (Arch)"
                )
            }
            PdfError::TooLarge => write!(formatter, "the file is larger than {} MiB", MAX_FILE / 1024 / 1024),
            PdfError::Unreadable(reason) => write!(formatter, "the file cannot be read as a PDF: {reason}"),
        }
    }
}

impl std::error::Error for PdfError {}

/// Runs a tool of Poppler with the PDF on its standard input.
///
/// Input:
/// - `tool`: the tool (`"pdfinfo"`, `"pdftotext"`).
/// - `arguments`: its arguments, the PDF named `fd://0`.
/// - `pdf`: the bytes of the PDF, within [`MAX_FILE`].
///
/// Output: what the tool wrote; `PdfError::ToolMissing` when it is not
/// installed, `PdfError::Unreadable` with what it said when it fails.
///
/// Called by: [`info`], [`text_of_pages`].
fn run(tool: &'static str, arguments: &[&str], pdf: &[u8]) -> Result<Output, PdfError> {
    let mut child = Command::new(tool)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                PdfError::ToolMissing(tool)
            } else {
                PdfError::Unreadable(error.to_string())
            }
        })?;

    // The input was asked for as a pipe above, so the child has it.
    let mut stdin = child.stdin.take().ok_or_else(|| PdfError::Unreadable(format!("{tool} has no input")))?;

    // Written from another thread while the output is read here, so
    // that neither side waits for the other with a full pipe.
    let (written, output) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(pdf));
        let output = child.wait_with_output();

        (writer.join(), output)
    });
    let output = output.map_err(|error| PdfError::Unreadable(error.to_string()))?;

    // A tool that stops reading early (a broken file) breaks the pipe:
    // that is no error here, its exit status tells. Any other failure of
    // the writing is.
    match written {
        Ok(Ok(())) => {}
        Ok(Err(error)) if error.kind() == ErrorKind::BrokenPipe => {}
        Ok(Err(error)) => return Err(PdfError::Unreadable(error.to_string())),
        Err(_) => return Err(PdfError::Unreadable(format!("the PDF could not be given to {tool}"))),
    }

    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        let said = said.lines().find(|line| !line.trim().is_empty()).unwrap_or("no explanation");

        return Err(PdfError::Unreadable(said.trim().to_owned()));
    }

    Ok(output)
}

/// The number of pages and the title of a PDF.
///
/// Input: `pdf`, the bytes of the PDF.
///
/// Output: its pages and title (`PdfInfo { pages: 471, title: Some("CEM") }`);
/// `PdfError::TooLarge` over [`MAX_FILE`], `PdfError::Unreadable` when the
/// tool names no pages or more than [`MAX_PAGES`].
///
/// Called by: [`text`].
/// Calls: [`run`].
pub fn info(pdf: &[u8]) -> Result<PdfInfo, PdfError> {
    ensure!(u64::try_from(pdf.len()).is_ok_and(|size| size <= MAX_FILE), PdfError::TooLarge);

    let output = run("pdfinfo", &["-enc", "UTF-8", "fd://0"], pdf)?;
    let text = String::from_utf8_lossy(&output.stdout);

    let field = |name: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(name))
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };

    let pages: u32 = field("Pages:")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| PdfError::Unreadable("pdfinfo names no number of pages".to_owned()))?;

    ensure!(pages > 0 && pages <= MAX_PAGES, PdfError::Unreadable(format!("the file has {pages} pages")));

    Ok(PdfInfo { pages, title: field("Title:") })
}

/// The text of the pages `first` to `last`: one text per page.
///
/// Input:
/// - `pdf`: the bytes of the PDF.
/// - `first`, `last`: the pages, counted from 1, `first` not after `last`,
///   `last` not after [`MAX_PAGES`].
///
/// Output: the text of each page, in order (`["page 1", "page 2"]`); an
/// empty text for a page without text; `PdfError::Unreadable` for pages out
/// of their range or a file the tool cannot read.
///
/// Called by: [`text`].
/// Calls: [`run`].
pub fn text_of_pages(pdf: &[u8], first: u32, last: u32) -> Result<Vec<String>, PdfError> {
    ensure!(
        1 <= first && first <= last && last <= MAX_PAGES,
        PdfError::Unreadable(format!("no pages {first} to {last}"))
    );

    let (first_text, last_text) = (first.to_string(), last.to_string());

    let output = run("pdftotext", &["-enc", "UTF-8", "-f", &first_text, "-l", &last_text, "fd://0", "-"], pdf)?;

    let text = String::from_utf8_lossy(&output.stdout);
    let count = usize::try_from(last - first + 1).map_err(|_| PdfError::Unreadable(format!("{last} pages")))?;
    let mut pages = Vec::with_capacity(count);

    // Every page ends with a form feed, so the last piece is empty; a
    // page Poppler gives no text for stays empty.
    for page in text.split('\u{000C}').take(count) {
        pages.push(page.trim_end().to_owned());
    }

    pages.resize(count, String::new());

    Ok(pages)
}

/// The whole text of the PDF `pdf`, read by parts of [`PART`] pages, the
/// pages parted by an empty line.
///
/// Called by: `objects/ai/rag/rag_rules.rs`: `source_text` (a source);
/// `objects/evaluation/product_document/product_document_rules.rs`: `document_text` (a
/// document of the product).
/// Calls: [`info`], [`text_of_pages`].
pub fn text(pdf: &[u8]) -> Result<String, PdfError> {
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

#[cfg(test)]
mod tests {
    //! Unit tests of `poppler.rs`, with small PDFs made by
    //! `test_support/pdf.rs` and the real tools of Poppler.

    use super::*;

    use crate::test_support::pdf::pdf;

    #[test]
    fn the_pages_and_the_title_are_read() {
        let file = pdf(&["First page", "Second page"]);

        assert_eq!(info(&file).unwrap(), PdfInfo { pages: 2, title: Some("Test standard".to_owned()) });
    }

    #[test]
    fn the_text_of_a_range_of_pages_is_read() {
        let texts: Vec<String> = (1..=PART + 5).map(|number| format!("Text of page {number}")).collect();
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let file = pdf(&refs);

        let first = text_of_pages(&file, 1, PART).unwrap();
        let rest = text_of_pages(&file, PART + 1, PART + 5).unwrap();

        assert_eq!(first.len(), PART as usize);
        assert_eq!(first[0], "Text of page 1");
        assert_eq!(
            rest,
            ["Text of page 21", "Text of page 22", "Text of page 23", "Text of page 24", "Text of page 25"]
        );
    }

    #[test]
    fn a_range_that_is_no_range_is_refused() {
        let file = pdf(&["Only page"]);

        assert!(matches!(text_of_pages(&file, 0, 1), Err(PdfError::Unreadable(_))));
        assert!(matches!(text_of_pages(&file, 2, 1), Err(PdfError::Unreadable(_))));
    }

    #[test]
    fn a_file_that_is_not_a_pdf_is_refused() {
        assert!(matches!(info(b"just text"), Err(PdfError::Unreadable(_))));
    }
}
