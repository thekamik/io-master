//! The knowledge for the AI (RAG): the knowledge bases (`KnowledgeBase`)
//! and their sources (`Source`), in the tables `knowledge_bases` and
//! `knowledge_sources` of the database of the user: create, read, update;
//! never deleted (user, 2026-10-28). With them the index of the passages,
//! which is no model: whenever the text of a source is written, it is cut
//! into passages of at most [`PASSAGE_CHARS`] characters and indexed by
//! FTS5 of SQLite anew, and [`read_passages`] finds those that fit a
//! question.
//!
//! ```text
//! knowledge_bases      id, name (unique), description
//! knowledge_sources    id, knowledge_base, name, text
//! knowledge_passages   the index (FTS5): text, source; made from the text of the sources
//! ```

use rusqlite::{Connection, Row, named_params};

use crate::db::error::DbError;
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::rag::{KnowledgeBase, Source};

/// The most knowledge bases, and sources of one base.
pub const MAX_BASES: usize = 50;
pub const MAX_SOURCES: usize = 200;

/// The longest passage, in characters.
pub const PASSAGE_CHARS: usize = 1500;

/// The longest text of a source, in characters (a standard of 500 pages
/// is about 1.5 million).
pub const MAX_SOURCE_CHARS: usize = 20_000_000;

/// The most passages a search gives, and the most words of a question it
/// looks for.
pub const MAX_FOUND: usize = 50;
const MAX_WORDS: usize = 64;

/// The words of English a search passes over (user, 2026-10-29): they are
/// in almost every passage, so a question that names them ("What is the
/// TSF?") would find passages by "is" and "the" instead of by "TSF".
const STOP_WORDS: [&str; 64] = [
    "a", "about", "all", "also", "an", "and", "any", "are", "as", "at", "be", "been", "but", "by", "can", "do", "does",
    "each", "for", "from", "has", "have", "he", "her", "his", "how", "i", "if", "in", "into", "is", "it", "its", "me",
    "my", "no", "not", "of", "on", "or", "other", "our", "shall", "she", "should", "so", "such", "that", "the",
    "their", "them", "then", "there", "these", "they", "this", "to", "was", "we", "what", "when", "which", "who",
    "with",
];

/// The tables, made when they are missing (`schema.rs`, `USER_TABLES`). The tokenizer
/// `porter` finds an English word by its stem ("keys" finds "key"),
/// `unicode61` folds the case and the diacritics.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS knowledge_bases (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL UNIQUE,
        description TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS knowledge_sources (
        id             INTEGER PRIMARY KEY,
        knowledge_base INTEGER NOT NULL REFERENCES knowledge_bases(id),
        name           TEXT NOT NULL,
        text           TEXT NOT NULL
    );

    CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_passages USING fts5(
        text,
        source UNINDEXED,
        tokenize = 'porter unicode61 remove_diacritics 2'
    );
";

/// Stores a new knowledge base and gives its id. A new object has no id
/// yet (`id` 0): the database gives it.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn create_knowledge_base(db: &UserDatabase, base: &KnowledgeBase) -> Result<i64, DbError> {
    ensure!(base.id == 0, DbError::Inconsistent("a new knowledge base has an id already".to_owned()));
    ensure!(!base.name.is_empty(), DbError::Inconsistent("a knowledge base without a name".to_owned()));

    let connection = db.connection();
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM knowledge_bases", [], |row| row.get(0))?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_BASES,
        DbError::Inconsistent(format!("more than {MAX_BASES} knowledge bases"))
    );

    connection.execute(
        "INSERT INTO knowledge_bases (name, description) VALUES (:name, :description)",
        named_params! { ":name": base.name, ":description": base.description },
    )?;

    Ok(connection.last_insert_rowid())
}

/// The model of a row of `knowledge_bases`.
///
/// Called by: [`read_knowledge_base`], [`read_knowledge_bases`].
fn base_of(row: &Row<'_>) -> Result<KnowledgeBase, DbError> {
    Ok(KnowledgeBase { id: row.get("id")?, name: row.get("name")?, description: row.get("description")? })
}

/// Reads the knowledge base `id`.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`base_of`].
pub fn read_knowledge_base(db: &UserDatabase, id: i64) -> Result<KnowledgeBase, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT id, name, description FROM knowledge_bases WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "knowledge_bases", id });
    };

    base_of(row)
}

/// Reads every knowledge base, by its name.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`base_of`].
pub fn read_knowledge_bases(db: &UserDatabase) -> Result<Vec<KnowledgeBase>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT id, name, description FROM knowledge_bases ORDER BY name")?;
    let mut rows = statement.query([])?;
    let mut bases = Vec::new();

    for _ in 0..=MAX_BASES {
        let Some(row) = rows.next()? else {
            return Ok(bases);
        };

        bases.push(base_of(row)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_BASES} knowledge bases in the database")))
}

/// Writes the knowledge base `base.id` anew.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn update_knowledge_base(db: &UserDatabase, base: &KnowledgeBase) -> Result<(), DbError> {
    ensure!(!base.name.is_empty(), DbError::Inconsistent("a knowledge base without a name".to_owned()));

    let changed = db.connection().execute(
        "UPDATE knowledge_bases SET name = :name, description = :description WHERE id = :id",
        named_params! { ":id": base.id, ":name": base.name, ":description": base.description },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "knowledge_bases", id: base.id });

    Ok(())
}

/// Cuts `text` into passages of at most [`PASSAGE_CHARS`] characters: whole
/// paragraphs (parted by an empty line) joined while they fit, a longer
/// paragraph cut between words, a word longer than a passage cut where it
/// reaches the limit. The spaces between the words become one.
///
/// Called by: [`write_passages`]; `db/models/evaluation/product_document.rs` (the
/// index of a document of the product).
/// Calls: [`push_passage`].
pub(in crate::db) fn passages(text: &str) -> Vec<String> {
    let mut passages = Vec::new();
    let mut current = String::new();

    for paragraph in text.split("\n\n") {
        if !current.is_empty() && !current.ends_with('\n') {
            current.push('\n');
        }

        for word in paragraph.split_whitespace() {
            let mut rest: Vec<char> = word.chars().collect();

            // Each turn places the word, closes the passage, or takes a
            // passage of a word longer than one: at most one turn a character.
            for _ in 0..=rest.len() {
                let room = PASSAGE_CHARS.saturating_sub(current.chars().count() + 1);

                if rest.len() <= room || (current.is_empty() && rest.len() <= PASSAGE_CHARS) {
                    if !current.is_empty() && !current.ends_with('\n') {
                        current.push(' ');
                    }

                    current.extend(rest.drain(..));
                    break;
                }

                if !current.is_empty() {
                    push_passage(&mut passages, &mut current);
                    continue;
                }

                current.extend(rest.drain(..PASSAGE_CHARS));
                push_passage(&mut passages, &mut current);
            }
        }
    }

    push_passage(&mut passages, &mut current);

    passages
}

/// Closes the passage `current`: adds it to `passages` without the spaces
/// around it, when it holds anything, and empties it.
///
/// Called by: [`passages`].
fn push_passage(passages: &mut Vec<String>, current: &mut String) {
    let passage = current.trim();

    if !passage.is_empty() {
        passages.push(passage.to_owned());
    }

    current.clear();
}

/// Makes the index of the source `id` anew from its text. Run inside the
/// transaction of its caller; the rows of the index are no data of their
/// own, so their replacement is no deletion of a model.
///
/// Called by: [`create_source`], [`update_source`].
/// Calls: [`passages`].
fn write_passages(connection: &Connection, id: i64, text: &str) -> Result<(), DbError> {
    connection.execute("DELETE FROM knowledge_passages WHERE source = :id", named_params! { ":id": id })?;

    for passage in passages(text) {
        connection.execute(
            "INSERT INTO knowledge_passages (text, source) VALUES (:text, :id)",
            named_params! { ":text": passage, ":id": id },
        )?;
    }

    Ok(())
}

/// Refuses a source without a name, or with a text over its limit.
///
/// Called by: [`create_source`], [`update_source`].
fn check_source(source: &Source) -> Result<(), DbError> {
    ensure!(!source.name.is_empty(), DbError::Inconsistent("a source without a name".to_owned()));
    ensure!(
        source.text.chars().count() <= MAX_SOURCE_CHARS,
        DbError::Inconsistent(format!("a source over {MAX_SOURCE_CHARS} characters"))
    );

    Ok(())
}

/// Stores a new source of its knowledge base with the index of its
/// passages, in one transaction, and gives its id. A new object has no id
/// yet (`id` 0): the database gives it.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_source`], [`write_passages`].
pub fn create_source(db: &UserDatabase, source: &Source) -> Result<i64, DbError> {
    ensure!(source.id == 0, DbError::Inconsistent("a new source has an id already".to_owned()));
    check_source(source)?;

    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM knowledge_sources WHERE knowledge_base = :base",
        named_params! { ":base": source.knowledge_base },
        |row| row.get(0),
    )?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_SOURCES,
        DbError::Inconsistent(format!("more than {MAX_SOURCES} sources of a knowledge base"))
    );

    transaction.execute(
        "INSERT INTO knowledge_sources (knowledge_base, name, text) VALUES (:base, :name, :text)",
        named_params! { ":base": source.knowledge_base, ":name": source.name, ":text": source.text },
    )?;

    let id = transaction.last_insert_rowid();

    write_passages(&transaction, id, &source.text)?;
    transaction.commit()?;

    Ok(id)
}

/// The model of a row of `knowledge_sources`.
///
/// Called by: [`read_source`], [`read_sources`].
fn source_of(row: &Row<'_>) -> Result<Source, DbError> {
    Ok(Source {
        id: row.get("id")?,
        knowledge_base: row.get("knowledge_base")?,
        name: row.get("name")?,
        text: row.get("text")?,
    })
}

/// Reads the source `id`, with its text.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`source_of`].
pub fn read_source(db: &UserDatabase, id: i64) -> Result<Source, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT id, knowledge_base, name, text FROM knowledge_sources WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "knowledge_sources", id });
    };

    source_of(row)
}

/// Reads every source of the knowledge base `base`, by its name.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`source_of`].
pub fn read_sources(db: &UserDatabase, base: i64) -> Result<Vec<Source>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT id, knowledge_base, name, text FROM knowledge_sources WHERE knowledge_base = :base ORDER BY name",
    )?;
    let mut rows = statement.query(named_params! { ":base": base })?;
    let mut sources = Vec::new();

    for _ in 0..=MAX_SOURCES {
        let Some(row) = rows.next()? else {
            return Ok(sources);
        };

        sources.push(source_of(row)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_SOURCES} sources of a knowledge base in the database")))
}

/// Writes the source `source.id` anew, with the index of its passages, in
/// one transaction.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_source`], [`write_passages`].
pub fn update_source(db: &UserDatabase, source: &Source) -> Result<(), DbError> {
    check_source(source)?;

    let transaction = db.connection().unchecked_transaction()?;
    let changed = transaction.execute(
        "UPDATE knowledge_sources SET knowledge_base = :base, name = :name, text = :text WHERE id = :id",
        named_params! { ":id": source.id, ":base": source.knowledge_base, ":name": source.name, ":text": source.text },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "knowledge_sources", id: source.id });

    write_passages(&transaction, source.id, &source.text)?;
    transaction.commit()?;

    Ok(())
}

/// The query of FTS5 for a question: its words but the [`STOP_WORDS`], each
/// quoted (so that no word is read as an operator of FTS5), any of them;
/// `None` for a question without such words.
///
/// Called by: [`read_passages`]; `db/models/evaluation/product_document.rs` (the
/// search in the documents of the product).
pub(in crate::db) fn query_of(question: &str) -> Option<String> {
    let mut words = Vec::new();

    // The closure tells `split` where a word ends: at every sign that is no
    // letter or digit (quotes and operators of FTS5 among them).
    for word in question.split(|character: char| !character.is_alphanumeric()).take(MAX_WORDS * 4) {
        if !word.is_empty() && words.len() < MAX_WORDS && !STOP_WORDS.contains(&word.to_lowercase().as_str()) {
            words.push(format!("\"{word}\""));
        }
    }

    if words.is_empty() { None } else { Some(words.join(" OR ")) }
}

/// The passages of the sources of the knowledge bases `bases` that fit the
/// `question` best (BM25 of FTS5), at most `count` and at most
/// [`MAX_FOUND`], the best first.
///
/// Called by: nothing yet (the AI layer rewritten on `db/`).
/// Calls: [`query_of`].
pub fn read_passages(db: &UserDatabase, bases: &[i64], question: &str, count: usize) -> Result<Vec<String>, DbError> {
    ensure!(bases.len() <= MAX_BASES, DbError::Inconsistent(format!("more than {MAX_BASES} knowledge bases")));

    let Some(query) = query_of(question) else {
        return Ok(Vec::new());
    };
    let wanted = count.min(MAX_FOUND);
    let mut ids = Vec::with_capacity(bases.len());

    for base in bases {
        ids.push(base.to_string());
    }

    let bases = format!("[{}]", ids.join(","));
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT knowledge_passages.text AS text FROM knowledge_passages
         WHERE knowledge_passages MATCH :query
           AND source IN (SELECT id FROM knowledge_sources
                          WHERE knowledge_base IN (SELECT value FROM json_each(:bases)))
         ORDER BY bm25(knowledge_passages) LIMIT :wanted",
    )?;
    let mut rows = statement.query(named_params! {
        ":query": query,
        ":bases": bases,
        ":wanted": i64::try_from(wanted).map_err(|error| DbError::Inconsistent(error.to_string()))?,
    })?;
    let mut found = Vec::with_capacity(wanted);

    for _ in 0..wanted {
        let Some(row) = rows.next()? else { break };

        found.push(row.get("text")?);
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    //! Tests of the knowledge: bases and sources stored, read, changed; the
    //! cutting of a text into passages; the search of the passages.

    use super::*;

    use crate::test_support::database::database;

    fn base(name: &str) -> KnowledgeBase {
        KnowledgeBase { id: 0, name: name.to_owned(), description: String::new() }
    }

    fn source(base: i64, text: &str) -> Source {
        Source { id: 0, knowledge_base: base, name: "Router X, 2025".to_owned(), text: text.to_owned() }
    }

    #[test]
    fn a_base_and_its_sources_are_stored_read_and_changed() {
        let (_directory, db) = database();
        let id = create_knowledge_base(&db, &base("Earlier evaluations")).unwrap();
        let source_id = create_source(&db, &source(id, "The TSF protects the keys.")).unwrap();
        let mut stored = read_source(&db, source_id).unwrap();

        assert_eq!(read_knowledge_bases(&db).unwrap(), vec![KnowledgeBase { id, ..base("Earlier evaluations") }]);
        assert_eq!(stored, Source { id: source_id, ..source(id, "The TSF protects the keys.") });

        stored.text = "The firmware is signed.".to_owned();
        update_source(&db, &stored).unwrap();

        assert_eq!(read_sources(&db, id).unwrap(), vec![stored]);
        assert!(read_passages(&db, &[id], "keys", 8).unwrap().is_empty());
        assert_eq!(read_passages(&db, &[id], "signed firmware", 8).unwrap(), vec!["The firmware is signed."]);
    }

    #[test]
    fn a_text_is_cut_into_passages_of_its_limit() {
        let paragraph = "word ".repeat(400);
        let long_word = "x".repeat(PASSAGE_CHARS * 2 + 10);
        let cut = passages(&format!("{paragraph}\n\nShort one.\n\n{long_word}"));

        for passage in &cut {
            assert!(passage.chars().count() <= PASSAGE_CHARS, "{}", passage.chars().count());
        }

        assert_eq!(cut.concat().matches("word").count(), 400);
        assert_eq!(cut.concat().matches('x').count(), PASSAGE_CHARS * 2 + 10);
        assert!(passages("one\n\ntwo").first().is_some_and(|first| first == "one\ntwo"));
    }

    #[test]
    fn a_search_looks_only_in_the_bases_it_is_given() {
        let (_directory, db) = database();
        let one = create_knowledge_base(&db, &base("One")).unwrap();
        let two = create_knowledge_base(&db, &base("Two")).unwrap();

        create_source(&db, &source(one, "Generated keys are stored encrypted.")).unwrap();
        create_source(&db, &source(two, "The key of the door.")).unwrap();

        assert_eq!(read_passages(&db, &[one], "key", 8).unwrap(), vec!["Generated keys are stored encrypted."]);
        assert_eq!(read_passages(&db, &[one, two], "key", 8).unwrap().len(), 2);
        assert!(read_passages(&db, &[one], "\"OR NOT(", 8).unwrap().is_empty());
        // "What is the" are passed over: only "key" is looked for.
        assert_eq!(read_passages(&db, &[one, two], "What is the key?", 8).unwrap().len(), 2);
        assert!(read_passages(&db, &[one, two], "What is the", 8).unwrap().is_empty());
    }
}
