//! The integrations with AI services, as the settings of the AI change
//! them: a new one, its data changed, the active one. Its API key is a rule
//! of the key (`objects/ai/api_key/api_key_rules.rs`), the question that
//! checks it the AI of the integration (`integration_ai/check.rs`).
//!
//! An integration is checked before it is stored: its address is one the
//! program sends to (`https://`, or `http://` on this computer), its CA
//! certificate is one the connection can use (`address.rs`). The first
//! integration made becomes the active one.

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::integration::integration_model::Integration;
use crate::objects::ai::integration::integration_rules::address::{check_address, check_certificate};
use crate::objects::ai::integration::integration_store::{create_integration, read_integration, update_integration};
use crate::objects::error::DomainError;
use crate::objects::user::program_data::program_data_model::Settings;
use crate::objects::user::program_data::program_data_store::{read_settings, update_settings};

/// The longest name of an integration, in characters.
const MAX_NAME: usize = 100;

/// The smallest context of a model, in tokens: less leaves no room for a
/// question.
const MIN_CONTEXT: u32 = 512;

/// The integration with its text trimmed, checked: a name, an address the
/// program sends to, a model, a context, a CA certificate the connection can
/// use (an empty one is none).
///
/// Input: `integration`, the integration as given by the page.
/// Output: the integration to write; `DomainError::Refused` for an empty or
/// too long name, no model, a context under [`MIN_CONTEXT`], an address or a
/// certificate refused.
///
/// Called by: [`add_integration`], [`change_integration`].
/// Calls: `objects/ai/integration/integration_rules/address.rs`: `check_address`,
/// `check_certificate`.
fn checked(integration: &Integration) -> Result<Integration, DomainError> {
    let name = integration.name.trim();
    let certificate = integration.ca_certificate.as_deref().unwrap_or_default().trim();

    ensure!(!name.is_empty(), DomainError::Refused("an integration without a name".to_owned()));
    ensure!(name.chars().count() <= MAX_NAME, DomainError::Refused(format!("a name over {MAX_NAME} characters")));
    ensure!(!integration.model.trim().is_empty(), DomainError::Refused("an integration without a model".to_owned()));
    ensure!(
        integration.context_size >= MIN_CONTEXT,
        DomainError::Refused(format!("a context of fewer than {MIN_CONTEXT} tokens"))
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

/// Stores a new integration; the first one becomes the active one.
///
/// Input: `integration`, the new integration, its id 0.
/// Output: its id (`2`); `DomainError::Refused` for an id given or what
/// [`checked`] refuses, an error of the database.
///
/// Called by: `panels/views/settings/integrations.rs`: `integration_create`.
/// Calls: [`checked`]; `objects/ai/integration/integration_store.rs`: `create_integration`;
/// `objects/user/program_data/program_data_store.rs`: `read_settings`, `update_settings`.
pub fn add_integration(db: &UserDatabase, integration: &Integration) -> Result<i64, DomainError> {
    ensure!(integration.id == 0, DomainError::Refused("a new integration has an id already".to_owned()));

    let id = create_integration(db, &checked(integration)?)?;

    if read_settings(db)?.active_integration.is_none() {
        update_settings(db, &Settings { active_integration: Some(id) })?;
    }

    Ok(id)
}

/// Writes the data of the integration `integration.id` anew. Its key stays
/// bound to the old address: a new address needs the key given anew.
///
/// Input: `integration`, the integration as changed, its id that of a stored
/// one.
/// Output: nothing; `DomainError::Refused` for what [`checked`] refuses, an
/// error of the database for an integration missing.
///
/// Called by: `panels/views/settings/integrations.rs`: `integration_save`.
/// Calls: `objects/ai/integration/integration_store.rs`: `read_integration`,
/// `update_integration`; [`checked`].
pub fn change_integration(db: &UserDatabase, integration: &Integration) -> Result<(), DomainError> {
    read_integration(db, integration.id)?;

    Ok(update_integration(db, &checked(integration)?)?)
}

/// Makes the integration `id` the active one: the one asked where no agent
/// names another.
///
/// Called by: `panels/views/settings/integrations.rs`: `integration_activate`.
/// Calls: `objects/ai/integration/integration_store.rs`: `read_integration`;
/// `objects/user/program_data/program_data_store.rs`: `update_settings`.
pub fn activate_integration(db: &UserDatabase, id: i64) -> Result<(), DomainError> {
    read_integration(db, id)?;

    Ok(update_settings(db, &Settings { active_integration: Some(id) })?)
}

#[cfg(test)]
mod tests {
    //! Tests of the integrations: checked, the first one active.

    use super::*;

    use crate::objects::ai::integration::integration_model::Protocol;
    use crate::objects::user::program_data::program_data_store::create_settings;
    use crate::test_support::TempDir;
    use crate::test_support::database::{database, integration};

    /// A new database with the settings of a new account (no active
    /// integration), as `create_account` makes it.
    fn database_with_settings() -> (TempDir, UserDatabase) {
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

        let refused_address = add_integration(&db, &plain);
        let refused_certificate = add_integration(&db, &foreign);

        assert!(matches!(refused_address, Err(DomainError::Refused(reason)) if reason.starts_with("the address")));
        assert!(matches!(refused_certificate, Err(DomainError::Refused(reason)) if reason.contains("CA certificate")));
        assert!(matches!(
            add_integration(&db, &Integration { name: " ".to_owned(), ..integration() }),
            Err(DomainError::Refused(_))
        ));
    }
}
