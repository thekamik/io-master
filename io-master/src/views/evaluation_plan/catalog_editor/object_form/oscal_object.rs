//! A group or a control of OSCAL sent by the page as JSON, read into the
//! types of the program; what is not OSCAL is refused before it reaches
//! the catalog.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::app::error::AppError;

/// `value` read as a `T` of OSCAL (a group or a control).
pub fn read<T: DeserializeOwned>(value: Value) -> Result<T, AppError> {
    match serde_json::from_value(value) {
        Ok(object) => Ok(object),
        Err(error) => Err(AppError::Invalid(format!("the object is not OSCAL: {error}"))),
    }
}
