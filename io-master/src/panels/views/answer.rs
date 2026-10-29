//! The answer of the AI as a command awaits it whole: the prepared question
//! is sent (`ai/ask.rs`) and the pieces of its answer, which a chat would
//! show as they come, are drained. Used where the page wants the whole
//! answer at once: the check of an integration, "Fill with AI".

use tokio::sync::mpsc;

use crate::ai::ask::{Prepared, Sent, send};

/// The most pieces of an answer drained: every piece has at least one byte
/// of the stream, whose limit is 32 MiB (`ai/client.rs`).
const MAX_PIECES: usize = 32 * 1024 * 1024;

/// Sends the prepared question and drains the pieces of its answer. Uses no
/// database: the caller has released the lock of the session.
///
/// Called by: `settings/integrations.rs` (`integration_check`),
/// `evaluation_plan/fill.rs` (`plan_fill`).
/// Calls: `ask::send`.
pub async fn send_drained(prepared: &Prepared) -> Sent {
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
