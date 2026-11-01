//! The protection of the memory of the process:
//! the decrypted data and the key of the database are only in RAM, so the
//! process must never be written into a file by the system.

use rustix::io::Errno;
use rustix::process::{DumpableBehavior, set_dumpable_behavior};

/// Forbids the memory dumps of the process: a crash writes no core file
/// with the decrypted data, and no other process of the account may attach
/// to it (`prctl(PR_SET_DUMPABLE, 0)`).
///
/// Called by: `tauri_startup` (`main.rs`), before the window opens.
/// Calls: `rustix::process::set_dumpable_behavior`.
pub fn forbid_memory_dumps() -> Result<(), Errno> {
    set_dumpable_behavior(DumpableBehavior::NotDumpable)
}

#[cfg(test)]
mod tests {
    use super::*;

    use rustix::process::dumpable_behavior;

    #[test]
    fn the_process_is_no_longer_dumpable() {
        forbid_memory_dumps().unwrap();

        assert_eq!(dumpable_behavior().unwrap(), DumpableBehavior::NotDumpable);
    }
}
