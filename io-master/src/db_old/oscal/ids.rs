//! New UUIDs and the time of a change, for the documents of OSCAL the
//! program makes and changes.

use crate::model_old::oscal::values::{date_time, uuid_v4};
use crate::model_old::time::now;
use crate::security::random::fill_random;

use super::error::OscalError;

/// A new UUID of version 4, from random bytes of the kernel.
pub fn new_uuid() -> Result<String, OscalError> {
    let mut bytes = [0u8; 16];

    if let Err(error) = fill_random(&mut bytes) {
        return Err(OscalError::Invalid(format!("no random bytes: {error}")));
    }

    Ok(uuid_v4(bytes))
}

/// The date and time now, as OSCAL writes it (the `last-modified` of a
/// change).
pub fn now_date_time() -> String {
    date_time(now())
}
