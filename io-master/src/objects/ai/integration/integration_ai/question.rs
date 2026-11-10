//! The first step of the way to the AI (user, 2026-10-29), under the lock
//! of the session: a question read with its integration and its key, made
//! into the body of the protocol of the integration, and checked that it
//! fits into the context of the model beside its answer.
//!
//! ```text
//! Question ──► prepare ──► Prepared ──► sending.rs: send ──► Sent ──► journal_ai.rs: record ──► Answer
//!               ├► read_integration, read_api_key       (the integration and its key)
//!               ├► check_address, turns, check_size      (refused, never shortened)
//!               └► protocol.rs: body                     (the body of the request)
//! Prepared, the tokens the service counted ──► shortened_warning   (a question it shortened in silence;
//!                                                                    journal_ai.rs: record, Answer::shortened)
//! ```
//!
//! The steps are apart because the database is not used while the answer
//! is awaited: the caller holds the lock of the session (and so the
//! database) only for `prepare` and `record`, so that the other commands
//! of the window are not held up by a long answer.

use std::time::Duration;

use zeroize::Zeroizing;

use crate::infrastructure::clock::now;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::api_key::api_key_store::read_api_key;
use crate::objects::ai::chat::chat_model::{Message, Speaker};
use crate::objects::ai::integration::integration_ai::connection::Request;
use crate::objects::ai::integration::integration_ai::protocol::{body, needs_key};
use crate::objects::ai::integration::integration_ai::turns::{Turn, turns};
use crate::objects::ai::integration::integration_model::Integration;
use crate::objects::ai::integration::integration_rules::address::check_address;
use crate::objects::ai::integration::integration_store::read_integration;
use crate::objects::ai_error::AiError;

/// The answer [`needed_context`] counts beside an ST of 100 pages, in tokens:
/// the longest answer of a new agent (`objects/ai/agent/agent_rules.rs`:
/// `NEW_ANSWER_TOKENS`). A question asks for the longest answer of its agent
/// (`Agent::max_answer_tokens`), never less.
pub const MAX_ANSWER_TOKENS: u32 = 16_000;

/// The most messages of one question.
pub const MAX_MESSAGES: usize = 1000;

/// The characters of a page of a document, as the context a model needs is
/// counted: generous, a page of an ST holds about 2200 (user, 2026-11-02).
const PAGE_CHARACTERS: u64 = 3000;

/// The pages of the documents one question of an assessment should hold
/// whole: an ST of 100 pages (user, 2026-11-02).
const DOCUMENT_PAGES: u64 = 100;

/// The pages beside the documents: the requirement of the standard (at most
/// one page) and the knowledge of the agent (about one page).
const OTHER_PAGES: u64 = 2;

/// The tokens of the instructions of an agent and of the task of an
/// assessment, beside the pages.
const TASK_TOKENS: u64 = 2000;

/// How many times fewer tokens than the estimate the service must count for
/// [`shortened_warning`] to warn: the estimate is rough (a tokenizer counts
/// from two to five characters a token), a question cut to a small context
/// is counted many times lower.
const SHORTENED_RATIO: u64 = 2;

/// The context a model needs for a question of [`DOCUMENT_PAGES`] and
/// [`OTHER_PAGES`] pages and its longest answer, in tokens, counted as
/// [`prepare`] counts, by the characters of a token of the integration.
///
/// Input: `integration`, an integration, its `characters_per_token` checked
/// when it was written (1 to 10).
/// Output: the tokens (`Some(120000)` for three characters of a token; the
/// integrations are made with a context of 128 000 and three characters of a
/// token by default, the page: `ui/src/views/settings/IntegrationForm.svelte`);
/// `None` for no characters of a token (a damaged integration).
///
/// Called by: [`context_warning`].
fn needed_context(integration: &Integration) -> Option<u64> {
    let pages =
        ((DOCUMENT_PAGES + OTHER_PAGES) * PAGE_CHARACTERS).checked_div(u64::from(integration.characters_per_token))?;

    Some(pages + TASK_TOKENS + u64::from(MAX_ANSWER_TOKENS))
}

/// A question to ask.
pub struct Question {
    /// The integration asked.
    pub integration: i64,
    /// The agent that asks; `None` without one.
    pub agent: Option<i64>,
    /// The work unit the question is for; `None` outside an evaluation.
    pub work_unit: Option<i64>,
    /// The instructions: who the model is and how it answers.
    pub system: String,
    /// The conversation; its last message is the user's.
    pub messages: Vec<Message>,
    /// The longest answer asked for, in tokens: that of the agent, or that
    /// of the check of an integration; asked whole, never lowered.
    pub max_answer_tokens: u32,
}

/// A question ready to be sent, with what its entry of the journal needs.
pub struct Prepared {
    pub(in crate::objects::ai) request: Request,
    /// The name of the integration asked ("Ollama"), for the journal.
    pub(in crate::objects::ai) integration_name: String,
    /// The model asked, for an answer that does not name its own.
    pub(in crate::objects::ai) model: String,
    pub(in crate::objects::ai) agent: Option<i64>,
    pub(in crate::objects::ai) work_unit: Option<i64>,
    pub(in crate::objects::ai) time: i64,
    /// The tokens of the question as [`check_size`] estimated them, to be
    /// held against what the service counted ([`shortened_warning`]).
    pub(in crate::objects::ai) estimated_tokens: u64,
}

/// Refuses a question that does not fit into the context of the model
/// beside its whole answer (`limit`, the agent's, never lowered in silence).
///
/// The tokens are counted by the service, each model by its own tokenizer;
/// the program only estimates them, by `Integration::characters_per_token`,
/// as a guard before the question is sent (user, 2026-11-02): a local
/// server (Ollama, llama.cpp) shortens a question over its context in
/// silence, and the model would judge a document it did not read whole. A
/// question that does not fit is refused here instead, and the caller gives
/// the documents as their passages or leaves the oldest messages of a chat
/// out.
///
/// Input: `integration`, the integration asked; `system`, the instructions;
/// `turns`, the conversation; `limit`, the longest answer asked for.
/// Output: the estimate when it fits (`9000`); `AiError::TooLong` with the
/// estimate and the room (`{ estimated: 9000, room: 6000 }`) when it does not.
///
/// Called by: [`prepare`].
fn check_size(integration: &Integration, system: &str, turns: &[Turn], limit: u32) -> Result<u64, AiError> {
    let mut characters = system.chars().count();

    for turn in turns {
        characters += turn.text.chars().count();
    }

    // The ratio was checked when the integration was written (1 to 10); a
    // zero would be a damaged integration, refused rather than divided by.
    ensure!(
        integration.characters_per_token > 0,
        AiError::Refused(format!("the integration {} has no characters of a token", integration.name))
    );

    let estimated = u64::try_from(characters).unwrap_or(u64::MAX).div_ceil(u64::from(integration.characters_per_token));
    let room = u64::from(integration.context_size.saturating_sub(limit));

    ensure!(estimated <= room, AiError::TooLong { estimated, room });

    Ok(estimated)
}

/// Prepares the question `question`: reads its integration and its key,
/// makes the body in the protocol of the integration and checks that it
/// fits. Runs under the lock of the session.
///
/// Input: `question`, the question, its last message the user's.
/// Output: the question ready to be sent, with what its entry of the journal
/// needs; `AiError::TooManyMessages` over [`MAX_MESSAGES`],
/// `AiError::NoQuestion` with nothing to ask, `AiError::NoKey` when the
/// protocol needs a key the integration has not, `AiError::TooLong` when it
/// does not fit, an error for an address refused or a database failed.
///
/// Called by: `objects/ai/chat/chat_ai.rs`: `prepare_shortened`;
/// `objects/evaluation/work_unit/work_unit_ai/assessment.rs`: `prepared_assessment`;
/// `objects/ai/integration/integration_ai/check.rs`: `check_question`;
/// `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `prepared_question`.
/// Calls: `objects/ai/integration/integration_store.rs`: `read_integration`; `objects/ai/api_key/api_key_store.rs`:
/// `read_api_key`; `objects/ai/integration/integration_rules/address.rs`: `check_address`;
/// `objects/ai/integration/integration_ai/turns.rs`: `turns`; [`check_size`];
/// `objects/ai/integration/integration_ai/protocol.rs`: `needs_key`, `body`; `infrastructure/clock.rs`: `now`.
pub fn prepare(db: &UserDatabase, question: &Question) -> Result<Prepared, AiError> {
    ensure!(
        question.messages.len() <= MAX_MESSAGES,
        AiError::TooManyMessages { count: question.messages.len(), max: MAX_MESSAGES }
    );

    let turns = turns(&question.messages);

    ensure!(matches!(turns.last(), Some(last) if last.speaker == Speaker::User), AiError::NoQuestion);
    ensure!(question.max_answer_tokens > 0, AiError::Refused("a question that asks for no answer".to_owned()));

    let integration = read_integration(db, question.integration)?;

    check_address(&integration.url)?;

    let key = read_api_key(db, integration.id)?;

    ensure!(key.is_some() || !needs_key(integration.protocol), AiError::NoKey);
    let estimated_tokens = check_size(&integration, &question.system, &turns, question.max_answer_tokens)?;

    let time = now().map_err(|error| AiError::Refused(error.to_string()))?;
    let body = body(integration.protocol, &integration.model, &question.system, &turns, question.max_answer_tokens);
    let request = Request {
        url: integration.url,
        ca_certificate: integration.ca_certificate,
        protocol: integration.protocol,
        key: key.map(|key| Zeroizing::new(key.key.as_str().to_owned())),
        body: body.to_string(),
        connect_timeout: Duration::from_secs(u64::from(integration.connect_timeout_seconds)),
        stream_silence: Duration::from_secs(u64::from(integration.stream_silence_seconds)),
    };

    Ok(Prepared {
        request,
        integration_name: integration.name,
        model: integration.model,
        agent: question.agent,
        work_unit: question.work_unit,
        time,
        estimated_tokens,
    })
}

/// The warning about a question the service seems to have shortened: it
/// counted fewer than half the tokens the program estimated. A local server
/// (Ollama, llama.cpp) cuts a question to its own context in silence, which
/// may be smaller than the one written in the integration; the model then
/// judged a document it did not read whole. Only what the service counts
/// can show it, after the answer.
///
/// Input: `prepared`, the question as it was sent; `counted`, the tokens of
/// the question as the service counted them (`Reading::input_tokens`), 0
/// when it did not.
/// Output: the warning ("The service counted 4096 tokens of a question
/// estimated at 30000 ..."); `None` when the service did not count, or
/// counted at least half the estimate.
///
/// Called by: `objects/ai/journal/journal_ai.rs`: `record`.
pub fn shortened_warning(prepared: &Prepared, counted: u32) -> Option<String> {
    let counted = u64::from(counted);
    let estimated = prepared.estimated_tokens;

    if counted == 0 || counted.saturating_mul(SHORTENED_RATIO) >= estimated {
        return None;
    }

    Some(format!(
        "The service counted {counted} tokens of a question estimated at {estimated}: it may have shortened the \
         question to a context smaller than that of the integration {}, so the model may not have read all it was \
         given. Check the context of the model on its server and in the AI settings.",
        prepared.integration_name
    ))
}

/// The warning about a model whose context is too small for the documents
/// of an evaluation: how many pages it holds, and what to do. The questions
/// still go: their documents as the passages that fit each work unit.
///
/// Input: `integration`, an integration of the user.
/// Output: the warning ("The context of Ollama (32768 tokens) holds about 15
/// pages ..."); `None` when the context holds [`needed_context`].
///
/// Called by: `panels/views/settings/integrations.rs`: `integration_list`,
/// `integration_check`.
/// Calls: [`needed_context`].
pub fn context_warning(integration: &Integration) -> Option<String> {
    let context = u64::from(integration.context_size);
    let Some(needed) = needed_context(integration) else {
        return Some(format!("The integration {} counts no characters of a token: set them.", integration.name));
    };

    if context >= needed {
        return None;
    }

    // The pages left after the answer, the task, the requirement and the
    // knowledge: the pages of the documents the model can read whole.
    let room = context.saturating_sub(u64::from(MAX_ANSWER_TOKENS) + TASK_TOKENS);
    let pages = (room * u64::from(integration.characters_per_token) / PAGE_CHARACTERS).saturating_sub(OTHER_PAGES);

    Some(format!(
        "The context of {} ({context} tokens) holds about {pages} pages of documents; an ST of {DOCUMENT_PAGES} pages needs \
         {needed}. Larger documents go to the model as their passages that fit each work unit, so it may miss \
         what it needs: split the documents into smaller parts, or choose a model with a larger context.",
        integration.name
    ))
}

#[cfg(test)]
mod tests {
    //! Tests of the question prepared: its size and its last message, and
    //! the warning about a question the service shortened.

    use super::*;

    use crate::objects::ai::integration::integration_store::create_integration;
    use crate::test_support::database::{database, integration};

    #[test]
    fn a_context_too_small_for_an_st_of_100_pages_is_warned_of() {
        let small = Integration { context_size: 32_768, ..integration() };
        let warning = context_warning(&small).unwrap();

        assert!(warning.contains("(32768 tokens) holds about 12 pages") && warning.contains("needs 120000"));
        assert!(context_warning(&Integration { context_size: 128_000, ..integration() }).is_none());
        assert_eq!(needed_context(&integration()), Some(120_000));
        assert_eq!(needed_context(&Integration { characters_per_token: 4, ..integration() }), Some(94_500));
    }

    /// A question without an agent to the integration `integration`.
    fn question(integration: i64, text: &str) -> Question {
        Question {
            integration,
            agent: None,
            work_unit: Some(7),
            system: "Be brief.".to_owned(),
            messages: vec![Message { speaker: Speaker::User, text: text.to_owned(), time: 0 }],
            max_answer_tokens: 100,
        }
    }

    #[test]
    fn a_question_the_service_counted_far_shorter_is_warned_of() {
        let (_directory, db) = database();
        let id = create_integration(&db, &Integration { context_size: 400, ..integration() }).unwrap();
        // 889 characters with the instructions: 297 tokens estimated.
        let prepared = prepare(&db, &question(id, &"x".repeat(880))).unwrap();

        assert!(
            shortened_warning(&prepared, 148).unwrap().contains("counted 148 tokens of a question estimated at 297")
        );
        assert!(shortened_warning(&prepared, 149).is_none());
        // A service that counts no tokens tells nothing.
        assert!(shortened_warning(&prepared, 0).is_none());
    }

    #[test]
    fn a_question_that_does_not_fit_or_asks_nothing_is_refused_before_it_is_sent() {
        let (_directory, db) = database();
        let small = Integration { context_size: 400, ..integration() };
        let id = create_integration(&db, &small).unwrap();

        // 400 tokens: 100 for the answer, 300 for the question (900
        // characters at most).
        assert!(prepare(&db, &question(id, &"x".repeat(880))).is_ok());
        assert!(matches!(prepare(&db, &question(id, &"x".repeat(1000))), Err(AiError::TooLong { room: 300, .. })));

        let mut nothing = question(id, " ");

        assert!(matches!(prepare(&db, &nothing), Err(AiError::NoQuestion)));

        nothing.messages.push(Message { speaker: Speaker::Ai, text: "an answer".to_owned(), time: 0 });
        assert!(matches!(prepare(&db, &nothing), Err(AiError::NoQuestion)));
        let mut many = question(id, "x");

        many.messages = vec![Message { speaker: Speaker::User, text: "x".to_owned(), time: 0 }; MAX_MESSAGES + 1];
        assert!(matches!(
            prepare(&db, &many),
            Err(AiError::TooManyMessages { count, max: MAX_MESSAGES }) if count == MAX_MESSAGES + 1
        ));
    }
}
