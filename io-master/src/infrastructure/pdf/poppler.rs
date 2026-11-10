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
//! A PDF comes from outside the laboratory, and the tools are large parsers:
//! each call is bounded in time ([`TIMEOUT`]) and in what it may write
//! ([`MAX_OUTPUT`]), so that a file made for it neither holds the program
//! nor fills its memory ([`run`]).
//!
//! A PDF may have hundreds of pages, so [`text`] reads it in parts of
//! [`PART`] pages ([`text_of_pages`]), each part one call of `pdftotext`.

use std::fmt;
use std::io::{ErrorKind, Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

/// How many pages are read at once.
pub const PART: u32 = 20;

/// The largest PDF that is read, in bytes.
pub const MAX_FILE: u64 = 256 * 1024 * 1024;

/// The most pages a document may have.
pub const MAX_PAGES: u32 = 10_000;

/// The most parts of [`PART`] pages a PDF of [`MAX_PAGES`] is read in.
const MAX_PARTS: u32 = MAX_PAGES.div_ceil(PART);

/// How long one call of a tool may run. A part of [`PART`] pages of a large
/// standard is read in a few seconds; a file that keeps the tool busy longer
/// than this is refused.
const TIMEOUT: Duration = Duration::from_secs(120);

/// How often the end of a tool is looked for, and how many times at most:
/// [`TIMEOUT`] in all.
const POLL: Duration = Duration::from_millis(5);
const MAX_POLLS: u32 = 24_000;

/// The most bytes one call of a tool may write on its standard output: far
/// more than the text of [`PART`] pages, and a bound on the memory a file
/// made for it can take.
const MAX_OUTPUT: u64 = 64 * 1024 * 1024;

/// The most bytes read of what a tool says on its standard error: its first
/// line is shown.
const MAX_SAID: u64 = 64 * 1024;

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

/// Reads `source` to its end, but no more than `limit` bytes and one: the
/// one byte over tells the caller that there was more.
///
/// Input: `source`, the output of a tool; `limit`, the most bytes taken.
/// Output: the bytes read, at most `limit + 1` (`b"Pages: 471\n"`); an error
/// of the reading.
///
/// Called by: [`run`], on a thread of its own for each output of the tool.
fn read_bounded(source: impl Read, limit: u64) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();

    source.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;

    Ok(bytes)
}

/// Waits for the tool `child` to end, at most [`TIMEOUT`]; one that runs
/// longer (a file made to keep it busy) is killed.
///
/// Input: `child`, the tool started; `tool`, its name, for the error.
/// Output: its exit status; `PdfError::Unreadable` when it did not end in
/// time (it is killed and awaited, so that no process is left behind), or
/// when the system cannot tell.
///
/// Called by: [`run`].
fn waited(child: &mut Child, tool: &str) -> Result<ExitStatus, PdfError> {
    for _ in 0..MAX_POLLS {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => std::thread::sleep(POLL),
            Err(error) => return Err(PdfError::Unreadable(error.to_string())),
        }
    }

    child.kill().map_err(|error| PdfError::Unreadable(format!("{tool} could not be stopped: {error}")))?;
    child.wait().map_err(|error| PdfError::Unreadable(error.to_string()))?;

    Err(PdfError::Unreadable(format!("{tool} did not end in {} seconds", TIMEOUT.as_secs())))
}

/// What a thread of [`run`] read or wrote; a thread that failed is an error.
///
/// Input: `joined`, the result of the thread; `tool`, the name of the tool.
/// Output: what the thread gave; `PdfError::Unreadable` when it failed.
///
/// Called by: [`run`].
fn joined<T>(joined: std::thread::Result<std::io::Result<T>>, tool: &str) -> Result<T, PdfError> {
    match joined {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(PdfError::Unreadable(format!("{tool}: {error}"))),
        Err(_) => Err(PdfError::Unreadable(format!("the work with {tool} failed"))),
    }
}

/// Runs a tool of Poppler with the PDF on its standard input. The file
/// comes from outside the laboratory (the manufacturer of the product), so
/// the tool is bounded: it runs at most [`TIMEOUT`], and writes at most
/// [`MAX_OUTPUT`] bytes, past which its output is closed and it ends.
///
/// Input:
/// - `tool`: the tool (`"pdfinfo"`, `"pdftotext"`).
/// - `arguments`: its arguments, the PDF named `fd://0`.
/// - `pdf`: the bytes of the PDF, within [`MAX_FILE`].
///
/// Output: what the tool wrote on its standard output; `PdfError::ToolMissing`
/// when it is not installed, `PdfError::Unreadable` with what it said when it
/// fails, when it runs too long or writes too much.
///
/// Called by: [`info`], [`text_of_pages`].
/// Calls: [`read_bounded`], [`waited`], [`joined`].
fn run(tool: &'static str, arguments: &[&str], pdf: &[u8]) -> Result<Vec<u8>, PdfError> {
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
    // The three were asked for as pipes above, so the child has them.
    let missing = || PdfError::Unreadable(format!("{tool} has no pipes"));
    let mut stdin = child.stdin.take().ok_or_else(missing)?;
    let stdout = child.stdout.take().ok_or_else(missing)?;
    let stderr = child.stderr.take().ok_or_else(missing)?;

    // The PDF is written and both outputs are read on threads of their own
    // while the tool is awaited here, so that no side waits for another with
    // a full pipe. A thread that ends drops its pipe: the tool then sees its
    // input ended, or its output closed when it wrote past the limit.
    let (written, output, said, status) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(pdf));
        let reader = scope.spawn(move || read_bounded(stdout, MAX_OUTPUT));
        let errors = scope.spawn(move || read_bounded(stderr, MAX_SAID));
        let status = waited(&mut child, tool);

        (writer.join(), reader.join(), errors.join(), status)
    });
    let status = status?;
    let output = joined(output, tool)?;
    let said = joined(said, tool)?;

    ensure!(
        u64::try_from(output.len()).is_ok_and(|length| length <= MAX_OUTPUT),
        PdfError::Unreadable(format!("{tool} wrote more than {} MiB", MAX_OUTPUT / 1024 / 1024))
    );

    // A tool that stops reading early (a broken file) breaks the pipe: that
    // is no error here, its exit status tells. Any other failure of the
    // writing is.
    match written {
        Ok(Ok(())) => {}
        Ok(Err(error)) if error.kind() == ErrorKind::BrokenPipe => {}
        Ok(Err(error)) => return Err(PdfError::Unreadable(error.to_string())),
        Err(_) => return Err(PdfError::Unreadable(format!("the PDF could not be given to {tool}"))),
    }

    if !status.success() {
        let said = String::from_utf8_lossy(&said);
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
    let text = String::from_utf8_lossy(&output);

    let named =
        field(&text, "Pages:").ok_or_else(|| PdfError::Unreadable("pdfinfo names no number of pages".to_owned()))?;
    let pages: u32 =
        named.parse().map_err(|error| PdfError::Unreadable(format!("pdfinfo names \"{named}\" pages: {error}")))?;

    ensure!(pages > 0 && pages <= MAX_PAGES, PdfError::Unreadable(format!("the file has {pages} pages")));

    Ok(PdfInfo { pages, title: field(&text, "Title:") })
}

/// The value of the field `name` in what `pdfinfo` wrote.
///
/// Input: `text`, what `pdfinfo` wrote, a field a line; `name`, the field
/// with its colon (`"Pages:"`).
/// Output: its value without spaces around (`Some("471")` of a line
/// "Pages:   471"); `None` when no line has the field or its value is empty.
///
/// Called by: [`info`].
fn field(text: &str, name: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(value) = line.strip_prefix(name) {
            let value = value.trim();

            return if value.is_empty() { None } else { Some(value.to_owned()) };
        }
    }

    None
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

    let text = String::from_utf8_lossy(&output);
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
/// Input: `pdf`, the bytes of the PDF, within [`MAX_FILE`].
/// Output: its text ("Page one...\n\nPage two..."); the errors of [`info`]
/// and [`text_of_pages`], and `PdfError::Unreadable` when not every page
/// was read.
///
/// Called by: `objects/ai/rag/rag_rules.rs`: `source_text` (a source);
/// `objects/evaluation/product_document/product_document_rules.rs`: `document_text` (a
/// document of the product).
/// Calls: [`info`], [`text_of_pages`].
pub fn text(pdf: &[u8]) -> Result<String, PdfError> {
    let pages = info(pdf)?.pages;
    let mut text = String::new();
    let mut first = 1;

    // Each turn reads one part; `info` checked the pages against MAX_PAGES,
    // so MAX_PARTS turns read them all.
    for _ in 0..MAX_PARTS {
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

    ensure!(first > pages, PdfError::Unreadable(format!("only {} of {pages} pages read", first - 1)));

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
    fn an_output_is_read_within_its_limit_and_one_byte_tells_of_more() {
        assert_eq!(read_bounded(&b"abc"[..], 3).unwrap(), b"abc");
        assert_eq!(read_bounded(&b"abcdef"[..], 3).unwrap(), b"abcd");
        assert!(read_bounded(&b""[..], 3).unwrap().is_empty());
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
