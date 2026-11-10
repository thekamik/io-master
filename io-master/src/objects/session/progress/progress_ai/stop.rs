//! A question of a long work of the AI that the "Stop" of the user ends
//! (user, 2026-11-01): the answer is awaited whole, and while it comes the
//! progress of the session is looked at, so that a slow model does not hold
//! the "Stop" up until its answer ends.
//!
//! ```text
//! integration_ai/question.rs: prepare ──► Prepared ──► send_until_stopped ──► Sent ──► journal_ai.rs: record
//!                                                        ├► integration_ai/sending.rs: send (the pieces drained)
//!                                                        └► Progress::stop, every STOP_CHECK ──► the receiver of
//!                                                           the pieces dropped: the answer ends as Stopped
//! ```

use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::objects::ai::integration::integration_ai::question::Prepared;
use crate::objects::ai::integration::integration_ai::sending::{Sent, send};
use crate::objects::session::progress::progress_model::Progress;

/// How often the waiting for an answer looks whether the user asked the work
/// to stop.
const STOP_CHECK: Duration = Duration::from_millis(200);

/// The most steps of the waiting for an answer: a piece or a look for a
/// stop. Every piece has at least one byte of the stream, whose limit is 32
/// MiB (`objects/ai/integration/integration_ai/connection.rs`); the looks are
/// as many again, which the silence of the stream ends long before.
const MAX_STEPS: usize = 2 * 32 * 1024 * 1024;

/// Whether the work goes on: the user did not ask it to stop. A broken lock
/// of the progress stops it as a "Stop" does.
///
/// Called by: [`send_until_stopped`].
fn goes_on(progress: &Mutex<Progress>) -> bool {
    match progress.lock() {
        Ok(progress) => !progress.stop,
        Err(_) => false,
    }
}

/// Sends the prepared question and awaits its whole answer, unless the user
/// asks the work to stop: the answer then ends at once, as stopped, with
/// what was written of it. Uses no database.
///
/// Input: `prepared`, the question prepared; `progress`, the lock of the
/// progress of the session, whose `stop` the "Stop" of the user sets.
/// Output: the answer as it came, readable only through
/// `objects/ai/journal/journal_ai.rs`: `record`.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/assessment.rs`: `assess_work_unit`;
/// `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `fill_work_unit`; the lock of
/// the database released.
/// Calls: `objects/ai/integration/integration_ai/sending.rs`: `send`; [`goes_on`].
pub async fn send_until_stopped(prepared: &Prepared, progress: &Mutex<Progress>) -> Sent {
    let (pieces, mut receiver) = mpsc::channel::<String>(64);
    let sending = async move { send(prepared, &pieces).await };
    // When this ends, the receiver of the pieces is dropped, which ends the
    // sending (`connection.rs`: `read_stream`).
    let draining = async move {
        let mut stop_check = tokio::time::interval(STOP_CHECK);

        for _ in 0..MAX_STEPS {
            let go_on = tokio::select! {
                piece = receiver.recv() => piece.is_some(),
                _ = stop_check.tick() => goes_on(progress),
            };

            if !go_on {
                break;
            }
        }
    };
    let (sent, ()) = tokio::join!(sending, draining);

    sent
}
