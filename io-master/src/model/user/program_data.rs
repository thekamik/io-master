//! The data of the program: the installation, made at the first start,
//! and the settings of the user. Every setting is a field of its own, so
//! that the settings that exist can be read here.

/// The installation of the program on one Linux account (user,
/// 2026-10-28). Kept in the table `installation` of the system
/// database.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code, reason = "a new model (2026-10-28), not used by the program yet, only by db/ and its tests")]
pub struct Installation {
    /// The identifier of the installation: 16 random bytes made at the
    /// first start.
    pub id: [u8; 16],

    /// When the program was installed: seconds since 1970 (UTC).
    pub created: i64,
}

/// The settings of the user (user, 2026-10-28). Kept in the table
/// `settings` of the encrypted database of the user.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code, reason = "a new model (2026-10-28), not used by the program yet, only by db/ and its tests")]
pub struct Settings {
    /// The integration used where no agent names one (the chat, the
    /// check of an integration); `None` before one is made.
    pub active_integration: Option<i64>,
}
