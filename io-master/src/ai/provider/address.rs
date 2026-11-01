//! Where the program sends a question: the address of an integration and
//! the CA certificate it trusts, checked before anything is sent.
//!
//! - the address is `https://`, or `http://` only on this computer (a local
//!   model), with no password in it ([`check_address`]);
//! - a CA certificate of an integration, when it has one, is a PEM that
//!   holds at least one certificate ([`certificates`]); the connection then
//!   trusts it alone (`connection.rs`).
//!
//! The settings of the AI check an integration with the same functions
//! when it is saved, so that what is saved can be used.

use reqwest::{Certificate, Url};

use crate::ai::error::AiError;

/// Refuses an address the program does not send to: one that is no URL,
/// one that is neither `https://` nor `http://` on this computer.
///
/// Called by: `ai/ask/question.rs`: `prepare`,
/// `connection.rs` (`exchange`); the settings of the AI (an integration
/// saved).
pub fn check_address(url: &str) -> Result<(), AiError> {
    let parsed = Url::parse(url).map_err(|error| AiError::Address(format!("{url}: {error}")))?;
    let local = matches!(parsed.host_str(), Some("127.0.0.1" | "[::1]" | "localhost"));

    ensure!(
        parsed.scheme() == "https" || (parsed.scheme() == "http" && local),
        AiError::Address("only https://, or http:// on this computer (127.0.0.1, localhost)".to_owned())
    );
    ensure!(
        parsed.username().is_empty() && parsed.password().is_none(),
        AiError::Address("a password in the address".to_owned())
    );

    Ok(())
}

/// The certificates of a CA certificate in PEM; refused when it holds none
/// or is no PEM.
///
/// Called by: `connection.rs` (`http_client`); [`check_certificate`].
pub(in crate::ai::provider) fn certificates(pem: &str) -> Result<Vec<Certificate>, AiError> {
    let certificates = Certificate::from_pem_bundle(pem.as_bytes())
        .map_err(|error| AiError::Address(format!("the CA certificate: {error}")))?;

    ensure!(!certificates.is_empty(), AiError::Address("the CA certificate holds no certificate".to_owned()));

    Ok(certificates)
}

/// Refuses a CA certificate the connection could not use.
///
/// Called by: the settings of the AI (an integration saved).
/// Calls: [`certificates`].
pub fn check_certificate(pem: &str) -> Result<(), AiError> {
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
}
