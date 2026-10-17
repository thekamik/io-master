//! The key that encrypts a database of the program.
//!
//! The key is 32 random bytes (256 bits), the size that SQLCipher
//! and AES-256 use; it is created by `generate_key.rs`. It is never
//! stored as it is: step 3 of `docs/todo.md` stores it encrypted
//! with the password of the evaluator and the secret of the device
//! (`docs/architecture.md`, section 5.2).

// TODO: Remove when the database uses this file (step 3 of
//       docs/todo.md); until then the compiler sees no user of it.
#![allow(dead_code)]

use std::fmt;

use zeroize::Zeroizing;

/// Length of the key in bytes.
pub const DATABASE_KEY_LENGTH: usize = 32;

/// The key of a database.
///
/// It is wiped from memory when dropped, is not shown by `{:?}`,
/// cannot be cloned, and gives its bytes out only through
/// [`DatabaseKey::expose`]. It holds exactly 32 bytes, so a key of
/// the wrong length cannot exist.
///
/// The bytes live on the heap (`Box`). Handing a `DatabaseKey`
/// from one function to another then moves only a pointer; bytes
/// kept in the value itself would be copied on every move, and
/// those copies would not be wiped.
pub struct DatabaseKey(Box<Zeroizing<[u8; DATABASE_KEY_LENGTH]>>);

impl DatabaseKey {
    /// Creates a key by letting `fill` write its bytes directly
    /// into the memory of the key, so they exist only there.
    /// When `fill` fails, its error is returned and the partly
    /// written bytes are wiped.
    pub fn fill_with<E>(
        fill: impl FnOnce(&mut [u8; DATABASE_KEY_LENGTH]) -> Result<(), E>,
    ) -> Result<DatabaseKey, E> {
        let mut key = DatabaseKey(Box::new(Zeroizing::new([0; DATABASE_KEY_LENGTH])));

        fill(&mut key.0)?;

        Ok(key)
    }

    /// The bytes of the key. Use them only where the key is
    /// actually needed: when the database is opened.
    pub fn expose(&self) -> &[u8; DATABASE_KEY_LENGTH] {
        &self.0
    }
}

impl fmt::Debug for DatabaseKey {
    /// Shows that there is a key, never what it is.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DatabaseKey(<hidden>)")
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "database_key_tests.rs"]
mod tests;
