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
/// ensure!(level <= MAX_DEPTH, CatalogError::TooDeep);
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

#[cfg(test)]
mod tests {
    //! Unit tests of `checks.rs`.

    /// Halves an even number; an odd one is an error.
    fn half(number: u32) -> Result<u32, String> {
        ensure!(number.is_multiple_of(2), format!("{number} is odd"));

        Ok(number / 2)
    }

    #[test]
    fn a_condition_that_holds_lets_the_function_go_on() {
        assert_eq!(half(10), Ok(5));
    }

    #[test]
    fn a_condition_that_fails_returns_the_error() {
        assert_eq!(half(7), Err("7 is odd".to_owned()));
    }
}
