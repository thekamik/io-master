//! The API key masked in every text taken from an answer: a service may
//! repeat a header in its error, and no error of the program may hold the
//! key (`objects/ai_error.rs`).

use crate::objects::ai_error::AiError;
use crate::objects::error::DomainError;

/// What stands in place of the key where a text could hold it.
pub const MASK: &str = "••••••••";

/// The text with every occurrence of the key replaced by [`MASK`].
///
/// Called by: [`masked`].
fn mask(text: String, key: Option<&str>) -> String {
    match key {
        Some(key) if !key.is_empty() && text.contains(key) => text.replace(key, MASK),
        _ => text,
    }
}

/// The error with the key masked in its explanation.
///
/// Input: `error`, an error of a question sent; `key`, the API key sent with
/// it, `None` when none was.
/// Output: the same error, its text with [`MASK`] for the key ("401: key
/// ••••••••  rejected").
///
/// Called by: `objects/ai/integration/integration_ai/connection.rs`: `send`.
/// Calls: [`mask`].
pub(in crate::objects::ai) fn masked(error: AiError, key: Option<&str>) -> AiError {
    match error {
        AiError::Address(text) => AiError::Address(mask(text, key)),
        AiError::KeyRejected(text) => AiError::KeyRejected(mask(text, key)),
        AiError::NotFound(text) => AiError::NotFound(mask(text, key)),
        AiError::LimitReached(text) => AiError::LimitReached(mask(text, key)),
        AiError::Rejected(status, text) => AiError::Rejected(status, mask(text, key)),
        AiError::ServiceFailed(status, text) => AiError::ServiceFailed(status, mask(text, key)),
        AiError::Reported(text) => AiError::Reported(mask(text, key)),
        AiError::NoConnection(text) => AiError::NoConnection(mask(text, key)),
        AiError::UnexpectedAnswer(text) => AiError::UnexpectedAnswer(mask(text, key)),
        AiError::Refused(text) => AiError::Refused(mask(text, key)),
        // A refusal of a rule may repeat what it refused, an address with
        // the key in it among them (`integration_rules/address.rs`).
        AiError::Domain(DomainError::Refused(text)) => AiError::Domain(DomainError::Refused(mask(text, key))),
        other @ (AiError::NoKey
        | AiError::NoQuestion
        | AiError::TooLong { .. }
        | AiError::TooManyMessages { .. }
        | AiError::Db(_)
        | AiError::Domain(_)) => other,
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the mask of the key.

    use super::*;

    #[test]
    fn every_occurrence_of_the_key_is_masked_and_no_key_leaves_the_text() {
        let error = masked(AiError::Reported("bad sk-1, sk-1".to_owned()), Some("sk-1"));

        assert!(matches!(error, AiError::Reported(text) if text == format!("bad {MASK}, {MASK}")));
        assert!(
            matches!(masked(AiError::Reported("sk-1".to_owned()), None), AiError::Reported(text) if text == "sk-1")
        );
        assert!(matches!(masked(AiError::Reported("x".to_owned()), Some("")), AiError::Reported(text) if text == "x"));
        let refused = AiError::Domain(DomainError::Refused("the address https://x/?k=sk-1".to_owned()));
        let shown = masked(refused, Some("sk-1"));

        assert!(matches!(shown, AiError::Domain(DomainError::Refused(text)) if !text.contains("sk-1")));
    }
}
