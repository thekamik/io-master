//! Where the data of the program is: `$XDG_DATA_HOME/io-master`, or
//! `~/.local/share/io-master` when that is not set. One Linux account holds
//! one installation (`docs/architecture.md`, section 3).

use std::path::PathBuf;

/// The name of the folder of the program in the data folder of the user.
const FOLDER: &str = "io-master";

/// The data folder of the program; `None` when neither `XDG_DATA_HOME` nor
/// `HOME` is set, so that the program cannot know where its data is.
///
/// Called by: `main.rs` (the start of the window).
pub fn default_data_folder() -> Option<PathBuf> {
    if let Some(base) = std::env::var_os("XDG_DATA_HOME")
        && !base.is_empty()
    {
        return Some(PathBuf::from(base).join(FOLDER));
    }

    let home = std::env::var_os("HOME")?;

    if home.is_empty() {
        return None;
    }

    Some(PathBuf::from(home).join(".local").join("share").join(FOLDER))
}
