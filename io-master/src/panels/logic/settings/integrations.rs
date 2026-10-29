//! The integrations with AI services, as the settings of the AI change
//! them: a new one, its data changed, the active one, its API key, the
//! question that checks it.
//!
//! An integration is checked before it is stored: its address is one the
//! program sends to (`https://`, or `http://` on this computer), its CA
//! certificate is one the connection can use. The first integration made
//! becomes the active one.

use zeroize::Zeroizing;

use crate::ai::ask::{Prepared, Question, prepare};
use crate::ai::client::{check_address, check_certificate};
use crate::db::ai::api_key::{MAX_KEY, read_api_key, update_api_key};
use crate::db::ai::integration::{create_integration, read_integration, update_integration};
use crate::db::database::Database;
use crate::db::user::program_data::{read_settings, update_settings};
use crate::model::ai::api_key::ApiKey;
use crate::model::ai::chat::{Message, Speaker};
use crate::model::ai::integration::Integration;
use crate::model::user::program_data::Settings;
use crate::panels::logic::settings::error::SettingsError;

/// The longest name of an integration, in characters.
const MAX_NAME: usize = 100;

/// The smallest context of a model, in tokens: less leaves no room for a
/// question.
const MIN_CONTEXT: u32 = 512;

/// The question that checks an integration.
const CHECK_QUESTION: &str = "Reply with the word OK.";

/// The integration with its text trimmed, checked: a name, an address the
/// program sends to, a model, a context, a CA certificate the connection can
/// use (an empty one is none).
///
/// Called by: [`add_integration`], [`change_integration`].
/// Calls: `check_address`, `check_certificate`.
fn checked(integration: &Integration) -> Result<Integration, SettingsError> {
    let name = integration.name.trim();
    let certificate = integration.ca_certificate.as_deref().unwrap_or_default().trim();

    ensure!(!name.is_empty(), SettingsError::Refused("an integration without a name".to_owned()));
    ensure!(name.chars().count() <= MAX_NAME, SettingsError::Refused(format!("a name over {MAX_NAME} characters")));
    ensure!(!integration.model.trim().is_empty(), SettingsError::Refused("an integration without a model".to_owned()));
    ensure!(
        integration.context_size >= MIN_CONTEXT,
        SettingsError::Refused(format!("a context of fewer than {MIN_CONTEXT} tokens"))
    );
    check_address(integration.url.trim())?;

    if !certificate.is_empty() {
        check_certificate(certificate)?;
    }

    Ok(Integration {
        id: integration.id,
        name: name.to_owned(),
        url: integration.url.trim().to_owned(),
        model: integration.model.trim().to_owned(),
        protocol: integration.protocol,
        ca_certificate: if certificate.is_empty() { None } else { Some(certificate.to_owned()) },
        context_size: integration.context_size,
    })
}

/// Stores a new integration and gives its id; the first one becomes the
/// active one.
///
/// Called by: `panels/views/settings/integrations.rs` (a new integration).
/// Calls: [`checked`], `create_integration`, `read_settings`,
/// `update_settings`.
pub fn add_integration(db: &Database, integration: &Integration) -> Result<i64, SettingsError> {
    ensure!(integration.id == 0, SettingsError::Refused("a new integration has an id already".to_owned()));

    let id = create_integration(db, &checked(integration)?)?;

    if read_settings(db)?.active_integration.is_none() {
        update_settings(db, &Settings { active_integration: Some(id) })?;
    }

    Ok(id)
}

/// Writes the data of the integration `integration.id` anew. Its key stays
/// bound to the old address: a new address needs the key given anew.
///
/// Called by: `panels/views/settings/integrations.rs` (an integration
/// saved).
/// Calls: `read_integration`, [`checked`], `update_integration`.
pub fn change_integration(db: &Database, integration: &Integration) -> Result<(), SettingsError> {
    read_integration(db, integration.id)?;

    Ok(update_integration(db, &checked(integration)?)?)
}

/// Makes the integration `id` the active one: the one asked where no agent
/// names another.
///
/// Called by: `panels/views/settings/integrations.rs`.
/// Calls: `read_integration`, `update_settings`.
pub fn activate_integration(db: &Database, id: i64) -> Result<(), SettingsError> {
    read_integration(db, id)?;

    Ok(update_settings(db, &Settings { active_integration: Some(id) })?)
}

/// Stores the API key of the integration `id`, bound to its present
/// address; a key it had is replaced. The key is trimmed: a key pasted with
/// a line break is the same key.
///
/// Called by: `panels/views/settings/integrations.rs` (a key entered).
/// Calls: `read_integration`, `update_api_key`.
pub fn set_key(db: &Database, id: i64, key: &str) -> Result<(), SettingsError> {
    let key = key.trim();

    ensure!(!key.is_empty(), SettingsError::Refused("an empty API key".to_owned()));
    ensure!(key.len() <= MAX_KEY, SettingsError::Refused(format!("an API key over {MAX_KEY} characters")));

    for character in key.chars() {
        ensure!(
            character.is_ascii_graphic(),
            SettingsError::Refused("an API key with spaces or characters a header cannot carry".to_owned())
        );
    }

    read_integration(db, id)?;

    Ok(update_api_key(db, &ApiKey { integration: id, key: Zeroizing::new(key.to_owned()) })?)
}

/// Whether the integration `id` has a key for its present address. The key
/// itself never leaves `db/` and `ai/`.
///
/// Called by: `panels/views/settings/integrations.rs` (the list).
/// Calls: `read_api_key`.
pub fn has_key(db: &Database, id: i64) -> Result<bool, SettingsError> {
    Ok(read_api_key(db, id)?.is_some())
}

/// The question that checks the integration `id`, prepared: one short
/// message, no instructions, no agent, so it is not kept in a journal
/// (`ai/ask.rs`).
///
/// Called by: `panels/views/settings/integrations.rs` (the check).
/// Calls: `prepare`.
pub fn check_question(db: &Database, id: i64) -> Result<Prepared, SettingsError> {
    let question = Question {
        integration: id,
        agent: None,
        work_unit: None,
        system: String::new(),
        messages: vec![Message { speaker: Speaker::User, text: CHECK_QUESTION.to_owned(), time: 0 }],
    };

    Ok(prepare(db, &question)?)
}

#[cfg(test)]
mod tests {
    //! Tests of the integrations: checked, the first one active, the key
    //! stored and replaced.

    use super::*;

    use crate::ai::error::AiError;
    use crate::db::database::Database;
    use crate::db::user::program_data::create_settings;
    use crate::model::ai::integration::Protocol;
    use crate::test_support::TempDir;
    use crate::test_support::database::{database, integration};

    /// A new database with the settings of a new account (no active
    /// integration), as `create_account` makes it.
    fn database_with_settings() -> (TempDir, Database) {
        let (directory, db) = database();

        create_settings(&db, &Settings { active_integration: None }).unwrap();
        (directory, db)
    }

    #[test]
    fn an_integration_is_checked_and_the_first_one_is_active() {
        let (_directory, db) = database_with_settings();
        let first = add_integration(&db, &Integration { name: "  Local  ".to_owned(), ..integration() }).unwrap();
        let remote = Integration {
            name: "Claude".to_owned(),
            url: "https://api.anthropic.com/v1/messages".to_owned(),
            model: "claude-opus-5-5".to_owned(),
            protocol: Protocol::AnthropicMessages,
            ca_certificate: Some("  ".to_owned()),
            ..integration()
        };
        let second = add_integration(&db, &remote).unwrap();

        assert_eq!(read_integration(&db, first).unwrap().name, "Local");
        assert_eq!(read_integration(&db, second).unwrap().ca_certificate, None);
        assert_eq!(read_settings(&db).unwrap().active_integration, Some(first));

        activate_integration(&db, second).unwrap();
        assert_eq!(read_settings(&db).unwrap().active_integration, Some(second));

        let plain = Integration { url: "http://example.com/v1/chat/completions".to_owned(), ..integration() };
        let foreign = Integration { ca_certificate: Some("no PEM".to_owned()), ..integration() };

        assert!(matches!(add_integration(&db, &plain), Err(SettingsError::Ai(AiError::Address(_)))));
        assert!(matches!(add_integration(&db, &foreign), Err(SettingsError::Ai(AiError::Address(_)))));
        assert!(matches!(
            add_integration(&db, &Integration { name: " ".to_owned(), ..integration() }),
            Err(SettingsError::Refused(_))
        ));
    }

    #[test]
    fn a_key_is_stored_trimmed_and_replaced_and_never_given_back() {
        let (_directory, db) = database_with_settings();
        let id = add_integration(&db, &integration()).unwrap();

        assert!(!has_key(&db, id).unwrap());

        set_key(&db, id, " sk-first\n").unwrap();
        set_key(&db, id, "sk-second").unwrap();

        assert!(has_key(&db, id).unwrap());
        assert_eq!(read_api_key(&db, id).unwrap().unwrap().key.as_str(), "sk-second");
        assert!(matches!(set_key(&db, id, "  "), Err(SettingsError::Refused(_))));
        assert!(matches!(set_key(&db, id, "sk with space"), Err(SettingsError::Refused(_))));
    }
}
