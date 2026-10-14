//! The record of the installation: what was chosen at the first
//! start. Whether this is the first start at all is decided in
//! `lifecycle.rs`, together with the TPM.
//!
//! The first start is the only moment at which the role of the
//! installation (workstation or master) is chosen, together with
//! the first user. Afterwards the choice cannot be reached again:
//! the program only offers it in the state [`State::FirstRun`].
//!
//! TODO (step 3 of docs/todo.md): the database does not exist yet.
//! Until then the record is kept in a temporary file,
//! `installation.json` in the data directory. The file goes away
//! when the system database takes its place.

use std::fmt;
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// Name of the temporary installation file in the data directory.
const FILE_NAME: &str = "installation.json";

/// The longest name of a user, in characters.
pub const USER_NAME_MAX: usize = 64;

/// Length of the identifier of an installation in bytes.
pub const ID_LENGTH: usize = 16;

/// The identifier of an installation: random, created at the first
/// start, and kept both in the record and in the marker of the TPM,
/// so that the two can be matched.
pub type InstallationId = [u8; ID_LENGTH];

/// The role of the installation (`docs/architecture.md`,
/// section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The computer of an evaluator: every function except the
    /// master panel. Any number of users with equal rights.
    Workstation,
    /// The password reset service: only its panel, and only the
    /// user who installed it.
    Master,
}

impl Role {
    /// The name of the role as written in the installation.
    pub fn name(self) -> &'static str {
        match self {
            Role::Workstation => "workstation",
            Role::Master => "master",
        }
    }

    /// The role with the given name.
    pub fn from_name(name: &str) -> Option<Role> {
        match name {
            "workstation" => Some(Role::Workstation),
            "master" => Some(Role::Master),
            _ => None,
        }
    }
}

/// What was chosen at the first start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation {
    pub id: InstallationId,
    pub role: Role,
    /// The users of the installation, the first one first.
    pub users: Vec<String>,
}

impl Installation {
    /// Tells whether a user of that name exists.
    pub fn has_user(&self, name: &str) -> bool {
        self.users.iter().any(|user| user == name)
    }

    /// Tells whether users may add further users: on a workstation
    /// they may; a master has only the user who installed it.
    // TODO: used by the management of users (step 3 of
    //       docs/todo.md); until then only by the tests.
    #[allow(dead_code)]
    pub fn can_add_users(&self) -> bool {
        self.role == Role::Workstation
    }
}

/// The state of the installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Nothing is installed yet: the role and the first user are
    /// chosen now.
    FirstRun,
    /// The installation is done.
    Installed(Installation),
}

/// Why the installation could not be read or written.
#[derive(Debug)]
pub enum InstallationError {
    /// No data directory is known: neither `XDG_DATA_HOME` nor
    /// `HOME` is set.
    NoLocation,
    /// The installation exists already; it is never written twice.
    AlreadyInstalled,
    /// A user name that is not allowed; holds the reason.
    InvalidUserName(String),
    /// The installation is damaged: the program must not start.
    /// Holds the file and the reason.
    Damaged { path: PathBuf, reason: String },
    /// A file or directory could not be read or written.
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for InstallationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallationError::NoLocation => {
                formatter.write_str("cannot find the data directory: HOME is not set")
            }
            InstallationError::AlreadyInstalled => formatter.write_str("the program is already installed"),
            InstallationError::InvalidUserName(reason) => write!(formatter, "invalid user name: {reason}"),
            InstallationError::Damaged { path, reason } => {
                write!(formatter, "the installation is damaged ({}): {}", path.display(), reason)
            }
            InstallationError::Io { path, source } => write!(formatter, "{}: {}", path.display(), source),
        }
    }
}

impl std::error::Error for InstallationError {}

/// The usual data directory: `$XDG_DATA_HOME/io-master`, or
/// `~/.local/share/io-master` when that is not set.
pub fn default_data_dir() -> Result<PathBuf, InstallationError> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or(InstallationError::NoLocation)?;

    Ok(base.join("io-master"))
}

/// Checks a user name: 1 to 64 characters, no control characters,
/// no spaces at its ends. Returns the reason when it is refused.
pub fn check_user_name(name: &str) -> Result<(), InstallationError> {
    let length = name.chars().count();

    if length == 0 || length > USER_NAME_MAX {
        return Err(InstallationError::InvalidUserName(format!(
            "a name has 1 to {USER_NAME_MAX} characters"
        )));
    }

    if name.trim() != name || name.chars().any(char::is_control) {
        return Err(InstallationError::InvalidUserName(
            "a name may not start or end with a space or contain control characters".to_owned(),
        ));
    }

    Ok(())
}

/// Reads the state of the installation in a data directory.
pub fn load(data_dir: &Path) -> Result<State, InstallationError> {
    let path = data_dir.join(FILE_NAME);

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(State::FirstRun),
        Err(source) => return Err(InstallationError::Io { path, source }),
    };

    parse(&text)
        .map(State::Installed)
        .map_err(|reason| InstallationError::Damaged { path, reason })
}

/// Writes the record at the first start: the identifier, the role
/// and the first user.
///
/// Fails with [`InstallationError::AlreadyInstalled`] when an
/// installation exists, also when another request wrote it a
/// moment earlier: the file is created only if it does not exist,
/// in one step of the system.
pub fn create(
    data_dir: &Path,
    id: InstallationId,
    role: Role,
    first_user: &str,
) -> Result<Installation, InstallationError> {
    check_user_name(first_user)?;

    // The data directory is private to the user.
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(data_dir)
        .map_err(|source| InstallationError::Io {
            path: data_dir.to_owned(),
            source,
        })?;

    let path = data_dir.join(FILE_NAME);

    let installation = Installation {
        id,
        role,
        users: vec![first_user.to_owned()],
    };

    // create_new: fails when the file exists, so two first starts
    // can never both succeed.
    let mut file = match OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(InstallationError::AlreadyInstalled);
        }
        Err(source) => return Err(InstallationError::Io { path, source }),
    };

    file.write_all(to_text(&installation).as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|source| InstallationError::Io { path, source })?;

    Ok(installation)
}

/// The installation as the text of the file.
fn to_text(installation: &Installation) -> String {
    let document = json!({
        "id": to_hex(&installation.id),
        "role": installation.role.name(),
        "users": installation.users,
    });

    format!("{document:#}\n")
}

/// Reads the text of the file. The error is the reason, in words.
fn parse(text: &str) -> Result<Installation, String> {
    let document: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;

    let id = document
        .get("id")
        .and_then(Value::as_str)
        .and_then(from_hex)
        .ok_or("no valid identifier")?;

    let role = document
        .get("role")
        .and_then(Value::as_str)
        .and_then(Role::from_name)
        .ok_or("no valid role")?;

    let users: Vec<String> = document
        .get("users")
        .and_then(Value::as_array)
        .ok_or("no list of users")?
        .iter()
        .map(|user| user.as_str().map(str::to_owned).ok_or("a user name is not text"))
        .collect::<Result<_, _>>()?;

    if users.is_empty() {
        return Err("no users".to_owned());
    }

    if role == Role::Master && users.len() > 1 {
        return Err("a master has only one user".to_owned());
    }

    Ok(Installation { id, role, users })
}

/// Removes the record; used when the program is uninstalled.
/// A missing record is not an error.
pub fn remove(data_dir: &Path) -> Result<(), InstallationError> {
    let path = data_dir.join(FILE_NAME);

    match fs::remove_file(&path) {
        Err(source) if source.kind() != io::ErrorKind::NotFound => {
            Err(InstallationError::Io { path, source })
        }
        _ => Ok(()),
    }
}

/// The identifier as 32 hexadecimal digits.
pub fn to_hex(id: &InstallationId) -> String {
    id.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The identifier from 32 hexadecimal digits.
fn from_hex(text: &str) -> Option<InstallationId> {
    if text.len() != ID_LENGTH * 2 || !text.is_ascii() {
        return None;
    }

    let mut id = [0u8; ID_LENGTH];

    for (byte, pair) in id.iter_mut().zip(text.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }

    Some(id)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "installation_tests.rs"]
mod tests;
