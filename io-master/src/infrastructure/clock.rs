//! The time of the computer, as the program writes it into the databases:
//! seconds since 1970 (UTC), the time of a message, of an entry of the
//! journal, of an evaluation begun, of an installation.
//!
//! One function for the whole program, at the bottom of the layers, so
//! that every time written is read from the clock in the same way.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// The clock of the computer gives a time the program cannot write: before
/// 1970, or past the year 292 277 026 596.
#[derive(Debug)]
pub struct ClockError(String);

impl fmt::Display for ClockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "the clock of the computer: {}", self.0)
    }
}

impl std::error::Error for ClockError {}

/// The time now, in seconds since 1970 (UTC): `1793577600`.
///
/// Output: the seconds; an error when the clock of the computer is set
/// before 1970.
///
/// Called by: `objects/ai/integration/integration_ai/question.rs`: `prepare`;
/// `objects/ai/chat/chat_rules.rs`: `add_question`; `objects/ai/chat/chat_ai.rs`:
/// `add_answer`;
/// `domain/models/evaluation/evaluation_plan/evaluation.rs`: `new_evaluation`;
/// `objects/user/user_data/user_data_rules.rs`: `install`.
pub fn now() -> Result<i64, ClockError> {
    let seconds =
        SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| ClockError(error.to_string()))?.as_secs();

    i64::try_from(seconds).map_err(|error| ClockError(error.to_string()))
}

#[cfg(test)]
mod tests {
    //! Tests of the time of the computer.

    use super::*;

    #[test]
    fn the_time_is_after_the_program_was_written() {
        // 2026-10-01 00:00 UTC.
        assert!(now().unwrap() > 1_790_812_800);
    }
}
