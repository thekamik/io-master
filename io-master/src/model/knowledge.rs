//! The knowledge bases of the AI (step 57.2 of docs/todo.md): the sources
//! the evaluator gives a base (a PDF, a text file, a catalog of OSCAL, a
//! note), cut into passages of a bounded size, each with its page or its
//! requirement, and the words a search looks for. Plain data and rules; the
//! passages are kept and searched by `db/knowledge/` (FTS5 of SQLite).

use crate::model::text::{Field, Invalid, check_text};

/// The most knowledge bases of an evaluator.
pub const MAX_BASES: usize = 50;

/// The most sources of one base.
pub const MAX_SOURCES: usize = 200;

/// The most passages of one source: a standard of some thousand pages.
pub const MAX_PASSAGES: usize = 50_000;

/// The most characters of a passage (user, 2026-10-25: a paragraph, within
/// one page, so that a quote can be shown with its page).
pub const PASSAGE_CHARS: usize = 1500;

/// The longest name of a base or of a source.
pub const NAME_MAX: usize = 200;

/// The longest description of a base.
pub const DESCRIPTION_MAX: usize = 2000;

/// The longest note given as a source.
pub const NOTE_MAX: usize = 1_000_000;

/// The largest PDF read as a source, in bytes.
pub const PDF_MAX: u64 = 128 * 1024 * 1024;

/// The largest text file read as a source, in bytes.
pub const TEXT_FILE_MAX: u64 = 16 * 1024 * 1024;

/// The most pages of a PDF read as a source.
pub const MAX_PAGES: u32 = 5000;

/// The longest text searched for, in characters.
pub const QUERY_MAX: usize = 2000;

/// The most words of a search.
pub const MAX_TERMS: usize = 64;

/// The most passages a search gives.
pub const MAX_HITS: usize = 50;

/// What a source of a base is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// A PDF; its text is read page by page.
    Pdf,
    /// A text file.
    Text,
    /// A catalog of OSCAL of the installation: one requirement after the other.
    Catalog,
    /// A text written or pasted by the evaluator (an interpretation, a note of the scheme).
    Note,
}

impl SourceKind {
    pub const ALL: [SourceKind; 4] = [SourceKind::Pdf, SourceKind::Text, SourceKind::Catalog, SourceKind::Note];

    pub fn name(self) -> &'static str {
        match self {
            SourceKind::Pdf => "pdf",
            SourceKind::Text => "text",
            SourceKind::Catalog => "catalog",
            SourceKind::Note => "note",
        }
    }

    pub fn from_name(name: &str) -> Option<SourceKind> {
        SourceKind::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

/// A knowledge base: its name and what it is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeBase {
    pub id: i64,
    pub name: String,
    pub description: String,
}

/// A knowledge base in the list, with the numbers of its sources and passages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseSummary {
    pub base: KnowledgeBase,
    pub sources: usize,
    pub passages: usize,
}

/// A source of a base, with the number of its passages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub id: i64,
    pub kind: SourceKind,
    pub name: String,
    pub passages: usize,
    pub added: i64,
}

/// A passage of a source: its text, and where it stands (the page of a PDF
/// or of a text, the label of a requirement of a catalog).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passage {
    pub page: Option<u32>,
    pub label: String,
    pub text: String,
}

/// A passage found by a search, with its source and its score (BM25 of
/// SQLite: the lower, the better it fits).
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub id: i64,
    pub source: String,
    pub passage: Passage,
    pub score: f64,
}

/// Checks the name and the description of a base: a name not empty, both
/// within their limits.
///
/// Called by: `views/knowledge/knowledge_list/` and `knowledge_base/` (a base made or changed).
pub fn check_base(name: &str, description: &str) -> Result<(), Invalid> {
    if name.trim().is_empty() {
        return Err(Invalid::new(Field::Name, "the name is empty"));
    }

    check_text(Field::Name, name, NAME_MAX, false)?;
    check_text(Field::Description, description, DESCRIPTION_MAX, true)
}

/// The passages of the text `text` of one page (or of one requirement,
/// `label`): its paragraphs (parted by an empty line), joined while they
/// fit in [`PASSAGE_CHARS`]; a longer paragraph is cut between words. The
/// white space inside a paragraph becomes one space.
///
/// Called by: `views/knowledge/knowledge_base/add.rs` (every source added).
/// Calls: [`words_into`].
pub fn passages(page: Option<u32>, label: &str, text: &str) -> Vec<Passage> {
    let mut texts: Vec<String> = Vec::new();
    let mut current = String::new();

    for paragraph in text.replace('\r', "").split("\n\n") {
        let paragraph = paragraph.split_whitespace().collect::<Vec<&str>>().join(" ");

        if paragraph.is_empty() {
            continue;
        }

        let joined = current.chars().count() + 2 + paragraph.chars().count();

        if !current.is_empty() && joined <= PASSAGE_CHARS {
            current.push_str("\n\n");
            current.push_str(&paragraph);
            continue;
        }

        if !current.is_empty() {
            texts.push(std::mem::take(&mut current));
        }

        current = words_into(&paragraph, &mut texts);
    }

    if !current.is_empty() {
        texts.push(current);
    }

    let mut found = Vec::with_capacity(texts.len());

    for text in texts {
        found.push(Passage { page, label: label.to_owned(), text });
    }

    found
}

/// Cuts the paragraph `paragraph` between words into pieces of at most
/// [`PASSAGE_CHARS`]: every full piece goes to `texts`, the last one is
/// given back. A word longer than a passage is cut by characters.
///
/// Called by: [`passages`].
fn words_into(paragraph: &str, texts: &mut Vec<String>) -> String {
    let mut current = String::new();
    let mut count = 0;

    for word in paragraph.split(' ') {
        let length = word.chars().count();

        if count > 0 && count + 1 + length > PASSAGE_CHARS {
            texts.push(std::mem::take(&mut current));
            count = 0;
        }

        if length > PASSAGE_CHARS {
            let characters: Vec<char> = word.chars().collect();

            for piece in characters.chunks(PASSAGE_CHARS) {
                texts.push(piece.iter().collect());
            }

            continue;
        }

        if count > 0 {
            current.push(' ');
            count += 1;
        }

        current.push_str(word);
        count += length;
    }

    current
}

/// The query of FTS5 for the text `text`: every word (at most
/// [`MAX_TERMS`]) a phrase of its letters and digits (`FCS_CKM.1` becomes
/// `"FCS CKM 1"`, as the index cuts it), the phrases joined by `OR`, so that
/// BM25 ranks the passages that hold most of them, the rarest weighing most.
/// `None` when the text has no word to look for. The phrases hold letters,
/// digits and spaces only, so nothing of the text can change the query.
///
/// Called by: `views/knowledge/knowledge_search/search.rs`.
pub fn search_query(text: &str) -> Option<String> {
    let mut phrases: Vec<String> = Vec::new();

    for word in text.split_whitespace().take(MAX_TERMS) {
        let mut phrase = String::new();

        for character in word.chars() {
            if character.is_alphanumeric() {
                phrase.push(character);
            } else if !phrase.is_empty() && !phrase.ends_with(' ') {
                phrase.push(' ');
            }
        }

        let phrase = phrase.trim_end();

        if phrase.chars().count() < 2 {
            continue;
        }

        phrases.push(format!("\"{phrase}\""));
    }

    if phrases.is_empty() { None } else { Some(phrases.join(" OR ")) }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `knowledge.rs`.

    use super::*;

    #[test]
    fn a_kind_has_a_name_it_is_read_from() {
        for kind in SourceKind::ALL {
            assert_eq!(SourceKind::from_name(kind.name()), Some(kind));
        }

        assert_eq!(SourceKind::from_name("video"), None);
    }

    #[test]
    fn a_base_needs_a_name() {
        assert!(check_base("Common Criteria", "CC Part 1 to 3 and the CEM.").is_ok());
        assert!(check_base("  ", "").is_err());
        assert!(check_base(&"n".repeat(NAME_MAX + 1), "").is_err());
    }

    #[test]
    fn short_paragraphs_are_joined_and_keep_their_page() {
        let found = passages(Some(7), "", "The TSF shall\ngenerate keys.\n\nSecond   paragraph.\n\n\n");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "The TSF shall generate keys.\n\nSecond paragraph.");
        assert_eq!(found[0].page, Some(7));
    }

    #[test]
    fn a_long_paragraph_is_cut_between_words() {
        let long = "word ".repeat(700);

        let found = passages(None, "FCS_CKM.1", &format!("Intro.\n\n{long}"));

        assert_eq!(found[0].text, "Intro.");
        assert!(found.len() >= 3);

        for passage in &found {
            assert!(passage.text.chars().count() <= PASSAGE_CHARS);
            assert_eq!(passage.label, "FCS_CKM.1");
        }
    }

    #[test]
    fn a_word_longer_than_a_passage_is_cut() {
        let found = passages(None, "", &"x".repeat(PASSAGE_CHARS * 2 + 5));

        assert_eq!(found.len(), 3);
        assert!(passages(None, "", " \n\n ").is_empty());
    }

    #[test]
    fn a_search_becomes_phrases_joined_by_or() {
        assert_eq!(search_query("FCS_CKM.1 key \"generation\"").unwrap(), r#""FCS CKM 1" OR "key" OR "generation""#);
        assert_eq!(search_query("a - ;"), None);
        assert_eq!(search_query(&"w ".repeat(200)), None, "one letter is no word");
        assert_eq!(search_query(&"wo ".repeat(200)).unwrap().matches(" OR ").count(), MAX_TERMS - 1);
    }
}
