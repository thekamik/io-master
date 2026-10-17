//! Changing the results of the requirements of an evaluation. The
//! structure of an evaluation is never changed (there is no function
//! for it). Every change is written with its entry of the journal, in
//! one transaction (`journal/create.rs`).

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::evaluations::{Proposal, ResultField, ResultOrigin};

use super::journal::create::{JournalStep, entry};

/// The statement that makes or replaces the result of a field.
const STORE_RESULT: &str = "INSERT INTO evaluation_results (tile_id, field, text, origin) \
VALUES (:tile, :field, :text, :origin) \
ON CONFLICT (tile_id, field) DO UPDATE SET text = excluded.text, origin = excluded.origin";

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
    step: &JournalStep<'_>,
) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        STORE_RESULT,
        named_params! {
            ":tile": proposal.tile,
            ":field": proposal.field.name(),
            ":text": text,
            ":origin": origin.name(),
        },
    )?;
    transaction.execute("DELETE FROM evaluation_proposals WHERE id = :id", named_params! { ":id": proposal.id })?;
    entry(&transaction, evaluation, step)?;
    transaction.commit()?;

    Ok(())
}

/// Stores the result the evaluator wrote for `field` of requirement
/// `tile` (`Some`: made or replaced, origin evaluator; `None`:
/// removed), with the entry `step`, in one transaction.
pub fn result(
    db: &UserDb,
    evaluation: i64,
    tile: i64,
    field: ResultField,
    text: Option<&str>,
    step: &JournalStep<'_>,
) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    match text {
        None => {
            transaction.execute(
                "DELETE FROM evaluation_results WHERE tile_id = :tile AND field = :field",
                named_params! { ":tile": tile, ":field": field.name() },
            )?;
        }
        Some(text) => {
            transaction.execute(
                STORE_RESULT,
                named_params! {
                    ":tile": tile,
                    ":field": field.name(),
                    ":text": text,
                    ":origin": ResultOrigin::Evaluator.name(),
                },
            )?;
        }
    }

    entry(&transaction, evaluation, step)?;
    transaction.commit()?;

    Ok(())
}
