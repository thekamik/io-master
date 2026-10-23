//! The checks of conditions that should never happen (rule 5 of
//! CLAUDE.md, "The rules for Rust").
//!
//! `assert!` stops the whole program when it fails, which is no
//! recovery. [`ensure!`] returns an error to the caller instead, so
//! the caller decides what happens, as it does with every other error.
//!
//! The macro is declared before every folder in `main.rs`
//! (`#[macro_use] mod checks;`), so it is known in the whole program
//! by its name alone, like `assert!`.

/// Returns `Err(error)` from the function when `condition` is false.
///
/// ```text
/// ensure!(columns.len() <= MAX_COLUMNS, ScanSettingsError::TooManyColumns(columns.len()));
/// ```
///
/// It expands into one `if` statement and nothing else (rule 8): the
/// condition is evaluated once, the error only when the condition
/// fails.
macro_rules! ensure {
    ($condition:expr, $error:expr) => {
        if !($condition) {
            return Err($error);
        }
    };
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "checks_tests.rs"]
mod tests;
