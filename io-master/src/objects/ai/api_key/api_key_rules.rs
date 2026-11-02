//! The API key of an integration, as the settings of the AI change it: a
//! key given, bound to the present address of its integration, and whether
//! one is stored. The key itself never leaves the store and the AI of the
//! integration.

use zeroize::Zeroizing;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::api_key::api_key_model::ApiKey;
use crate::objects::ai::api_key::api_key_store::{MAX_KEY, read_api_key, update_api_key};
use crate::objects::ai::integration::integration_store::read_integration;
use crate::objects::error::DomainError;

/// Stores the API key of the integration `id`, bound to its present
/// address; a key it had is replaced. The key is trimmed: a key pasted with
/// a line break is the same key.
///
/// Input: `id`, the id of a stored integration; `key`, the key as typed.
/// Output: nothing; `DomainError::Refused` for an empty key, one over
/// `MAX_KEY` or with a character a header cannot carry; an error of the
/// database for an integration missing.
///
/// Called by: `panels/views/settings/integrations.rs`: `integration_set_key`.
/// Calls: `objects/ai/integration/integration_store.rs`: `read_integration`;
/// `objects/ai/api_key/api_key_store.rs`: `update_api_key`.
pub fn set_key(db: &UserDatabase, id: i64, key: &str) -> Result<(), DomainError> {
    let key = key.trim();

    ensure!(!key.is_empty(), DomainError::Refused("an empty API key".to_owned()));
    ensure!(key.len() <= MAX_KEY, DomainError::Refused(format!("an API key over {MAX_KEY} characters")));

    for character in key.chars() {
        ensure!(
            character.is_ascii_graphic(),
            DomainError::Refused("an API key with spaces or characters a header cannot carry".to_owned())
        );
    }

    read_integration(db, id)?;

    Ok(update_api_key(db, &ApiKey { integration: id, key: Zeroizing::new(key.to_owned()) })?)
}

/// Whether the integration `id` has a key for its present address. The key
/// itself never leaves the store and the AI of the integration.
///
/// Called by: `panels/views/settings/integrations.rs`: `integration_list`.
/// Calls: `objects/ai/api_key/api_key_store.rs`: `read_api_key`.
pub fn has_key(db: &UserDatabase, id: i64) -> Result<bool, DomainError> {
    Ok(read_api_key(db, id)?.is_some())
}

#[cfg(test)]
mod tests {
    //! Tests of the key: stored trimmed, replaced, never given back.

    use super::*;

    use crate::objects::ai::integration::integration_store::create_integration;
    use crate::test_support::database::{database, integration};

    #[test]
    fn a_key_is_stored_trimmed_and_replaced_and_never_given_back() {
        let (_directory, db) = database();
        let id = create_integration(&db, &integration()).unwrap();

        assert!(!has_key(&db, id).unwrap());

        set_key(&db, id, " sk-first\n").unwrap();
        set_key(&db, id, "sk-second").unwrap();

        assert!(has_key(&db, id).unwrap());
        assert_eq!(read_api_key(&db, id).unwrap().unwrap().key.as_str(), "sk-second");
        assert!(matches!(set_key(&db, id, "  "), Err(DomainError::Refused(_))));
        assert!(matches!(set_key(&db, id, "sk with space"), Err(DomainError::Refused(_))));
    }
}
