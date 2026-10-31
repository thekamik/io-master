//! The lists of a model kept in a table of their own: a list of ids (the
//! dependencies of a work unit, the standards of a plan) is one row per
//! element, with its position, since a cell of SQL holds one value. The
//! help every file of `db/` uses to write and read them, each read bounded
//! by the limit of its list (rule 2 of CLAUDE.md).

use rusqlite::{Connection, named_params};

use crate::db::error::DbError;

/// The position of the element `index` of a list, as a number of SQL.
///
/// Called by: the files of `db/` that write a list.
pub(in crate::db) fn position(index: usize) -> Result<i64, DbError> {
    i64::try_from(index).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// Refuses a list of `what` longer than `max`, before it is written.
///
/// Called by: the files of `db/` that write a list.
pub(in crate::db) fn check_length(length: usize, max: usize, what: &str) -> Result<(), DbError> {
    ensure!(length <= max, DbError::Inconsistent(format!("more than {max} {what}")));

    Ok(())
}

/// Reads a list of ids: `sql` selects the column `id` of the rows of the
/// owner `:owner`, in the order of their positions; more than `max` rows
/// mean a database the program did not write.
///
/// Called by: the files of `db/` that read a list.
pub(in crate::db) fn read_ids(connection: &Connection, sql: &str, owner: i64, max: usize) -> Result<Vec<i64>, DbError> {
    ensure!(sql.contains(":owner"), DbError::Inconsistent("a list read without its owner".to_owned()));

    let mut statement = connection.prepare(sql)?;
    let mut rows = statement.query(named_params! { ":owner": owner })?;
    let mut ids = Vec::new();

    for _ in 0..=max {
        let Some(row) = rows.next()? else {
            return Ok(ids);
        };

        ids.push(row.get("id")?);
    }

    Err(DbError::Inconsistent(format!("a list of more than {max} rows in the database")))
}

/// Writes a list of ids anew: `delete` removes the rows of the owner
/// `:owner`, `insert` writes one row of `:owner`, `:position` and `:id`.
/// Run inside the transaction of its caller.
///
/// Called by: the files of `db/` that write a list.
pub(in crate::db) fn write_ids(
    connection: &Connection,
    delete: &str,
    insert: &str,
    owner: i64,
    ids: &[i64],
) -> Result<(), DbError> {
    ensure!(insert.contains(":position"), DbError::Inconsistent("a list written without positions".to_owned()));

    connection.execute(delete, named_params! { ":owner": owner })?;

    for (index, id) in ids.iter().enumerate() {
        connection.execute(insert, named_params! { ":owner": owner, ":position": position(index)?, ":id": id })?;
    }

    Ok(())
}
