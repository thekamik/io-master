//! The custom formats of the AI services (`CustomFormat`) in the table
//! `ai_custom_formats` of the database of the user: create, read, update.
//! Both JSON texts are kept whole, as the user wrote them.
//!
//! ```text
//! ai_custom_formats   id, name (unique), request, response, response_kind, key_header, key_prefix
//! ```

use rusqlite::{Row, named_params};

use crate::infrastructure::storage::error::DbError;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::custom_format::custom_format_model::{CustomFormat, ResponseKind};

/// The most custom formats of the user.
pub const MAX_FORMATS: usize = 50;

/// The longest JSON of a request or of a response, in bytes.
pub const MAX_FORMAT_JSON: usize = 256 * 1024;

/// The longest name of the header of the key, and of what stands before the
/// key, in bytes.
pub const MAX_KEY_HEADER: usize = 100;

/// The table, made when it is missing (`objects/schema.rs`, `USER_TABLES`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_custom_formats (
        id            INTEGER PRIMARY KEY,
        name          TEXT NOT NULL UNIQUE,
        request       TEXT NOT NULL,
        response      TEXT NOT NULL,
        response_kind TEXT NOT NULL CHECK (response_kind IN ('whole', 'events')),
        key_header    TEXT NOT NULL,
        key_prefix    TEXT NOT NULL
    );
";

/// The columns every read of a format selects.
const COLUMNS: &str = "id, name, request, response, response_kind, key_header, key_prefix";

/// The name of a kind of response in the column `response_kind`
/// (`ResponseKind::Whole` → `"whole"`).
///
/// Called by: [`create_custom_format`], [`update_custom_format`].
fn kind_name(kind: ResponseKind) -> &'static str {
    match kind {
        ResponseKind::Whole => "whole",
        ResponseKind::Events => "events",
    }
}

/// The kind of response of its name in the column `response_kind`
/// (`"whole"` → `ResponseKind::Whole`); `DbError::Corrupt` for a name the
/// program never writes.
///
/// Called by: [`format_of`].
fn kind_of(name: &str) -> Result<ResponseKind, DbError> {
    match name {
        "whole" => Ok(ResponseKind::Whole),
        "events" => Ok(ResponseKind::Events),
        _ => Err(DbError::Corrupt { table: "ai_custom_formats", column: "response_kind", value: name.to_owned() }),
    }
}

/// Refuses a format without a name, without one of its JSON texts or the
/// header of its key, or with a text over its limit.
///
/// Input: `format`, the format to be stored.
/// Output: nothing when it is good; otherwise `DbError::Inconsistent`.
///
/// Called by: [`create_custom_format`], [`update_custom_format`].
fn check_format(format: &CustomFormat) -> Result<(), DbError> {
    ensure!(!format.name.is_empty(), DbError::Inconsistent("a custom format without a name".to_owned()));
    ensure!(
        !format.request.is_empty() && !format.response.is_empty(),
        DbError::Inconsistent("a custom format without its request or its response".to_owned())
    );
    ensure!(
        format.request.len() <= MAX_FORMAT_JSON && format.response.len() <= MAX_FORMAT_JSON,
        DbError::Inconsistent(format!("a JSON of a custom format over {MAX_FORMAT_JSON} bytes"))
    );
    ensure!(
        !format.key_header.is_empty()
            && format.key_header.len() <= MAX_KEY_HEADER
            && format.key_prefix.len() <= MAX_KEY_HEADER,
        DbError::Inconsistent("the header of the key of a custom format empty or too long".to_owned())
    );

    Ok(())
}

/// Stores a new custom format and gives its id.
///
/// Input: `db`, the open database of the user; `format`, the new format,
/// its `id` 0 (the database gives it).
/// Output: the id of the format (2). An error when it is refused
/// ([`check_format`]), when the user has [`MAX_FORMATS`] already, or when
/// its name is taken (`UNIQUE`).
///
/// Called by: `objects/ai/custom_format/custom_format_rules.rs`: `add_format`.
/// Calls: [`check_format`], [`kind_name`].
pub fn create_custom_format(db: &UserDatabase, format: &CustomFormat) -> Result<i64, DbError> {
    ensure!(format.id == 0, DbError::Inconsistent("a new custom format has an id already".to_owned()));
    check_format(format)?;

    let connection = db.connection();
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM ai_custom_formats", [], |row| row.get(0))?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_FORMATS,
        DbError::Inconsistent(format!("more than {MAX_FORMATS} custom formats"))
    );

    connection.execute(
        "INSERT INTO ai_custom_formats (name, request, response, response_kind, key_header, key_prefix)
         VALUES (:name, :request, :response, :response_kind, :key_header, :key_prefix)",
        named_params! {
            ":name": format.name,
            ":request": format.request,
            ":response": format.response,
            ":response_kind": kind_name(format.response_kind),
            ":key_header": format.key_header,
            ":key_prefix": format.key_prefix,
        },
    )?;

    Ok(connection.last_insert_rowid())
}

/// The model of one row of `ai_custom_formats`.
///
/// Called by: [`read_custom_format`], [`read_custom_formats`].
/// Calls: [`kind_of`].
fn format_of(row: &Row<'_>) -> Result<CustomFormat, DbError> {
    let kind: String = row.get("response_kind")?;

    Ok(CustomFormat {
        id: row.get("id")?,
        name: row.get("name")?,
        request: row.get("request")?,
        response: row.get("response")?,
        response_kind: kind_of(&kind)?,
        key_header: row.get("key_header")?,
        key_prefix: row.get("key_prefix")?,
    })
}

/// Reads the custom format `id`.
///
/// Input: `db`, the open database of the user; `id`, the id of the format.
/// Output: the format; `DbError::NoRow` when there is none.
///
/// Called by: `objects/ai/custom_format/custom_format_rules.rs`: `change_format`;
/// `objects/ai/integration/integration_rules/settings.rs`: `checked`;
/// `objects/ai/integration/integration_ai/question.rs`: `prepare`.
/// Calls: [`format_of`].
pub fn read_custom_format(db: &UserDatabase, id: i64) -> Result<CustomFormat, DbError> {
    let mut statement =
        db.connection().prepare_cached(&format!("SELECT {COLUMNS} FROM ai_custom_formats WHERE id = :id"))?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "ai_custom_formats", id });
    };

    format_of(row)
}

/// Reads every custom format, by their names.
///
/// Input: `db`, the open database of the user.
/// Output: the formats, ordered by name; an error when the table holds more
/// than [`MAX_FORMATS`] (a database the program did not write).
///
/// Called by: `panels/views/settings/custom_formats.rs`: `custom_format_list`.
/// Calls: [`format_of`].
pub fn read_custom_formats(db: &UserDatabase) -> Result<Vec<CustomFormat>, DbError> {
    let mut statement = db.connection().prepare(&format!("SELECT {COLUMNS} FROM ai_custom_formats ORDER BY name"))?;
    let mut rows = statement.query([])?;
    let mut formats = Vec::new();

    for _ in 0..=MAX_FORMATS {
        let Some(row) = rows.next()? else {
            return Ok(formats);
        };

        formats.push(format_of(row)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_FORMATS} custom formats in the database")))
}

/// Writes the custom format `format.id` anew.
///
/// Input: `db`, the open database of the user; `format`, the format as it
/// is to be kept, its `id` that of a stored one.
/// Output: nothing; an error as [`create_custom_format`], or
/// `DbError::NoRow` when there is no format `format.id`.
///
/// Called by: `objects/ai/custom_format/custom_format_rules.rs`: `change_format`.
/// Calls: [`check_format`], [`kind_name`].
pub fn update_custom_format(db: &UserDatabase, format: &CustomFormat) -> Result<(), DbError> {
    check_format(format)?;

    let changed = db.connection().execute(
        "UPDATE ai_custom_formats SET name = :name, request = :request, response = :response,
                response_kind = :response_kind, key_header = :key_header, key_prefix = :key_prefix
         WHERE id = :id",
        named_params! {
            ":id": format.id,
            ":name": format.name,
            ":request": format.request,
            ":response": format.response,
            ":response_kind": kind_name(format.response_kind),
            ":key_header": format.key_header,
            ":key_prefix": format.key_prefix,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "ai_custom_formats", id: format.id });

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the custom formats: stored, read and changed whole.

    use super::*;

    use crate::test_support::database::{custom_format, database};

    #[test]
    fn a_format_is_stored_read_and_changed_with_its_json_whole() {
        let (_directory, db) = database();
        let id = create_custom_format(&db, &custom_format()).unwrap();
        let mut stored = read_custom_format(&db, id).unwrap();

        assert_eq!(stored, CustomFormat { id, ..custom_format() });

        stored.response_kind = ResponseKind::Events;
        stored.request = format!("{{\"padding\": \"{}\", \"q\": \"<prompt>\"}}", "x".repeat(100_000));
        update_custom_format(&db, &stored).unwrap();

        assert_eq!(read_custom_formats(&db).unwrap(), vec![stored.clone()]);
        assert!(matches!(create_custom_format(&db, &custom_format()), Err(DbError::Sql(_))));
        assert!(matches!(read_custom_format(&db, 99), Err(DbError::NoRow { .. })));
        assert!(matches!(
            update_custom_format(&db, &CustomFormat { request: String::new(), ..stored }),
            Err(DbError::Inconsistent(_))
        ));
    }
}
