//! Where the program sends a question: the address of an integration and
//! the CA certificate it trusts, checked before anything is sent.
//!
//! - the address is `https://`, or `http://` only on this computer (a local
//!   model), with no password in it ([`check_address`]);
//! - a CA certificate of an integration, when it has one, is a PEM that
//!   holds at least one certificate ([`certificates`]); the connection then
//!   trusts it alone (`integration_ai/connection.rs`).
//!
//! A rule of the integration: the settings of the AI check an integration
//! with the same functions when it is saved (`settings.rs`), and the AI of
//! the integration before it sends (`integration_ai/question.rs`,
//! `integration_ai/connection.rs`), so that what is saved can be used.

use reqwest::{Certificate, Url};

use crate::objects::error::DomainError;

/// The beginning of every refusal of this file, so that the user knows what
/// was refused.
const REFUSED: &str = "the address of the integration";

/// Refuses an address the program does not send to: one that is no URL,
/// one that is neither `https://` nor `http://` on this computer.
///
/// Input: `url`, the address of an integration
/// (`"http://127.0.0.1:11434/v1/chat/completions"`).
///
/// Output: nothing when the program sends to it; `DomainError::Refused`
/// with the reason otherwise ("the address of the integration: a password
/// in the address").
///
/// Called by: `settings.rs`: `checked` (an integration saved);
/// `integration_ai/question.rs`: `prepare`; `integration_ai/connection.rs`:
/// `exchange`.
pub fn check_address(url: &str) -> Result<(), DomainError> {
    let parsed = Url::parse(url).map_err(|error| DomainError::Refused(format!("{REFUSED}: {url}: {error}")))?;
    let local = matches!(parsed.host_str(), Some("127.0.0.1" | "[::1]" | "localhost"));

    ensure!(
        parsed.scheme() == "https" || (parsed.scheme() == "http" && local),
        DomainError::Refused(format!("{REFUSED}: only https://, or http:// on this computer (127.0.0.1, localhost)"))
    );
    ensure!(
        parsed.username().is_empty() && parsed.password().is_none(),
        DomainError::Refused(format!("{REFUSED}: a password in the address"))
    );

    Ok(())
}

/// The certificates of a CA certificate in PEM; refused when it holds none
/// or is no PEM.
///
/// Called by: `integration_ai/connection.rs` (`http_client`); [`check_certificate`].
pub(in crate::objects::ai::integration) fn certificates(pem: &str) -> Result<Vec<Certificate>, DomainError> {
    let certificates = Certificate::from_pem_bundle(pem.as_bytes())
        .map_err(|error| DomainError::Refused(format!("{REFUSED}: the CA certificate: {error}")))?;

    ensure!(
        !certificates.is_empty(),
        DomainError::Refused(format!("{REFUSED}: the CA certificate holds no certificate"))
    );

    Ok(certificates)
}

/// Refuses a CA certificate the connection could not use.
///
/// Called by: `settings.rs`: `checked` (an integration saved).
/// Calls: [`certificates`].
pub fn check_certificate(pem: &str) -> Result<(), DomainError> {
    certificates(pem)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the checks of an address and of a CA certificate.

    use super::*;

    #[test]
    fn plain_http_only_on_this_computer_and_no_password_in_the_address() {
        assert!(check_address("http://127.0.0.1:11434/v1/chat/completions").is_ok());
        assert!(check_address("http://example.com/v1/chat/completions").is_err());
        assert!(check_address("https://user:pw@example.com/v1").is_err());
        assert!(check_address("https://api.anthropic.com/v1/messages").is_ok());
    }

    #[test]
    fn a_certificate_that_is_no_pem_is_refused() {
        assert!(check_certificate("no certificate").is_err());
    }

    #[test]
    fn a_refusal_says_what_was_refused() {
        let Err(DomainError::Refused(reason)) = check_address("http://example.com/v1") else {
            panic!("the address is refused");
        };

        assert_eq!(
            reason,
            "the address of the integration: only https://, or http:// on this computer (127.0.0.1, localhost)"
        );
    }
}
