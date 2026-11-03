//! The time of the computer, as the program writes it into the databases:
//! seconds since 1970 (UTC), the time of a message, of an entry of the
//! journal, of an evaluation begun, of an installation.
//!
//! One function for the whole program, at the bottom of the layers, so
//! that every time written is read from the clock in the same way; and the
//! day of a time, as a report writes it.

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
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/evaluation.rs`: `new_evaluation`;
/// `objects/user/user_data/user_data_rules.rs`: `install`.
pub fn now() -> Result<i64, ClockError> {
    let seconds =
        SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| ClockError(error.to_string()))?.as_secs();

    i64::try_from(seconds).map_err(|error| ClockError(error.to_string()))
}

/// The day of the time `seconds` (seconds since 1970, UTC), as a report
/// writes it: `"2026-11-03"`.
///
/// Input: `seconds`, a time as the program writes it (`1793750400`).
/// Output: the day, year-month-day (`"2026-11-03"`).
///
/// Called by: `objects/evaluation/evaluation_plan/evaluation_plan_rules/fields.rs`: `evaluation_plan_fields`;
/// `objects/evaluation/lab_proof/lab_proof_rules.rs`: `lab_proof_fields`.
pub fn date_of(seconds: i64) -> String {
    // The days since 1970 counted from 1 March of the year 0, in eras of 400
    // years of 146 097 days each (H. Hinnant, "chrono-Compatible Low-Level
    // Date Algorithms"): no loop, a date of any year.
    let days = seconds.div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 { month_from_march + 3 } else { month_from_march - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    //! Tests of the time of the computer and of its day.

    use super::*;

    #[test]
    fn the_time_is_after_the_program_was_written() {
        // 2026-10-01 00:00 UTC.
        assert!(now().unwrap() > 1_790_812_800);
    }

    #[test]
    fn a_time_gives_its_day() {
        assert_eq!(date_of(0), "1970-01-01");
        assert_eq!(date_of(1_790_812_800), "2026-10-01");
        assert_eq!(date_of(1_709_164_800 + 86_399), "2024-02-29");
        assert_eq!(date_of(951_782_400), "2000-02-29");
        assert_eq!(date_of(-1), "1969-12-31");
    }
}
