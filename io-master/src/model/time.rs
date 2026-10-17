//! The time stamps of the program: seconds since 1970 (UTC), as the
//! databases keep them.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since 1970 now; 0 for a clock set before 1970.
pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| i64::try_from(time.as_secs()).unwrap_or(i64::MAX))
}
