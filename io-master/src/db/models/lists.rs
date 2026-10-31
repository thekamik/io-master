//! The lists of ids of a model, kept in a table of their own.
//!
//! A cell of SQL holds one value, so a field like `WorkUnit::dependencies`
//! (`Vec<i64>`) cannot be one column. Each such list has a table of its own,
//! one row per element, with its position, so that the list comes back in
//! the same order:
//!
//! ```text
//! WorkUnit { id: 7, dependencies: [12, 15] }
//!
//! work_unit_dependencies
//!  work_unit │ position │ depends_on
//!      7     │    0     │    12          write_id_list: the rows of 7 removed, the list inserted anew
//!      7     │    1     │    15          read_id_list:  the rows of 7 read back into [12, 15]
//! ```
//!
//! The file of a model knows which table and which columns hold its list;
//! this file knows how a list becomes rows and rows become a list again,
//! the same for every list:
//!
//! | Model | Field | Table |
//! |---|---|---|
//! | `Agent` | `knowledge_bases` | `ai_agent_knowledge_bases` |
//! | `EvaluationPlan` | `oscal_documents` | `evaluation_plan_standards` |
//! | `EvaluationPlan` | `product_documents` | `evaluation_plan_documents` |
//! | `WorkUnit` | `dependencies` | `work_unit_dependencies` |
//! | `WorkUnit` | `input_product_documents` | `work_unit_documents` |
//! | `WorkUnit` | `input_functional` | `work_unit_proofs` |
//!
//! Only the objects' ids are read and written here, never the objects they
//! name: the work units 12 and 15 are read by `read_work_unit`, when a
//! caller needs them. The lists of values (the glossary of an agent, the
//! verdicts of a plan, the quotes of a work unit) have more than one column
//! each; their files write them with their own SQL and take from here only
//! [`position`] and [`check_length`].

use rusqlite::{Connection, named_params};

use crate::db::error::DbError;

/// The position of an element of a list, as a number of SQL.
///
/// Input: `index`, the place of the element in its `Vec` (0 for the first).
///
/// Output: the same number as `i64` (`2` → `2`); an error for an index SQL
/// cannot hold, which no list within its limit reaches.
///
/// Called by: `ai/agent.rs`: `write_lists`; `evaluation/evaluation_plan.rs`:
/// `write_lists`, `order_work_units`; `evaluation/work_unit.rs`:
/// `write_lists`; this file: [`write_id_list`].
pub(in crate::db) fn position(index: usize) -> Result<i64, DbError> {
    i64::try_from(index).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// Refuses a list longer than its limit, before it is written.
///
/// Input:
/// - `length`: the number of elements of the list (`unit.dependencies.len()`).
/// - `max`: the most elements the list may hold (`MAX_DEPENDENCIES`).
/// - `what`: the name of the list, for the error ("dependencies of a work unit").
///
/// Output: nothing when the list fits; otherwise the error "more than
/// `max` `what`".
///
/// Called by: `ai/agent.rs`: `check_agent`; `evaluation/evaluation_plan.rs`:
/// `check_plan`; `evaluation/work_unit.rs`: `check_lists`,
/// `write_dependencies`.
pub(in crate::db) fn check_length(length: usize, max: usize, what: &str) -> Result<(), DbError> {
    ensure!(length <= max, DbError::Inconsistent(format!("more than {max} {what}")));

    Ok(())
}

/// Reads a list of ids that belongs to one object, for example the
/// dependencies of a work unit.
///
/// Input:
/// - `connection`: the open database.
/// - `sql`: a SELECT of one column named `id`, of the rows of the owner
///   `:owner`: `SELECT depends_on AS id FROM work_unit_dependencies WHERE
///   work_unit = :owner ORDER BY position`. `AS id` gives the column the
///   one name this function reads, whatever its table calls it.
/// - `owner`: the id of the object the list belongs to (the work unit 7).
/// - `max`: the most elements the list may hold.
///
/// Output: the ids in the order the SELECT gives them (`[12, 15]`); an
/// error when the owner has more than `max` rows.
///
/// Called by: `ai/agent.rs`: `read_agent`; `evaluation/evaluation_plan.rs`:
/// `read_lists`, `order_work_units` (the work units of a plan, read from
/// `work_units`); `evaluation/work_unit.rs`: `read_lists`, `check_links`,
/// `read_work_units`.
pub(in crate::db) fn read_id_list(
    connection: &Connection,
    sql: &str,
    owner: i64,
    max: usize,
) -> Result<Vec<i64>, DbError> {
    ensure!(sql.contains(":owner"), DbError::Inconsistent("a list read without its owner".to_owned()));

    // The rows of the owner, in the order the SELECT gives them.
    let mut statement = connection.prepare(sql)?;
    let mut rows = statement.query(named_params! { ":owner": owner })?;
    let mut ids = Vec::new();

    // One row more than the limit is read (rule 2): if it exists, the
    // database was not written by this program.
    for _ in 0..=max {
        // No more rows: the list is complete.
        let Some(row) = rows.next()? else {
            return Ok(ids);
        };

        ids.push(row.get("id")?);
    }

    Err(DbError::Inconsistent(format!("a list of more than {max} rows in the database")))
}

/// Writes a list of ids that belongs to one object anew, for example the
/// dependencies of a work unit: its old rows removed, each element
/// inserted with its position. Run inside the transaction of its caller,
/// so that no one sees the list half written.
///
/// Input:
/// - `connection`: the open database, inside a transaction.
/// - `delete`: removes the rows of the owner `:owner`: `DELETE FROM
///   work_unit_dependencies WHERE work_unit = :owner`.
/// - `insert`: inserts one row of `:owner`, `:position` and `:id`:
///   `INSERT INTO work_unit_dependencies (work_unit, position, depends_on)
///   VALUES (:owner, :position, :id)`.
/// - `owner`: the id of the object the list belongs to (the work unit 7).
/// - `ids`: the list, its limit checked already by [`check_length`]
///   (`[15, 40]`).
///
/// Output: nothing; the table holds the rows (7, 0, 15) and (7, 1, 40).
/// An error of SQLite when an id names no object (a foreign key) or comes
/// twice (`UNIQUE`); the transaction of the caller then writes nothing.
///
/// Called by: `ai/agent.rs`: `write_lists`; `evaluation/evaluation_plan.rs`:
/// `write_lists`; `evaluation/work_unit.rs`: `write_lists`,
/// `write_dependencies`.
/// Calls: [`position`].
pub(in crate::db) fn write_id_list(
    connection: &Connection,
    delete: &str,
    insert: &str,
    owner: i64,
    ids: &[i64],
) -> Result<(), DbError> {
    ensure!(insert.contains(":position"), DbError::Inconsistent("a list written without positions".to_owned()));

    // The list is written whole, not compared with the old one: simpler,
    // and the positions always follow the order of `ids`.
    connection.execute(delete, named_params! { ":owner": owner })?;

    for (index, id) in ids.iter().enumerate() {
        connection.execute(insert, named_params! { ":owner": owner, ":position": position(index)?, ":id": id })?;
    }

    Ok(())
}
