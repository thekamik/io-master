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
use crate::objects::ai::custom_format::custom_format_store::read_custom_format;
use crate::objects::ai::integration::integration_model::{Integration, Protocol};
use crate::objects::ai::integration::integration_rules::address::{check_address, check_certificate};
use crate::objects::ai::integration::integration_store::{
    MAX_CHARACTERS_PER_TOKEN, MAX_CONNECT_TIMEOUT_SECONDS, MAX_STREAM_SILENCE_SECONDS, create_integration,
    read_integration, update_integration,
};
use crate::objects::error::DomainError;
use crate::objects::user::program_data::program_data_model::Settings;
use crate::objects::user::program_data::program_data_store::{read_settings, update_settings};

/// The longest name of an integration, in characters.
const MAX_NAME: usize = 100;

/// The smallest context of a model, in tokens: less leaves no room for a
/// question.
pub const MIN_CONTEXT: u32 = 512;

/// Where a new integration starts, whatever its protocol (user, 2026-11-02:
/// the program gives them, the page none): three characters of a token,
/// fewer than most models use (about four for English), so that the estimate
/// errs on the side of a question refused; 15 seconds for a connection to
/// open; 300 seconds of silence for a local model that thinks long.
const NEW_CHARACTERS_PER_TOKEN: u32 = 3;
const NEW_CONNECT_TIMEOUT_SECONDS: u32 = 15;
const NEW_STREAM_SILENCE_SECONDS: u32 = 300;

/// The protocol a new integration starts with: a model on this computer.
pub const NEW_PROTOCOL: Protocol = Protocol::OpenAiChat;

/// Where a new integration of the chat completions starts: Ollama on this
/// computer, a model of a context of 128 000 tokens (an ST of 100 pages
/// whole: `objects/ai/integration/integration_ai/question.rs`:
/// `needed_context`).
const NEW_OPENAI_NAME: &str = "Ollama on this computer";
const NEW_OPENAI_URL: &str = "http://127.0.0.1:11434/v1/chat/completions";
const NEW_OPENAI_MODEL: &str = "llama3.1";
const NEW_OPENAI_CONTEXT: u32 = 128_000;

/// Where a new integration of the messages of Anthropic starts: the model
/// the laboratory tests with (user, 2026-11-03), its context of 1 000 000
/// tokens as Anthropic gives it.
const NEW_ANTHROPIC_NAME: &str = "Anthropic";
const NEW_ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const NEW_ANTHROPIC_MODEL: &str = "claude-haiku-5-5";
const NEW_ANTHROPIC_CONTEXT: u32 = 1_000_000;

/// The name a new integration with a custom service starts from.
const NEW_CUSTOM_NAME: &str = "Custom service";

/// Where a new integration of the protocol `protocol` starts: the name,
/// address, model and context its service most often has, and the estimate
/// and the waits of every integration; the form shows them to be corrected.
///
/// Input: `protocol`, the protocol chosen.
/// Output: the integration, not stored (`Integration { id: 0, name:
/// "Anthropic", model: "claude-haiku-5-5", context_size: 1000000, .. }`).
///
/// Called by: `panels/views/settings/integrations.rs`: `integration_form`.
pub fn new_integration(protocol: Protocol) -> Integration {
    let (name, url, model, context_size) = match protocol {
        Protocol::OpenAiChat => (NEW_OPENAI_NAME, NEW_OPENAI_URL, NEW_OPENAI_MODEL, NEW_OPENAI_CONTEXT),
        Protocol::AnthropicMessages => {
            (NEW_ANTHROPIC_NAME, NEW_ANTHROPIC_URL, NEW_ANTHROPIC_MODEL, NEW_ANTHROPIC_CONTEXT)
        }
        // A custom service has no usual address or model: the user gives both.
        Protocol::Custom => (NEW_CUSTOM_NAME, "", "", NEW_OPENAI_CONTEXT),
    };

    Integration {
        id: 0,
        name: name.to_owned(),
        url: url.to_owned(),
        model: model.to_owned(),
        protocol,
        custom_format: None,
        ca_certificate: None,
        context_size,
        characters_per_token: NEW_CHARACTERS_PER_TOKEN,
        connect_timeout_seconds: NEW_CONNECT_TIMEOUT_SECONDS,
        stream_silence_seconds: NEW_STREAM_SILENCE_SECONDS,
    }
}

/// The integration with its text trimmed, checked: a name, an address the
/// program sends to, a model, a context, the characters of a token, a CA
/// certificate the connection can use (an empty one is none).
///
/// Input: `integration`, the integration as given by the page.
/// Output: the integration to write; `DomainError::Refused` for an empty or
/// too long name, no model, a context under [`MIN_CONTEXT`], characters of a
/// token out of 1 to `MAX_CHARACTERS_PER_TOKEN`, a wait out of its range, an
/// address or a certificate refused.
///
/// Called by: [`add_integration`], [`change_integration`].
/// Calls: `objects/ai/integration/integration_rules/address.rs`: `check_address`, `check_certificate`;
/// `objects/ai/custom_format/custom_format_store.rs`: `read_custom_format`.
fn checked(db: &UserDatabase, integration: &Integration) -> Result<Integration, DomainError> {
    let name = integration.name.trim();
    let certificate = integration.ca_certificate.as_deref().unwrap_or_default().trim();

    ensure!(!name.is_empty(), DomainError::Refused("an integration without a name".to_owned()));
    ensure!(name.chars().count() <= MAX_NAME, DomainError::Refused(format!("a name over {MAX_NAME} characters")));
    ensure!(!integration.model.trim().is_empty(), DomainError::Refused("an integration without a model".to_owned()));
    ensure!(
        integration.context_size >= MIN_CONTEXT,
        DomainError::Refused(format!("a context of fewer than {MIN_CONTEXT} tokens"))
    );
    ensure!(
        (1..=MAX_CHARACTERS_PER_TOKEN).contains(&integration.characters_per_token),
        DomainError::Refused(format!("the characters of a token: from 1 to {MAX_CHARACTERS_PER_TOKEN}"))
    );
    ensure!(
        (1..=MAX_CONNECT_TIMEOUT_SECONDS).contains(&integration.connect_timeout_seconds),
        DomainError::Refused(format!("the wait for a connection: from 1 to {MAX_CONNECT_TIMEOUT_SECONDS} seconds"))
    );
    ensure!(
        (1..=MAX_STREAM_SILENCE_SECONDS).contains(&integration.stream_silence_seconds),
        DomainError::Refused(format!("the silence of a stream: from 1 to {MAX_STREAM_SILENCE_SECONDS} seconds"))
    );
    check_address(integration.url.trim())?;

    // A custom service is spoken with by a stored format; the two protocols
    // of the program name none.
    match (integration.protocol, integration.custom_format) {
        (Protocol::Custom, Some(format)) => {
            read_custom_format(db, format)?;
        }
        (Protocol::Custom, None) => {
            return Err(DomainError::Refused("the custom protocol needs a custom format".to_owned()));
        }
        (Protocol::OpenAiChat | Protocol::AnthropicMessages, format) => {
            ensure!(
                format.is_none(),
                DomainError::Refused("only the custom protocol names a custom format".to_owned())
            );
        }
    }

    if !certificate.is_empty() {
        check_certificate(certificate)?;
    }

    Ok(Integration {
        id: integration.id,
        name: name.to_owned(),
        url: integration.url.trim().to_owned(),
        model: integration.model.trim().to_owned(),
        protocol: integration.protocol,
        custom_format: integration.custom_format,
        ca_certificate: if certificate.is_empty() { None } else { Some(certificate.to_owned()) },
        context_size: integration.context_size,
        characters_per_token: integration.characters_per_token,
        connect_timeout_seconds: integration.connect_timeout_seconds,
        stream_silence_seconds: integration.stream_silence_seconds,
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

    let id = create_integration(db, &checked(db, integration)?)?;

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
/// Calls: `objects/ai/integration/integration_store.rs`: `read_integration`, `update_integration`; [`checked`].
pub fn change_integration(db: &UserDatabase, integration: &Integration) -> Result<(), DomainError> {
    read_integration(db, integration.id)?;

    Ok(update_integration(db, &checked(db, integration)?)?)
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

    use crate::objects::ai::custom_format::custom_format_store::create_custom_format;
    use crate::objects::user::program_data::program_data_store::create_settings;
    use crate::test_support::TempDir;
    use crate::test_support::database::{custom_format, database, integration};

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
        assert!(matches!(
            add_integration(&db, &Integration { characters_per_token: 0, ..integration() }),
            Err(DomainError::Refused(_))
        ));
    }

    #[test]
    fn a_new_integration_starts_from_its_protocol_and_is_accepted() {
        let (_directory, db) = database();
        let anthropic = new_integration(Protocol::AnthropicMessages);

        assert_eq!((anthropic.model.as_str(), anthropic.context_size), ("claude-haiku-5-5", 1_000_000));
        assert!(checked(&db, &anthropic).is_ok() && checked(&db, &new_integration(NEW_PROTOCOL)).is_ok());
    }

    #[test]
    fn the_custom_protocol_names_a_stored_format_and_the_others_none() {
        let (_directory, db) = database_with_settings();
        let format = create_custom_format(&db, &custom_format()).unwrap();
        let custom = Integration {
            name: "The institute".to_owned(),
            url: "https://ai.institute.example/api/ask".to_owned(),
            model: "institute-7".to_owned(),
            protocol: Protocol::Custom,
            custom_format: Some(format),
            ..integration()
        };
        let id = add_integration(&db, &custom).unwrap();

        assert_eq!(read_integration(&db, id).unwrap().custom_format, Some(format));

        for refused in [
            Integration { custom_format: None, ..custom.clone() },
            Integration { custom_format: Some(99), name: "Other".to_owned(), ..custom.clone() },
            Integration { custom_format: Some(format), ..integration() },
        ] {
            assert!(add_integration(&db, &refused).is_err());
        }

        assert_eq!(new_integration(Protocol::Custom).custom_format, None);
    }
}
