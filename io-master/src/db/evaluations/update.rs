//! Changing the results of the requirements of an evaluation. The
//! structure of an evaluation is never changed (there is no function
//! for it). Every change is written with its entry of the journal, in
//! one transaction (`create.rs`, `journal`).

use rusqlite::params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::evaluation::model::{Proposal, ResultOrigin};

use super::create::{Step, journal};

/// Makes `text` the result of the field of `proposal` (replacing an
/// earlier one) and removes the proposal, with the entry `step`, in one
/// transaction: the evaluator accepted the proposal, as it is or
/// corrected.
pub fn settle_proposal(
    db: &UserDb,
    evaluation: i64,
    proposal: &Proposal,
    text: &str,
    origin: ResultOrigin,
    step: &Step<'_>,
) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "INSERT INTO evaluation_results (tile_id, field, text, origin) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT (tile_id, field) DO UPDATE SET text = excluded.text, origin = excluded.origin",
        params![proposal.tile, proposal.field.name(), text, origin.name()],
    )?;
    transaction.execute("DELETE FROM evaluation_proposals WHERE id = ?1", [proposal.id])?;
    journal(&transaction, evaluation, step)?;
    transaction.commit()?;

    Ok(())
}

/// Stores the result the evaluator wrote for the field of `step`
/// (`Some`: made or replaced, origin evaluator; `None`: removed), with
/// the entry `step`, in one transaction.
pub fn result(db: &UserDb, evaluation: i64, tile: i64, text: Option<&str>, step: &Step<'_>) -> Result<(), DbError> {
    let field = step.field.ok_or_else(|| DbError::Sql("a result needs its field".to_owned()))?;
    let transaction = db.connection().unchecked_transaction()?;

    match text {
        None => {
            transaction.execute(
                "DELETE FROM evaluation_results WHERE tile_id = ?1 AND field = ?2",
                params![tile, field.name()],
            )?;
        }
        Some(text) => {
            transaction.execute(
                "INSERT INTO evaluation_results (tile_id, field, text, origin) VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT (tile_id, field) DO UPDATE SET text = excluded.text, origin = excluded.origin",
                params![tile, field.name(), text, ResultOrigin::Evaluator.name()],
            )?;
        }
    }

    journal(&transaction, evaluation, step)?;
    transaction.commit()?;

    Ok(())
}
