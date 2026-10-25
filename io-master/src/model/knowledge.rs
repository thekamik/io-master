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

/// The most agents of an evaluator.
pub const MAX_AGENTS: usize = 50;

/// The longest instructions of an agent.
pub const AGENT_INSTRUCTIONS_MAX: usize = 20_000;

/// The longest glossary of an agent, and the most entries in it.
pub const GLOSSARY_MAX: usize = 50_000;
pub const MAX_GLOSSARY_ENTRIES: usize = 500;

/// The most passages an agent gives the model with one question, and the
/// number of a new agent.
pub const MAX_AGENT_PASSAGES: usize = 20;
pub const DEFAULT_AGENT_PASSAGES: usize = 8;

/// The characters of the passages an agent gives with one question: the
/// least, the most, and those of a new agent.
pub const MIN_CONTEXT_CHARS: usize = PASSAGE_CHARS;
pub const MAX_CONTEXT_CHARS: usize = 40_000;
pub const DEFAULT_CONTEXT_CHARS: usize = 12_000;

/// The longest question of the proving ground.
pub const QUESTION_MAX: usize = 4000;

/// The most quotes of an answer that are checked.
pub const MAX_QUOTES: usize = 20;

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

/// Where an agent with knowledge is attached (step 57.6): every work of
/// the AI of a master project, of the conversion of a catalog, or of a chat
/// uses the agent of its owner. (The Evaluation lab keeps its agents in
/// tables of its own, step 57.4, since a task may change that of its
/// evaluation.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPlace {
    /// A master project of the Evaluation plan (its UUID).
    Project,
    /// A catalog converted in the Converter (its UUID).
    Catalog,
    /// A chat of the view Chat (its id).
    Chat,
}

impl AgentPlace {
    pub fn name(self) -> &'static str {
        match self {
            AgentPlace::Project => "project",
            AgentPlace::Catalog => "catalog",
            AgentPlace::Chat => "chat",
        }
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

/// What the evaluator sets of an agent (step 57.3): its name, its
/// instructions (who it is, how it answers), its glossary (lines `term =
/// other term`), how many passages and characters it gives the model, and
/// the bases it draws on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentFields {
    pub name: String,
    pub instructions: String,
    pub glossary: String,
    pub passages: usize,
    pub characters: usize,
    pub bases: Vec<i64>,
}

/// An agent of the AI with knowledge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub id: i64,
    pub fields: AgentFields,
}

/// An agent in the list, with the number of its bases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSummary {
    pub id: i64,
    pub name: String,
    pub bases: usize,
}

/// A quote of an answer of the proving ground: the number of the passage
/// it names (from 1) and whether the program found it there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialQuote {
    pub passage: usize,
    pub quote: String,
    pub found: bool,
}

/// The last question of the proving ground of an agent: the question, the
/// passages it was given (in their order, numbered from 1), the answer and
/// its quotes, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Trial {
    pub question: String,
    pub passages: Vec<Hit>,
    pub answer: String,
    pub quotes: Vec<TrialQuote>,
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

/// Checks what the evaluator set of an agent: a name, the limits of every
/// text, the numbers of passages and characters within their bounds, at
/// most `MAX_BASES` bases, a glossary that reads (`glossary`).
///
/// Called by: `views/knowledge/` (an agent made or saved).
/// Calls: `check_text`, [`glossary`].
pub fn check_agent(fields: &AgentFields) -> Result<(), Invalid> {
    if fields.name.trim().is_empty() {
        return Err(Invalid::new(Field::Name, "the name is empty"));
    }

    check_text(Field::Name, &fields.name, NAME_MAX, false)?;
    check_text(Field::Instructions, &fields.instructions, AGENT_INSTRUCTIONS_MAX, true)?;
    check_text(Field::Glossary, &fields.glossary, GLOSSARY_MAX, true)?;

    if !(1..=MAX_AGENT_PASSAGES).contains(&fields.passages) {
        return Err(Invalid::new(Field::Kind, format!("an agent gives 1 to {MAX_AGENT_PASSAGES} passages")));
    }

    if !(MIN_CONTEXT_CHARS..=MAX_CONTEXT_CHARS).contains(&fields.characters) {
        return Err(Invalid::new(
            Field::Kind,
            format!("an agent gives {MIN_CONTEXT_CHARS} to {MAX_CONTEXT_CHARS} characters of passages"),
        ));
    }

    if fields.bases.len() > MAX_BASES {
        return Err(Invalid::new(Field::Kind, format!("an agent draws on at most {MAX_BASES} bases")));
    }

    glossary(&fields.glossary)?;

    Ok(())
}

/// The entries of the glossary `text`: one per line, `term = other term`
/// (an abbreviation and what it stands for, two names of one thing); an
/// empty line or one that starts with `#` is skipped. Refused when a line
/// has no `=` or an empty side, or there are more than
/// [`MAX_GLOSSARY_ENTRIES`].
///
/// Called by: [`check_agent`], `views/knowledge/proving_ground/` (the words of a search widened).
pub fn glossary(text: &str) -> Result<Vec<(String, String)>, Invalid> {
    let mut entries = Vec::new();

    for (number, line) in (1..).zip(text.lines()) {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((term, other)) = line.split_once('=') else {
            return Err(Invalid::new(Field::Glossary, format!("line {number} of the glossary has no \"=\"")));
        };

        if term.trim().is_empty() || other.trim().is_empty() {
            return Err(Invalid::new(Field::Glossary, format!("line {number} of the glossary has an empty side")));
        }

        if entries.len() == MAX_GLOSSARY_ENTRIES {
            return Err(Invalid::new(
                Field::Glossary,
                format!("a glossary has at most {MAX_GLOSSARY_ENTRIES} entries"),
            ));
        }

        entries.push((term.trim().to_owned(), other.trim().to_owned()));
    }

    Ok(entries)
}

/// The text `text` widened by the glossary `entries`: for every entry one
/// of whose sides the text holds as a whole word (any case), the other
/// side is added, so that a search finds both.
///
/// Called by: `views/knowledge/proving_ground/` (before `search_query`).
/// Calls: [`holds_term`].
pub fn widened(text: &str, entries: &[(String, String)]) -> String {
    let lower = text.to_lowercase();
    let mut widened = text.to_owned();

    for (term, other) in entries {
        if holds_term(&lower, &term.to_lowercase()) {
            widened.push(' ');
            widened.push_str(other);
        }

        if holds_term(&lower, &other.to_lowercase()) {
            widened.push(' ');
            widened.push_str(term);
        }
    }

    widened
}

/// Whether `text` holds `term` as a whole word or words: not inside a
/// longer word (`TSF` is not in `TSFI`).
///
/// Called by: [`widened`].
fn holds_term(text: &str, term: &str) -> bool {
    if term.is_empty() {
        return false;
    }

    for (start, _) in text.match_indices(term) {
        let before = text.get(..start).and_then(|head| head.chars().next_back());
        let after = text.get(start + term.len()..).and_then(|tail| tail.chars().next());
        if !inside_word(before) && !inside_word(after) {
            return true;
        }
    }

    false
}

/// Whether the character next to a term (none at the start or the end of
/// the text) makes it part of a longer word.
///
/// Called by: [`holds_term`].
fn inside_word(neighbour: Option<char>) -> bool {
    match neighbour {
        Some(character) => character.is_alphanumeric(),
        None => false,
    }
}

/// The passages an agent gives the model, from `hits` (the best first): at
/// most `passages` of them, of at most `characters` together; a passage
/// that does not fit is passed over for a shorter one after it.
///
/// Called by: `views/knowledge/proving_ground/`.
pub fn chosen(hits: Vec<Hit>, passages: usize, characters: usize) -> Vec<Hit> {
    let mut chosen = Vec::with_capacity(passages.min(hits.len()));
    let mut total = 0;

    for hit in hits {
        let length = hit.passage.text.chars().count();

        if chosen.len() == passages {
            break;
        }

        if total + length > characters {
            continue;
        }

        total += length;
        chosen.push(hit);
    }

    chosen
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

    fn agent() -> AgentFields {
        AgentFields {
            name: "CC agent".to_owned(),
            instructions: "You know Common Criteria.".to_owned(),
            glossary: "# abbreviations\nTSF = TOE Security Functionality\n\nST = Security Target".to_owned(),
            passages: DEFAULT_AGENT_PASSAGES,
            characters: DEFAULT_CONTEXT_CHARS,
            bases: vec![1, 2],
        }
    }

    fn hit(id: i64, length: usize) -> Hit {
        Hit {
            id,
            source: "CC".to_owned(),
            passage: Passage { page: None, label: String::new(), text: "x".repeat(length) },
            score: -1.0,
        }
    }

    #[test]
    fn an_agent_is_checked() {
        assert!(check_agent(&agent()).is_ok());

        let mut wrong = agent();
        wrong.passages = 0;
        assert!(check_agent(&wrong).is_err());

        let mut wrong = agent();
        wrong.characters = MAX_CONTEXT_CHARS + 1;
        assert!(check_agent(&wrong).is_err());

        let mut wrong = agent();
        wrong.glossary = "TSF TOE Security Functionality".to_owned();
        assert_eq!(check_agent(&wrong).unwrap_err().field, Field::Glossary);
    }

    #[test]
    fn the_glossary_is_read_line_by_line() {
        let entries = glossary(&agent().glossary).unwrap();

        assert_eq!(
            entries,
            [
                ("TSF".to_owned(), "TOE Security Functionality".to_owned()),
                ("ST".to_owned(), "Security Target".to_owned())
            ]
        );
        assert!(glossary("TSF = ").is_err());
        assert!(glossary(&"a = b\n".repeat(MAX_GLOSSARY_ENTRIES + 1)).is_err());
    }

    #[test]
    fn a_question_is_widened_by_the_glossary_both_ways() {
        let entries = glossary(&agent().glossary).unwrap();

        assert_eq!(widened("What does the TSF do?", &entries), "What does the TSF do? TOE Security Functionality");
        assert_eq!(widened("the security target of a TOE", &entries), "the security target of a TOE ST");
        assert_eq!(widened("The TSFI and the STs", &entries), "The TSFI and the STs", "only whole words");
    }

    #[test]
    fn the_passages_chosen_keep_both_limits() {
        let hits = vec![hit(1, 1000), hit(2, 1400), hit(3, 500), hit(4, 100)];

        let chosen_ids: Vec<i64> = chosen(hits.clone(), 3, 2000).into_iter().map(|hit| hit.id).collect();
        assert_eq!(chosen_ids, [1, 3, 4], "the second does not fit, shorter ones after it do");

        assert_eq!(chosen(hits, 1, 5000).len(), 1);
    }

    #[test]
    fn a_search_becomes_phrases_joined_by_or() {
        assert_eq!(search_query("FCS_CKM.1 key \"generation\"").unwrap(), r#""FCS CKM 1" OR "key" OR "generation""#);
        assert_eq!(search_query("a - ;"), None);
        assert_eq!(search_query(&"w ".repeat(200)), None, "one letter is no word");
        assert_eq!(search_query(&"wo ".repeat(200)).unwrap().matches(" OR ").count(), MAX_TERMS - 1);
    }
}
