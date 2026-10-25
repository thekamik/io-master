//! The permissions of the folder of the installation (the audit of
//! 2026-10-25; a candidate of step 12 of docs/todo.md): the folder and its
//! files are the account's alone. The program makes the folders `0700` and
//! the databases `0600`; this check finds a folder or a file that other
//! accounts of the computer may open (changed by hand, copied, restored from
//! a backup), and a folder that belongs to another account. It changes
//! nothing: the window shows what it finds and how to correct it.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::db::user::database::DIRECTORY;

/// The most entries of a folder that are looked at.
const MAX_ENTRIES: usize = 10_000;

/// The bits of a mode that let the group or the others in.
const OPEN_BITS: u32 = 0o077;

/// What the check found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionProblem {
    /// A folder or a file that other accounts may read or write.
    Open(PathBuf),
    /// The folder of the installation belongs to another account.
    Owner(PathBuf),
}

/// The problems of the folder `data_dir` (none when it does not exist yet):
/// its owner, its mode, and the modes of the folder of the databases of
/// the evaluators and of every file in both, at most [`MAX_ENTRIES`] each.
///
/// Called by: `views/shell/program/view.rs` (the bars at the top of the window).
/// Calls: [`open`], [`open_entries`].
pub fn problems(data_dir: &Path) -> Vec<PermissionProblem> {
    let Ok(metadata) = fs::metadata(data_dir) else {
        return Vec::new();
    };
    let mut found = Vec::new();

    if metadata.uid() != rustix::process::geteuid().as_raw() {
        found.push(PermissionProblem::Owner(data_dir.to_path_buf()));
    }

    open(data_dir, &mut found);
    open_entries(data_dir, &mut found);

    let databases = data_dir.join(DIRECTORY);

    if databases.is_dir() {
        open(&databases, &mut found);
        open_entries(&databases, &mut found);
    }

    found
}

/// Adds `path` to `found` when other accounts may open it.
///
/// Called by: [`problems`], [`open_entries`].
fn open(path: &Path, found: &mut Vec<PermissionProblem>) {
    if let Ok(metadata) = fs::symlink_metadata(path)
        && metadata.mode() & OPEN_BITS != 0
    {
        found.push(PermissionProblem::Open(path.to_path_buf()));
    }
}

/// Adds every file of `folder` that other accounts may open to `found`.
///
/// Called by: [`problems`].
/// Calls: [`open`].
fn open_entries(folder: &Path, found: &mut Vec<PermissionProblem>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };

    // An entry that cannot be read is passed over: the check only warns.
    for entry in entries.take(MAX_ENTRIES).flatten() {
        let path = entry.path();

        if path.is_file() {
            open(&path, found);
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `permissions.rs`.

    use super::*;
    use std::os::unix::fs::PermissionsExt;

    use crate::test_support::TempDir;

    #[test]
    fn a_private_folder_has_no_problem_and_an_open_one_is_found() {
        let directory = TempDir::new();
        let folder = directory.path().join("io-master");
        fs::create_dir(&folder).unwrap();
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).unwrap();
        let file = folder.join("system.db");
        fs::write(&file, "x").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();

        assert_eq!(problems(&folder), []);

        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(problems(&folder), [PermissionProblem::Open(folder.clone()), PermissionProblem::Open(file)]);
        assert_eq!(problems(&directory.path().join("missing")), []);
    }
}
