//! The API key of an integration with an AI service. It is a password to
//! an account, often a paid one, so it is kept only in the encrypted
//! database of the user (the table `secrets`), apart from the
//! integration, and never goes back to the page.

use zeroize::Zeroizing;

/// The API key of an integration (user, 2026-10-28). It derives no
/// `Debug` and no `Clone`, so that it is never printed into a log or an
/// error and has one copy in memory, wiped when it is dropped.
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct ApiKey {
    /// The integration the key belongs to.
    pub integration: i64,

    /// The key itself.
    pub key: Zeroizing<String>,
}
