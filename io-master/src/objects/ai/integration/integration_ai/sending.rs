//! The second step of the way to the AI: the prepared question sent, the
//! answer read while it streams. It uses no database: the caller has
//! released the lock of the session, so that the other commands of the
//! window are not held up by a long answer.
//!
//! ```text
//! question.rs: prepare ──► Prepared ──► send ──► Sent ──► journal_ai.rs: record
//!                                  └──► send_whole (the pieces drained: the answer awaited whole)
//! ```
//!
//! The answer comes out as [`Sent`], which only `record` opens: no caller
//! reads an answer before it is written into the journal of its agent.

use tokio::sync::mpsc;

use crate::objects::ai::integration::integration_ai::connection::{self, Received};
use crate::objects::ai::integration::integration_ai::question::Prepared;

/// The most pieces of an answer drained: every piece has at least one byte
/// of the stream, whose limit is 32 MiB (`connection.rs`).
const MAX_PIECES: usize = 32 * 1024 * 1024;

/// The answer as it came, not written into the journal yet: only `record`
/// (`objects/ai/journal/journal_ai.rs`) gives it out.
#[must_use = "an answer is read only through record(), which writes the journal of its agent"]
pub struct Sent(pub(in crate::objects::ai) Received);

/// Sends the question and passes the pieces of the answer into `pieces`
/// while it is written; stops when the receiver of `pieces` is dropped.
/// Uses no database.
///
/// Input: `prepared`, the question prepared; `pieces`, where each piece goes.
/// Output: the answer as it came, readable only through
/// `objects/ai/journal/journal_ai.rs`: `record`.
///
/// Called by: [`send_whole`]; `panels/views/chat/chats.rs`: `forwarded`, the
/// lock of the session released.
/// Calls: `objects/ai/integration/integration_ai/connection.rs`: `send`.
pub async fn send(prepared: &Prepared, pieces: &mpsc::Sender<String>) -> Sent {
    Sent(connection::send(&prepared.request, pieces).await)
}

/// Sends the prepared question and drains the pieces of its answer: for a
/// caller that wants the whole answer at once. Uses no database.
///
/// Input: `prepared`, the question prepared.
/// Output: the answer as it came, readable only through
/// `objects/ai/journal/journal_ai.rs`: `record`.
///
/// Called by: `panels/views/settings/integrations.rs`: `integration_check`;
/// `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `fill_work_unit`;
/// `objects/evaluation/work_unit/work_unit_ai/assessment.rs`: `assess_work_unit`; the
/// lock of the session released.
/// Calls: [`send`].
pub async fn send_whole(prepared: &Prepared) -> Sent {
    let (pieces, mut receiver) = mpsc::channel::<String>(64);
    let sending = async move { send(prepared, &pieces).await };
    let draining = async move {
        for _ in 0..MAX_PIECES {
            if receiver.recv().await.is_none() {
                break;
            }
        }
    };
    let (sent, ()) = tokio::join!(sending, draining);

    sent
}
