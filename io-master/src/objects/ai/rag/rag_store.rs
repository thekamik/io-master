//! The knowledge for the AI (RAG): the knowledge bases (`KnowledgeBase`)
//! and their sources (`Source`), in the tables `knowledge_bases` and
//! `knowledge_sources` of the database of the user: create, read, update;
//! never deleted (user, 2026-10-28). With them the index of the passages,
//! which is no model: whenever the text of a source is written, it is cut
//! into passages of at most the `passage_characters` of its base and indexed
//! by FTS5 of SQLite anew (all the sources of a base when that length
//! changes), and [`read_passages`] finds those that fit a question, passing
//! over the stop words of the bases searched.
//!
//! ```text
//! knowledge_bases            id, name (unique), description, passage_characters
//! knowledge_base_stop_words  (base, position, word)   stop_words
//! knowledge_sources          id, knowledge_base, name, text
//! knowledge_passages   the index (FTS5): text, source; made from the text of the sources
//! ```

use rusqlite::{Connection, Row, named_params};

use crate::infrastructure::storage::error::DbError;
use crate::infrastructure::storage::lists::{check_length, position};
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::rag::rag_model::{KnowledgeBase, Source};

/// The most knowledge bases, and sources of one base.
pub const MAX_BASES: usize = 50;
pub const MAX_SOURCES: usize = 200;

/// The longest passage of a document of the product, and where a new
/// knowledge base starts (`objects/ai/rag/rag_rules.rs`: `add_base`), in
/// characters.
pub const PASSAGE_CHARS: usize = 1500;

/// The shortest and the longest passage of a knowledge base, in characters:
/// a shorter one holds no sentence whole, a longer one fills the knowledge of
/// an agent with one passage.
pub const MIN_PASSAGE_CHARS: u32 = 200;
pub const MAX_PASSAGE_CHARS: u32 = 10_000;

/// The most stop words of a base, and the longest one, in characters.
pub const MAX_STOP_WORDS: usize = 1000;
pub const MAX_STOP_WORD: usize = 100;

/// The longest text of a source, in characters (a standard of 500 pages
/// is about 1.5 million).
pub const MAX_SOURCE_CHARS: usize = 20_000_000;

/// The most passages a search gives.
pub const MAX_FOUND: usize = 50;

/// The most words of a question a search looks for: a bound of the memory
/// (rule 3), far above the words of a requirement with its guidance and the
/// terms of a glossary, so that no word that matters is cut off.
const MAX_WORDS: usize = 4000;

/// The words of English a search of the documents of the product passes
/// over, and where the stop words of a new knowledge base start (user,
/// 2026-10-29): they are in almost every passage, so a question that names
/// them ("What is the TSF?") would find passages by "is" and "the" instead of
/// by "TSF".
pub const STOP_WORDS: [&str; 64] = [
    "a", "about", "all", "also", "an", "and", "any", "are", "as", "at", "be", "been", "but", "by", "can", "do", "does",
    "each", "for", "from", "has", "have", "he", "her", "his", "how", "i", "if", "in", "into", "is", "it", "its", "me",
    "my", "no", "not", "of", "on", "or", "other", "our", "shall", "she", "should", "so", "such", "that", "the",
    "their", "them", "then", "there", "these", "they", "this", "to", "was", "we", "what", "when", "which", "who",
    "with",
];

/// The tables, made when they are missing (`objects/schema.rs`, `USER_TABLES`). The tokenizer
/// `porter` finds an English word by its stem ("keys" finds "key"),
/// `unicode61` folds the case and the diacritics.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS knowledge_bases (
        id                 INTEGER PRIMARY KEY,
        name               TEXT NOT NULL UNIQUE,
        description        TEXT NOT NULL,
        passage_characters INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS knowledge_base_stop_words (
        base     INTEGER NOT NULL REFERENCES knowledge_bases(id),
        position INTEGER NOT NULL,
        word     TEXT NOT NULL,
        PRIMARY KEY (base, position)
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

/// Refuses a base without a name, with a passage out of
/// [`MIN_PASSAGE_CHARS`] to [`MAX_PASSAGE_CHARS`], or with stop words over
/// their limits or empty.
///
/// Input: `base`, the base to be stored.
/// Output: nothing when it is good; otherwise `DbError::Inconsistent`.
///
/// Called by: [`create_knowledge_base`], [`update_knowledge_base`].
/// Calls: `infrastructure/storage/lists.rs`: `check_length`.
fn check_base(base: &KnowledgeBase) -> Result<(), DbError> {
    ensure!(!base.name.is_empty(), DbError::Inconsistent("a knowledge base without a name".to_owned()));
    ensure!(
        (MIN_PASSAGE_CHARS..=MAX_PASSAGE_CHARS).contains(&base.passage_characters),
        DbError::Inconsistent(format!("a passage out of {MIN_PASSAGE_CHARS} to {MAX_PASSAGE_CHARS} characters"))
    );
    check_length(base.stop_words.len(), MAX_STOP_WORDS, "stop words of a knowledge base")?;

    for word in &base.stop_words {
        ensure!(
            !word.is_empty() && word.chars().count() <= MAX_STOP_WORD,
            DbError::Inconsistent(format!("a stop word empty or over {MAX_STOP_WORD} characters"))
        );
    }

    Ok(())
}

/// Writes the stop words of the base `id` anew, in their order. Run inside
/// the transaction of its caller.
///
/// Input: `connection`, the open database inside a transaction; `id`, the id
/// of the base; `words`, its stop words, checked by [`check_base`].
/// Output: nothing; an error of SQLite.
///
/// Called by: [`create_knowledge_base`], [`update_knowledge_base`].
/// Calls: `infrastructure/storage/lists.rs`: `position`.
fn write_stop_words(connection: &Connection, id: i64, words: &[String]) -> Result<(), DbError> {
    connection.execute("DELETE FROM knowledge_base_stop_words WHERE base = :id", named_params! { ":id": id })?;

    for (index, word) in words.iter().enumerate() {
        connection.execute(
            "INSERT INTO knowledge_base_stop_words (base, position, word) VALUES (:id, :position, :word)",
            named_params! { ":id": id, ":position": position(index)?, ":word": word },
        )?;
    }

    Ok(())
}

/// Stores a new knowledge base (a folder of sources, shared by the agents
/// that name it) with its stop words, in one transaction, and gives its id.
///
/// Input: `db`, the open database of the user; `base`, the new base, its `id`
/// 0 (the database gives it), its name ("CEM 3.1"), description, length of a
/// passage and stop words.
///
/// Output: the id of the base (2). An error when it is refused
/// ([`check_base`]), when the user has [`MAX_BASES`] already, or when its name
/// is taken (`UNIQUE`); nothing is then written.
///
/// Called by: `objects/ai/rag/rag_rules.rs`: `add_base`.
/// Calls: [`check_base`], [`write_stop_words`].
pub fn create_knowledge_base(db: &UserDatabase, base: &KnowledgeBase) -> Result<i64, DbError> {
    ensure!(base.id == 0, DbError::Inconsistent("a new knowledge base has an id already".to_owned()));
    check_base(base)?;

    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 = transaction.query_row("SELECT COUNT(*) FROM knowledge_bases", [], |row| row.get(0))?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_BASES,
        DbError::Inconsistent(format!("more than {MAX_BASES} knowledge bases"))
    );

    transaction.execute(
        "INSERT INTO knowledge_bases (name, description, passage_characters)
         VALUES (:name, :description, :passage_characters)",
        named_params! {
            ":name": base.name,
            ":description": base.description,
            ":passage_characters": base.passage_characters,
        },
    )?;

    let id = transaction.last_insert_rowid();

    write_stop_words(&transaction, id, &base.stop_words)?;
    transaction.commit()?;

    Ok(id)
}

/// Reads the stop words of the base `id`, in their order.
///
/// Input: `connection`, the open database; `id`, the id of the base.
/// Output: the words (`["the", "shall"]`); an error when there are more than
/// [`MAX_STOP_WORDS`] (a database the program did not write).
///
/// Called by: [`base_of`].
fn read_stop_words(connection: &Connection, id: i64) -> Result<Vec<String>, DbError> {
    let mut statement =
        connection.prepare("SELECT word FROM knowledge_base_stop_words WHERE base = :id ORDER BY position")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let mut words = Vec::new();

    for _ in 0..=MAX_STOP_WORDS {
        let Some(row) = rows.next()? else {
            return Ok(words);
        };

        words.push(row.get("word")?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_STOP_WORDS} stop words of a base in the database")))
}

/// The model of one row of `knowledge_bases`, with its stop words.
///
/// Called by: [`read_knowledge_base`], [`read_knowledge_bases`].
/// Calls: [`read_stop_words`].
fn base_of(connection: &Connection, row: &Row<'_>) -> Result<KnowledgeBase, DbError> {
    let id = row.get("id")?;

    Ok(KnowledgeBase {
        id,
        name: row.get("name")?,
        description: row.get("description")?,
        passage_characters: row.get("passage_characters")?,
        stop_words: read_stop_words(connection, id)?,
    })
}

/// Reads the knowledge base `id`.
///
/// Input: `db`, the open database of the user; `id`, the id of the base (2).
///
/// Output: the base with its stop words; `DbError::NoRow` when there is none.
///
/// Called by: `objects/ai/agent/agent_rules.rs`: `checked`;
/// `objects/ai/rag/rag_rules.rs`: `add_source`, `change_base`,
/// `search_base`.
/// Calls: [`base_of`].
pub fn read_knowledge_base(db: &UserDatabase, id: i64) -> Result<KnowledgeBase, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT id, name, description, passage_characters FROM knowledge_bases WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "knowledge_bases", id });
    };

    base_of(connection, row)
}

/// Reads every knowledge base, by their names.
///
/// Input: `db`, the open database of the user.
///
/// Output: the bases, ordered by name; an error when the table holds more
/// than [`MAX_BASES`] (a database the program did not write).
///
/// Called by: `objects/ai/rag/rag_rules.rs`: `add_base`;
/// `panels/views/agents/agent.rs`: `agent_get`;
/// `panels/views/agents/knowledge.rs`: `knowledge_base_list`.
/// Calls: [`base_of`].
pub fn read_knowledge_bases(db: &UserDatabase) -> Result<Vec<KnowledgeBase>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT id, name, description, passage_characters FROM knowledge_bases ORDER BY name")?;
    let mut rows = statement.query([])?;
    let mut bases = Vec::new();

    for _ in 0..=MAX_BASES {
        let Some(row) = rows.next()? else {
            return Ok(bases);
        };

        bases.push(base_of(connection, row)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_BASES} knowledge bases in the database")))
}

/// Makes the index of every source of the base `base` anew, its passages of
/// at most `limit` characters: the length of a passage of the base changed.
/// Run inside the transaction of its caller.
///
/// Input: `connection`, the open database inside a transaction; `base`, the
/// id of a stored base; `limit`, its new length of a passage.
/// Output: nothing; an error of SQLite, or when the base holds more than
/// [`MAX_SOURCES`] sources.
///
/// Called by: [`update_knowledge_base`].
/// Calls: [`write_passages`].
fn index_base(connection: &Connection, base: i64, limit: usize) -> Result<(), DbError> {
    let mut statement = connection.prepare("SELECT id, text FROM knowledge_sources WHERE knowledge_base = :base")?;
    let mut rows = statement.query(named_params! { ":base": base })?;

    for _ in 0..=MAX_SOURCES {
        let Some(row) = rows.next()? else {
            return Ok(());
        };
        let text: String = row.get("text")?;

        write_passages(connection, row.get("id")?, &text, limit)?;
    }

    Err(DbError::Inconsistent(format!("more than {MAX_SOURCES} sources of a knowledge base in the database")))
}

/// Writes the knowledge base `base.id` anew: its name, description, length
/// of a passage and stop words, in one transaction; its sources indexed anew
/// when the length of a passage changed.
///
/// Input: `db`, the open database of the user; `base`, the base as it is to
/// be kept, its `id` that of a stored one.
///
/// Output: nothing; an error as [`create_knowledge_base`], or
/// `DbError::NoRow` when there is no base `base.id`; nothing is then written.
///
/// Called by: `objects/ai/rag/rag_rules.rs`: `change_base`.
/// Calls: [`check_base`], [`passage_limit`], [`write_stop_words`], [`index_base`].
pub fn update_knowledge_base(db: &UserDatabase, base: &KnowledgeBase) -> Result<(), DbError> {
    check_base(base)?;

    let transaction = db.connection().unchecked_transaction()?;
    let before = passage_limit(&transaction, base.id)?;
    let changed = transaction.execute(
        "UPDATE knowledge_bases SET name = :name, description = :description,
                passage_characters = :passage_characters
         WHERE id = :id",
        named_params! {
            ":id": base.id,
            ":name": base.name,
            ":description": base.description,
            ":passage_characters": base.passage_characters,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "knowledge_bases", id: base.id });
    write_stop_words(&transaction, base.id, &base.stop_words)?;

    let after = usize::try_from(base.passage_characters).map_err(|error| DbError::Inconsistent(error.to_string()))?;

    // The index is made from the text by the length of a passage: a new
    // length makes the passages of every source of the base anew.
    if after != before {
        index_base(&transaction, base.id, after)?;
    }

    transaction.commit()?;

    Ok(())
}

/// The length of a passage of the base `base`, as the index cuts its
/// sources.
///
/// Input: `connection`, the open database; `base`, the id of a base.
/// Output: the length in characters (`1500`); `DbError::NoRow` when there is
/// no base `base`.
///
/// Called by: [`update_knowledge_base`], [`create_source`], [`update_source`].
fn passage_limit(connection: &Connection, base: i64) -> Result<usize, DbError> {
    let mut statement = connection.prepare("SELECT passage_characters FROM knowledge_bases WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": base })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "knowledge_bases", id: base });
    };
    let limit: u32 = row.get("passage_characters")?;

    usize::try_from(limit).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// Cuts a text into passages of at most `limit` characters, for the
/// index: whole paragraphs (parted by an empty line) joined while they fit,
/// a longer paragraph cut between words, a word longer than a passage cut
/// where it reaches the limit. The spaces between the words become one.
///
/// Input: `text`, the text of a source or of a document of the product, its
/// length checked by its caller; `limit`, the longest passage (the
/// `passage_characters` of a base, or [`PASSAGE_CHARS`] for a document of
/// the product), at least 1.
///
/// Output: the passages in the order of the text (a text of 4000 characters
/// in paragraphs of 1000 → `["<paragraphs 1 and 2>", "<paragraph 3>",
/// "<paragraph 4>"]`); none for an empty text.
///
/// Called by: [`write_passages`]; `objects/evaluation/product_document/product_document_store.rs`:
/// `write_passages` (the index of a document of the product).
/// Calls: [`push_passage`].
pub(in crate::objects) fn passages(text: &str, limit: usize) -> Vec<String> {
    let mut passages = Vec::new();
    let mut current = String::new();

    for paragraph in text.split("\n\n") {
        // A new paragraph starts on a new line of the passage being made.
        if !current.is_empty() && !current.ends_with('\n') {
            current.push('\n');
        }

        for word in paragraph.split_whitespace() {
            let mut rest: Vec<char> = word.chars().collect();

            // Each turn places the word, closes the passage, or takes a
            // passage of a word longer than one: at most one turn a character.
            for _ in 0..=rest.len() {
                let room = limit.saturating_sub(current.chars().count() + 1);

                // The word fits the passage being made (or starts an empty
                // one): it is added, after a space.
                if rest.len() <= room || (current.is_empty() && rest.len() <= limit) {
                    if !current.is_empty() && !current.ends_with('\n') {
                        current.push(' ');
                    }

                    current.extend(rest.drain(..));
                    break;
                }

                // It does not fit: the passage is closed, and the word tried
                // again in an empty one.
                if !current.is_empty() {
                    push_passage(&mut passages, &mut current);
                    continue;
                }

                // A word longer than a whole passage: its first part is a
                // passage of its own, the rest tried again.
                current.extend(rest.drain(..limit.min(rest.len())));
                push_passage(&mut passages, &mut current);
            }
        }
    }

    // The last passage, still open when the text ends.
    push_passage(&mut passages, &mut current);

    passages
}

/// Closes the passage being made: adds it to `passages` without the spaces
/// around it, when it holds anything, and empties it for the next.
///
/// Input: `passages`, the passages closed so far; `current`, the passage
/// being made.
///
/// Output: nothing; `passages` holds one more passage unless `current` was
/// empty, and `current` is empty.
///
/// Called by: [`passages`].
fn push_passage(passages: &mut Vec<String>, current: &mut String) {
    let passage = current.trim();

    if !passage.is_empty() {
        passages.push(passage.to_owned());
    }

    current.clear();
}

/// Makes the index of the source `id` anew from its text: its old passages
/// removed, the new ones inserted. Run inside the transaction of its caller;
/// the rows of the index are no data of their own, made again from the
/// text, so their replacement is no deletion of a model.
///
/// Input: `connection`, the open database inside a transaction; `id`, the id
/// of the source; `text`, its text; `limit`, the length of a passage of its
/// base.
///
/// Output: nothing; an error of SQLite when a statement fails.
///
/// Called by: [`create_source`], [`update_source`], [`index_base`].
/// Calls: [`passages`].
fn write_passages(connection: &Connection, id: i64, text: &str, limit: usize) -> Result<(), DbError> {
    connection.execute("DELETE FROM knowledge_passages WHERE source = :id", named_params! { ":id": id })?;

    for passage in passages(text, limit) {
        connection.execute(
            "INSERT INTO knowledge_passages (text, source) VALUES (:text, :id)",
            named_params! { ":text": passage, ":id": id },
        )?;
    }

    Ok(())
}

/// Refuses a source without a name, or with a text longer than
/// [`MAX_SOURCE_CHARS`].
///
/// Input: `source`, the source to be stored.
///
/// Output: nothing when it is good; otherwise `DbError::Inconsistent`.
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

/// Stores a new source of a knowledge base (one text: an earlier
/// evaluation, a note, a standard) with the index of its passages, in one
/// transaction, and gives its id.
///
/// Input: `db`, the open database of the user; `source`, the new source, its
/// `id` 0 (the database gives it), its knowledge base, its name and its text.
///
/// Output: the id of the source (9). An error when it is refused
/// ([`check_source`]), when its base has [`MAX_SOURCES`] already, or when the
/// base is not stored (foreign key); nothing is then written.
///
/// Called by: `objects/ai/rag/rag_rules.rs`: `add_source`.
/// Calls: [`check_source`], [`passage_limit`], [`write_passages`].
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
    let limit = passage_limit(&transaction, source.knowledge_base)?;

    write_passages(&transaction, id, &source.text, limit)?;
    transaction.commit()?;

    Ok(id)
}

/// The model of one row of `knowledge_sources`.
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
/// Input: `db`, the open database of the user; `id`, the id of the source (9).
///
/// Output: the source; `DbError::NoRow` when there is none.
///
/// Called by: `objects/ai/rag/rag_rules.rs`: `rename_source`,
/// `replace_source_text`.
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

/// Reads every source of a knowledge base, with their texts, by their
/// names.
///
/// Input: `db`, the open database of the user; `base`, the id of the
/// knowledge base.
///
/// Output: the sources, ordered by name; an error when the base has more
/// than [`MAX_SOURCES`] (a database the program did not write).
///
/// Called by: `objects/ai/rag/rag_rules.rs`: `add_source`;
/// `panels/views/agents/knowledge.rs`: `knowledge_base_list`,
/// `knowledge_source_list`.
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

/// Writes the source `source.id` anew (its base, its name, its text) with
/// the index of its passages, in one transaction.
///
/// Input: `db`, the open database of the user; `source`, the source as it is
/// to be kept, its `id` that of a stored one.
///
/// Output: nothing; an error as [`create_source`], or `DbError::NoRow` when
/// there is no source `source.id`; nothing is then written.
///
/// Called by: `objects/ai/rag/rag_rules.rs`: `rename_source`,
/// `replace_source_text`.
/// Calls: [`check_source`], [`passage_limit`], [`write_passages`].
pub fn update_source(db: &UserDatabase, source: &Source) -> Result<(), DbError> {
    check_source(source)?;

    let transaction = db.connection().unchecked_transaction()?;
    let changed = transaction.execute(
        "UPDATE knowledge_sources SET knowledge_base = :base, name = :name, text = :text WHERE id = :id",
        named_params! { ":id": source.id, ":base": source.knowledge_base, ":name": source.name, ":text": source.text },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "knowledge_sources", id: source.id });

    let limit = passage_limit(&transaction, source.knowledge_base)?;

    write_passages(&transaction, source.id, &source.text, limit)?;
    transaction.commit()?;

    Ok(())
}

/// Whether `word` (in lower case) is one of `stop_words`.
///
/// Called by: [`query_of`].
fn is_stop_word<S: AsRef<str>>(word: &str, stop_words: &[S]) -> bool {
    for stop in stop_words {
        if stop.as_ref() == word {
            return true;
        }
    }

    false
}

/// The stop words of the bases `bases` together: a word one of them passes
/// over is passed over in a search of them all.
///
/// Input: `connection`, the open database; `bases`, the ids of the bases as
/// one JSON array ("[2,5]"), at most `MAX_BASES` of them.
/// Output: the words, each once (`["the", "shall"]`); an error of SQLite, or
/// when there are more than the bases can hold.
///
/// Called by: [`read_passages`].
fn stop_words_of(connection: &Connection, bases: &str) -> Result<Vec<String>, DbError> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT word FROM knowledge_base_stop_words WHERE base IN (SELECT value FROM json_each(:bases))",
    )?;
    let mut rows = statement.query(named_params! { ":bases": bases })?;
    let mut words = Vec::new();

    for _ in 0..=MAX_BASES * MAX_STOP_WORDS {
        let Some(row) = rows.next()? else {
            return Ok(words);
        };

        words.push(row.get("word")?);
    }

    Err(DbError::Inconsistent("more stop words than the knowledge bases hold".to_owned()))
}

/// The query of FTS5 for a question: its words but the stop words
/// `stop_words`, each quoted (so that no word is read as an operator of
/// FTS5), any of them.
///
/// Input: `question`, the text to look for ("What is the TSF of the TOE?");
/// `stop_words`, the words passed over, in lower case (those of the bases
/// searched, or [`STOP_WORDS`] for the documents of the product).
///
/// Output: the query (`"\"TSF\" OR \"TOE\""`), at most `MAX_WORDS` words;
/// `None` for a question without such words.
///
/// Called by: [`read_passages`]; `objects/evaluation/product_document/product_document_store.rs`:
/// `read_document_passages` (the search in the documents of the product).
pub(in crate::objects) fn query_of<S: AsRef<str>>(question: &str, stop_words: &[S]) -> Option<String> {
    let mut words = Vec::new();

    // The closure tells `split` where a word ends: at every sign that is no
    // letter or digit (quotes and operators of FTS5 among them).
    for word in question.split(|character: char| !character.is_alphanumeric()).take(MAX_WORDS * 4) {
        let lower = word.to_lowercase();

        if !word.is_empty() && words.len() < MAX_WORDS && !is_stop_word(&lower, stop_words) {
            words.push(format!("\"{word}\""));
        }
    }

    if words.is_empty() { None } else { Some(words.join(" OR ")) }
}

/// Finds the passages of the knowledge bases of an agent that fit a
/// question best (BM25 of FTS5), the best first, each with the name of its
/// source, so that the model and the user know where a passage comes from;
/// the stop words of the bases are passed over.
///
/// Input:
/// - `db`: the open database of the user.
/// - `bases`: the ids of the knowledge bases to search, at most
///   [`MAX_BASES`].
/// - `question`: the text to look for.
/// - `count`: the most passages wanted (the limit of the agent).
///
/// Output: at most `count` and at most [`MAX_FOUND`] passages with their
/// sources (`[("CC interpretation 12.pdf", "The TSF shall ...")]`); none when
/// the question has no word to look for. An error when there are too many
/// bases.
///
/// Called by: `objects/ai/agent/agent_ai.rs`: `agent_knowledge`;
/// `objects/ai/rag/rag_rules.rs`: `search_base`.
/// Calls: [`stop_words_of`], [`query_of`].
pub fn read_passages(
    db: &UserDatabase,
    bases: &[i64],
    question: &str,
    count: usize,
) -> Result<Vec<(String, String)>, DbError> {
    ensure!(bases.len() <= MAX_BASES, DbError::Inconsistent(format!("more than {MAX_BASES} knowledge bases")));

    // The bases are given to SQL as one JSON array ("[2,5]"), read by
    // json_each: a list of any length in one named parameter.
    let mut ids = Vec::with_capacity(bases.len());

    for base in bases {
        ids.push(base.to_string());
    }

    let bases = format!("[{}]", ids.join(","));
    let connection = db.connection();
    let Some(query) = query_of(question, &stop_words_of(connection, &bases)?) else {
        return Ok(Vec::new());
    };
    let wanted = count.min(MAX_FOUND);
    let mut statement = connection.prepare(
        "SELECT knowledge_sources.name AS source, knowledge_passages.text AS text
         FROM knowledge_passages JOIN knowledge_sources ON knowledge_sources.id = knowledge_passages.source
         WHERE knowledge_passages MATCH :query
           AND knowledge_sources.knowledge_base IN (SELECT value FROM json_each(:bases))
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

        found.push((row.get("source")?, row.get("text")?));
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    //! Tests of the knowledge: bases and sources stored, read, changed; the
    //! cutting of a text into passages; the search of the passages.

    use super::*;

    use crate::test_support::database::{database, knowledge_base};

    fn source(base: i64, text: &str) -> Source {
        Source { id: 0, knowledge_base: base, name: "Router X, 2025".to_owned(), text: text.to_owned() }
    }

    #[test]
    fn a_base_and_its_sources_are_stored_read_and_changed() {
        let (_directory, db) = database();
        let id = create_knowledge_base(&db, &knowledge_base("Earlier evaluations")).unwrap();
        let source_id = create_source(&db, &source(id, "The TSF protects the keys.")).unwrap();
        let mut stored = read_source(&db, source_id).unwrap();

        assert_eq!(
            read_knowledge_bases(&db).unwrap(),
            vec![KnowledgeBase { id, ..knowledge_base("Earlier evaluations") }]
        );
        assert_eq!(stored, Source { id: source_id, ..source(id, "The TSF protects the keys.") });

        stored.text = "The firmware is signed.".to_owned();
        update_source(&db, &stored).unwrap();

        assert_eq!(read_sources(&db, id).unwrap(), vec![stored]);
        assert!(read_passages(&db, &[id], "keys", 8).unwrap().is_empty());
        assert_eq!(
            read_passages(&db, &[id], "signed firmware", 8).unwrap(),
            vec![("Router X, 2025".to_owned(), "The firmware is signed.".to_owned())]
        );
    }

    #[test]
    fn a_base_searches_by_its_stop_words_and_is_indexed_anew_by_its_passage() {
        let (_directory, db) = database();
        let id = create_knowledge_base(&db, &knowledge_base("Notes")).unwrap();
        let paragraph = "The firmware is signed. ".repeat(20);

        create_source(&db, &source(id, &format!("{paragraph}\n\n{paragraph}"))).unwrap();

        let mut stored = read_knowledge_base(&db, id).unwrap();

        // "signed" found; "the" passed over by the stop words of the program.
        assert_eq!(read_passages(&db, &[id], "signed", 8).unwrap().len(), 1);
        assert!(read_passages(&db, &[id], "the", 8).unwrap().is_empty());

        // "signed" made a stop word, "the" no more; the passages cut shorter.
        stored.stop_words = vec!["signed".to_owned()];
        stored.passage_characters = MIN_PASSAGE_CHARS;
        update_knowledge_base(&db, &stored).unwrap();

        assert!(read_passages(&db, &[id], "signed", 8).unwrap().is_empty());
        assert!(read_passages(&db, &[id], "the", 8).unwrap().len() > 1);
        assert_eq!(read_knowledge_base(&db, id).unwrap(), stored);
        assert!(matches!(
            update_knowledge_base(&db, &KnowledgeBase { passage_characters: MAX_PASSAGE_CHARS + 1, ..stored }),
            Err(DbError::Inconsistent(_))
        ));
    }

    #[test]
    fn a_text_is_cut_into_passages_of_its_limit() {
        let paragraph = "word ".repeat(400);
        let long_word = "x".repeat(PASSAGE_CHARS * 2 + 10);
        let cut = passages(&format!("{paragraph}\n\nShort one.\n\n{long_word}"), PASSAGE_CHARS);

        for passage in &cut {
            assert!(passage.chars().count() <= PASSAGE_CHARS, "{}", passage.chars().count());
        }

        assert_eq!(cut.concat().matches("word").count(), 400);
        assert_eq!(cut.concat().matches('x').count(), PASSAGE_CHARS * 2 + 10);
        assert!(passages("one\n\ntwo", PASSAGE_CHARS).first().is_some_and(|first| first == "one\ntwo"));
    }

    #[test]
    fn a_search_looks_only_in_the_bases_it_is_given() {
        let (_directory, db) = database();
        let one = create_knowledge_base(&db, &knowledge_base("One")).unwrap();
        let two = create_knowledge_base(&db, &knowledge_base("Two")).unwrap();

        create_source(&db, &source(one, "Generated keys are stored encrypted.")).unwrap();
        create_source(&db, &source(two, "The key of the door.")).unwrap();

        assert_eq!(read_passages(&db, &[one], "key", 8).unwrap()[0].1, "Generated keys are stored encrypted.");
        assert_eq!(read_passages(&db, &[one, two], "key", 8).unwrap().len(), 2);
        assert!(read_passages(&db, &[one], "\"OR NOT(", 8).unwrap().is_empty());
        // "What is the" are passed over: only "key" is looked for.
        assert_eq!(read_passages(&db, &[one, two], "What is the key?", 8).unwrap().len(), 2);
        assert!(read_passages(&db, &[one, two], "What is the", 8).unwrap().is_empty());
    }
}
