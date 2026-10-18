//! The values OSCAL defines by a pattern, made by the program: a UUID
//! (version 4, from random bytes the caller draws), a date and time
//! with its time zone (RFC 3339, UTC), and a token (an id of a control
//! or a group: a name that starts with a letter or `_`).

/// A UUID of version 4 from 16 random bytes (RFC 4122): the version
/// and the variant bits are set, the rest is the bytes.
pub fn uuid_v4(mut bytes: [u8; 16]) -> String {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();

    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

/// The date and time of `seconds` since 1970, in UTC, as OSCAL writes
/// it: `2026-10-18T13:10:00Z`.
pub fn date_time(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);

    // The civil date of a count of days since 1970 (H. Hinnant).
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// A token of OSCAL made from `text` (an identifier of a standard such
/// as "FAU_GEN.1" or "5.1-1"): letters, digits, `.`, `-` and `_` kept,
/// other characters become `_`, and a token that would not start with
/// a letter or `_` gets the prefix `id_`. Empty text
/// gives `_`.
pub fn token(text: &str) -> String {
    let mut token: String = text
        .trim()
        .chars()
        .map(|character| if character.is_alphanumeric() || ".-_".contains(character) { character } else { '_' })
        .collect();

    if !token.chars().next().is_some_and(|first| first.is_alphabetic() || first == '_') {
        token.insert_str(0, "id_");
    }

    token
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "values_tests.rs"]
mod tests;
