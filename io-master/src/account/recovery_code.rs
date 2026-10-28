//! The recovery code of a forgotten password (step 49 of docs/history.md,
//! `docs/architecture.md`, section 5.4): 160 random bits, written for a
//! person as seven groups of five characters, the last three of them a
//! check sum against typing errors.
//!
//! ```text
//! K7QX4-3MPAZ-...-...-...-...-..CCC     32 characters of the code (5 bits each),
//!                                       3 of the check sum, in groups of 5
//! ```
//!
//! The alphabet has no 0, O, 1 and I, which are easy to mix up. When a
//! code is typed, the letter case, the spaces and the dashes do not
//! matter. This file only writes and reads the text; the random bytes
//! are made by `account/recovery.rs`, which also wraps the key of
//! the database with them.

use std::fmt;

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

/// The number of random bytes of a code: 160 bits.
pub const CODE_BYTES: usize = 20;

/// The 32 characters of a code, 5 bits each.
const ALPHABET: &[u8; 32] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// The characters of the code itself: 160 bits / 5.
const DATA_CHARS: usize = 32;

/// The characters of the check sum: 15 bits of SHA-256 of the code.
const CHECK_CHARS: usize = 3;

/// All characters, without the dashes.
const ALL_CHARS: usize = DATA_CHARS + CHECK_CHARS;

/// The characters of a group.
const GROUP: usize = 5;

/// The most characters a typed code may have (with spaces and dashes),
/// so that a mistaken paste of a document is refused at once.
const TYPED_MAX: usize = 128;

/// A recovery code. Its bytes are wiped from memory when it is dropped.
pub struct RecoveryCode {
    bytes: Zeroizing<[u8; CODE_BYTES]>,
}

/// Why a typed code is not a recovery code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryCodeError {
    /// It has not 35 characters (the dashes and spaces aside).
    Length,
    /// A character that no code has (0, O, 1, I, ...).
    Character,
    /// The check sum does not fit: a typing error.
    Check,
}

impl fmt::Display for RecoveryCodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryCodeError::Length => formatter.write_str("a recovery code has 35 characters"),
            RecoveryCodeError::Character => {
                formatter.write_str("a recovery code has only the characters 2 to 9 and A to Z without I and O")
            }
            RecoveryCodeError::Check => formatter.write_str("the recovery code has a typing error"),
        }
    }
}

impl std::error::Error for RecoveryCodeError {}

// The code is a secret: it is never printed by the debug output.
impl fmt::Debug for RecoveryCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RecoveryCode(..)")
    }
}

impl RecoveryCode {
    /// The code of these random bytes.
    ///
    /// Called by: `account/recovery.rs` (`new_code`), and the tests.
    pub fn from_bytes(bytes: [u8; CODE_BYTES]) -> RecoveryCode {
        RecoveryCode { bytes: Zeroizing::new(bytes) }
    }

    /// The bytes of the code, for the derivation of its key.
    ///
    /// Called by: `account/recovery.rs`.
    pub fn expose(&self) -> &[u8; CODE_BYTES] {
        &self.bytes
    }

    /// The code as a person reads it: seven groups of five characters.
    ///
    /// Called by: `components/recovery_code/view.rs`, to show it once.
    /// Calls: [`symbols`], [`group`].
    pub fn to_text(&self) -> Zeroizing<String> {
        group(&symbols(&self.bytes))
    }

    /// Reads a typed code: the letter case, spaces and dashes aside.
    ///
    /// Called by: `views/users/users.rs` (the recovery of a forgotten
    /// password), when the code comes from the page.
    /// Calls: [`value_of`], [`symbols`].
    pub fn parse(typed: &str) -> Result<RecoveryCode, RecoveryCodeError> {
        ensure!(typed.len() <= TYPED_MAX, RecoveryCodeError::Length);

        let mut values: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(ALL_CHARS));

        for character in typed.chars() {
            if character == '-' || character.is_whitespace() {
                continue;
            }

            ensure!(values.len() < ALL_CHARS, RecoveryCodeError::Length);
            values.push(value_of(character).ok_or(RecoveryCodeError::Character)?);
        }

        ensure!(values.len() == ALL_CHARS, RecoveryCodeError::Length);

        let bytes = bytes_of(&values)?;
        let code = RecoveryCode::from_bytes(bytes);

        // The check sum of the bytes read must be the one typed.
        ensure!(symbols(&code.bytes)[DATA_CHARS..] == values[DATA_CHARS..], RecoveryCodeError::Check);

        Ok(code)
    }
}

/// The 35 values (0 to 31) of a code: its 160 bits five by five, then 15
/// bits of SHA-256 of the bytes.
///
/// Called by: [`RecoveryCode::to_text`], [`RecoveryCode::parse`].
/// Calls: [`bits`].
fn symbols(bytes: &[u8; CODE_BYTES]) -> Zeroizing<Vec<u8>> {
    let mut values = Zeroizing::new(Vec::with_capacity(ALL_CHARS));

    for index in 0..DATA_CHARS {
        values.push(bits(bytes, index * 5));
    }

    let digest = Sha256::digest(bytes);
    let check = [digest[0], digest[1]];

    for index in 0..CHECK_CHARS {
        values.push(bits(&check, index * 5));
    }

    values
}

/// The five bits of `bytes` that start at bit `offset` (the highest bit of
/// the first byte is bit 0); bits past the end are zero.
///
/// Called by: [`symbols`].
fn bits(bytes: &[u8], offset: usize) -> u8 {
    let mut value = 0u8;

    for bit in offset..offset + 5 {
        let byte = bytes.get(bit / 8).copied().unwrap_or(0);
        let set = (byte >> (7 - bit % 8)) & 1;

        value = (value << 1) | set;
    }

    value
}

/// The 20 bytes written by the first 32 values of a code.
///
/// Called by: [`RecoveryCode::parse`].
fn bytes_of(values: &[u8]) -> Result<[u8; CODE_BYTES], RecoveryCodeError> {
    ensure!(values.len() >= DATA_CHARS, RecoveryCodeError::Length);

    let mut bytes = [0u8; CODE_BYTES];

    for bit in 0..CODE_BYTES * 8 {
        let value = values.get(bit / 5).copied().ok_or(RecoveryCodeError::Length)?;
        let set = (value >> (4 - bit % 5)) & 1;
        let byte = bytes.get_mut(bit / 8).ok_or(RecoveryCodeError::Length)?;

        *byte |= set << (7 - bit % 8);
    }

    Ok(bytes)
}

/// The value of a typed character, or `None` when no code has it.
///
/// Called by: [`RecoveryCode::parse`].
fn value_of(character: char) -> Option<u8> {
    let upper = character.to_ascii_uppercase();
    let position = ALPHABET.iter().position(|&letter| char::from(letter) == upper)?;

    u8::try_from(position).ok()
}

/// The values as characters, in groups of five joined by dashes.
///
/// Called by: [`RecoveryCode::to_text`].
fn group(values: &[u8]) -> Zeroizing<String> {
    let mut text = Zeroizing::new(String::with_capacity(ALL_CHARS + ALL_CHARS / GROUP));

    for (index, value) in values.iter().enumerate() {
        if index > 0 && index % GROUP == 0 {
            text.push('-');
        }

        let letter = ALPHABET.get(usize::from(*value)).copied().unwrap_or(b'?');
        text.push(char::from(letter));
    }

    text
}

#[cfg(test)]
mod tests {
    //! Unit tests of `recovery_code.rs`.

    use super::*;

    fn code() -> RecoveryCode {
        let mut bytes = [0u8; CODE_BYTES];

        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::try_from(index * 13 % 256).unwrap();
        }

        RecoveryCode::from_bytes(bytes)
    }

    #[test]
    fn a_code_is_seven_groups_of_five_characters_of_its_alphabet() {
        let text = code().to_text();
        let groups: Vec<&str> = text.split('-').collect();

        assert_eq!(groups.len(), 7);
        assert!(groups.iter().all(|group| group.len() == 5));
        assert!(text.chars().all(|c| c == '-' || ALPHABET.contains(&(c as u8))));
        assert!(!text.contains(['0', 'O', '1', 'I']));
    }

    #[test]
    fn a_written_code_reads_back_whatever_its_case_spaces_and_dashes() {
        let text = code().to_text();
        let typed = text.to_lowercase().replace('-', " ");

        assert_eq!(RecoveryCode::parse(&text).unwrap().expose(), code().expose());
        assert_eq!(RecoveryCode::parse(&typed).unwrap().expose(), code().expose());
        assert_eq!(RecoveryCode::parse(&text.replace('-', "")).unwrap().expose(), code().expose());
    }

    #[test]
    fn every_byte_value_survives_the_writing() {
        let all = RecoveryCode::from_bytes([0xFF; CODE_BYTES]);
        let none = RecoveryCode::from_bytes([0; CODE_BYTES]);

        assert_eq!(RecoveryCode::parse(&all.to_text()).unwrap().expose(), &[0xFF; CODE_BYTES]);
        assert_eq!(RecoveryCode::parse(&none.to_text()).unwrap().expose(), &[0; CODE_BYTES]);
    }

    #[test]
    fn a_typing_error_is_found_by_the_check_sum() {
        let text = code().to_text().to_string();
        let first = text.chars().next().unwrap();
        let other = if first == 'A' { 'B' } else { 'A' };
        let changed = format!("{other}{}", &text[1..]);

        assert_eq!(RecoveryCode::parse(&changed).unwrap_err(), RecoveryCodeError::Check);
    }

    #[test]
    fn what_is_no_code_is_refused() {
        let text = code().to_text().to_string();

        assert_eq!(RecoveryCode::parse(&text[..20]).unwrap_err(), RecoveryCodeError::Length);
        assert_eq!(RecoveryCode::parse(&format!("{text}A")).unwrap_err(), RecoveryCodeError::Length);
        assert_eq!(RecoveryCode::parse(&format!("0{}", &text[1..])).unwrap_err(), RecoveryCodeError::Character);
        assert_eq!(RecoveryCode::parse(&"A".repeat(500)).unwrap_err(), RecoveryCodeError::Length);
        assert_eq!(RecoveryCode::parse("").unwrap_err(), RecoveryCodeError::Length);
    }

    #[test]
    fn the_debug_output_does_not_show_the_code() {
        assert_eq!(format!("{:?}", code()), "RecoveryCode(..)");
    }
}
