//! Instructions for a chatbot: help to fill one field of an AI
//! connection (step 15 of `docs/todo.md`).
//!
//! The evaluator presses "AI help" at a field of the tab AI of Settings,
//! copies the instruction into a chatbot of their choice, and types
//! the value the chatbot proposes. **Nothing comes back into the
//! program on its own**: the evaluator reads the answer and fills the
//! field by hand, and the rules of `connection.rs` still check it.
//!
//! An instruction holds:
//!
//! ```text
//! the context      what IO Master and a connection are
//! the field        what it means, its rules, an example
//! the connection   the fields that matter for this one, as they are now
//! the service      a place for the evaluator to describe their service
//! the answer       the value only, and never an API key
//! ```
//!
//! **No secret goes into an instruction**: no API key (it is never in
//! the form), and no value of a header, because a header may hold a
//! key typed by mistake instead of `{{api_key}}`: only the names of
//! the headers and the placeholders in them are shown, and the values
//! of the headers that never hold a secret ([`PLAIN_HEADERS`]). The CA
//! certificate is left out as well (long, and of no use here).

use super::connection::{
    API_KEY, AnswerMode, Connection, DEFAULT_MESSAGE_TEMPLATE, Field, MESSAGES, MODEL, RESERVED_HEADERS, ROLE,
    SYSTEM, TEXT,
};

/// Headers whose values never hold a secret, so they are shown.
const PLAIN_HEADERS: [&str; 2] = ["content-type", "accept"];

/// What every instruction starts with.
const CONTEXT: &str = "I am setting up a connection to an AI service in IO Master, a desktop tool for \
Common Criteria evaluators. IO Master knows no AI provider: each connection describes the service \
completely, field by field (the endpoint, the HTTP request, and where the program finds the text and \
other values in the answer). Please help me fill in one field of it.";

/// How a path is written; shared by every path field.
const PATH_SYNTAX: &str = "A path is written like this: field names separated by dots, and positions in a \
list in brackets counted from 0, or [*] for every element: \"choices[0].delta.content\", \
\"content[*].text\", \"[0].generated_text\" for an answer that is a list. The texts found by [*] are \
joined. A path that finds nothing is no error: the value is simply not there in that piece of the \
answer.";

/// What every instruction ends with.
const ANSWER: &str = "Answer with the value of this field only, ready to paste (in a code block), and \
then explain it in two or three sentences. If the documentation of the service leaves the value open, \
say so instead of guessing. Never ask me for my API key and never write a key into the value: the key \
is entered separately in IO Master and stands in a header as {{api_key}}.";

/// What the evaluator fills in about their service.
const SERVICE: &str = "[Describe your service here: its name and version (for example Ollama, vLLM, \
llama.cpp, a cloud service), how it is reached, and, if you have it, a link to its API documentation \
or an example of a request and an answer.]";

/// The explanation of one field: its title, its meaning, its rules,
/// an example.
struct Explanation {
    title: &'static str,
    meaning: String,
    rules: String,
    example: String,
}

/// The explanation of a field.
fn explanation(field: Field) -> Explanation {
    let reserved = RESERVED_HEADERS.join(", ");

    let (title, meaning, rules, example): (&str, String, String, String) = match field {
        Field::Name => (
            "Name",
            "The name of the connection in IO Master.".to_owned(),
            "Up to 60 characters; unique among the connections.".to_owned(),
            "Ollama on this computer".to_owned(),
        ),
        Field::Url => (
            "Endpoint (full URL)",
            "The full address the request is sent to (always with the method POST).".to_owned(),
            format!(
                "https:// with TLS 1.3 or newer; plain http:// only to this computer, written as an IP address \
                 (127.0.0.1 or [::1]), never \"localhost\" or another host. No user name or password in the URL \
                 and no #fragment. The URL may hold {MODEL}, replaced by the name of the model. It may never hold \
                 the API key: a key goes only into a header."
            ),
            "http://127.0.0.1:11434/v1/chat/completions".to_owned(),
        ),
        Field::Model => (
            "Model",
            format!("The name of the model as the service knows it; put where a template says {MODEL}."),
            "Visible ASCII characters only, no spaces. Needed only when the URL, a header or the body uses \
             {{model}}."
                .to_owned(),
            "llama3.3:70b".to_owned(),
        ),
        Field::CaCertificate => (
            "CA certificate of the service",
            "The certificate of the authority that signed the certificate of the service, for a service with \
             an internal authority (for example of a laboratory). When given, only this authority is trusted \
             for the service; when empty, the trusted certificates of the system are used."
                .to_owned(),
            "One or more certificates in PEM (-----BEGIN CERTIFICATE-----). Not a private key.".to_owned(),
            "(empty for a public service)".to_owned(),
        ),
        Field::Headers => (
            "Headers",
            "The HTTP headers of the request, one per line, written as \"name: value\".".to_owned(),
            format!(
                "Names are HTTP tokens; values visible ASCII and spaces. The API key is written only as \
                 {API_KEY} inside a header value (for example \"authorization: Bearer {API_KEY}\"); {MODEL} may \
                 be used too; no other placeholder. These headers are set by IO Master itself and cannot be \
                 given: {reserved}. When content-type is not given, application/json is sent."
            ),
            "content-type: application/json\nauthorization: Bearer {{api_key}}".to_owned(),
        ),
        Field::Body => (
            "Body (JSON template)",
            "The JSON body of the request, with placeholders that IO Master replaces with values.".to_owned(),
            format!(
                "Valid JSON. A placeholder is always a whole JSON text and is replaced by a value, never \
                 joined into a text: \"{MESSAGES}\" becomes the list of the messages of the conversation (each \
                 made from the message template), \"{SYSTEM}\" the instructions of IO Master for the model, \
                 \"{MODEL}\" the name of the model. \"{MESSAGES}\" is required. The API key may not be used in \
                 the body. Fields such as the largest number of tokens or \"stream\" are written as plain JSON."
            ),
            "{\"model\": \"{{model}}\", \"messages\": \"{{messages}}\", \"stream\": true}".to_owned(),
        ),
        Field::MessageTemplate => (
            "One message in {{messages}} (JSON template)",
            "How one message of the conversation looks in the list that replaces \"{{messages}}\".".to_owned(),
            format!(
                "Valid JSON. \"{ROLE}\" becomes the role of the speaker (the roles of the connection) and \
                 \"{TEXT}\" the text of the message, each a whole JSON text. \"{TEXT}\" is required."
            ),
            DEFAULT_MESSAGE_TEMPLATE.to_owned(),
        ),
        Field::UserRole => (
            "Role of the evaluator",
            "The role that marks the messages of the person in {{messages}}.".to_owned(),
            "A short text, as the service expects it.".to_owned(),
            "user".to_owned(),
        ),
        Field::ModelRole => (
            "Role of the model",
            "The role that marks the earlier replies of the model in {{messages}}.".to_owned(),
            "A short text, as the service expects it.".to_owned(),
            "assistant".to_owned(),
        ),
        Field::SystemRole => (
            "Role of the instructions",
            "The role under which the instructions of IO Master are put as the first message of {{messages}}."
                .to_owned(),
            format!(
                "A short text, or empty for a service that takes the instructions in a field of its own; the \
                 body then holds \"{SYSTEM}\" at that field."
            ),
            "system".to_owned(),
        ),
        Field::SystemPrompt => (
            "Instructions for the model",
            "What IO Master tells the model before the conversation (a system prompt).".to_owned(),
            "Plain text, up to 64 KiB.".to_owned(),
            "You are the assistant in IO Master, a tool used by Common Criteria evaluators...".to_owned(),
        ),
        Field::Answer => (
            "Answer",
            "How the service sends its answer.".to_owned(),
            "Either \"stream\": server-sent events while the reply is written, every event's data being JSON \
             (or the end marker); or \"whole\": one JSON when the reply is complete. The paths of the \
             connection are read in every event, or in the whole answer."
                .to_owned(),
            "stream".to_owned(),
        ),
        Field::TextPath => (
            "Path to the text",
            "Where the text of the reply is: in every event of a stream (the next piece), or in the whole \
             answer."
                .to_owned(),
            format!("Required. {PATH_SYNTAX}"),
            "choices[0].delta.content".to_owned(),
        ),
        Field::StopPath => (
            "Path to the stop reason",
            "Where the service says why the reply ended.".to_owned(),
            format!("Optional. {PATH_SYNTAX}"),
            "choices[0].finish_reason".to_owned(),
        ),
        Field::CutReasons => (
            "Reasons for \"cut\"",
            "The stop reasons that mean the reply reached its largest length and was cut.".to_owned(),
            "A comma-separated list of the exact values the service uses.".to_owned(),
            "length".to_owned(),
        ),
        Field::RefusedReasons => (
            "Reasons for \"declined\"",
            "The stop reasons that mean the model or the safety checks of the service declined to answer."
                .to_owned(),
            "A comma-separated list of the exact values the service uses. Any other reason counts as a \
             complete reply."
                .to_owned(),
            "content_filter".to_owned(),
        ),
        Field::ErrorPath => (
            "Path to an error",
            "Where the explanation of an error is, in the body of a failed answer or in an event of a stream."
                .to_owned(),
            format!("Optional. A text found there in an event ends the reply as failed. {PATH_SYNTAX}"),
            "error.message".to_owned(),
        ),
        Field::ModelPath => (
            "Path to the model",
            "Where the service names the model that wrote the reply.".to_owned(),
            format!("Optional. {PATH_SYNTAX}"),
            "model".to_owned(),
        ),
        Field::InputTokensPath => (
            "Path to the tokens of the request",
            "Where the service counts the tokens of the request.".to_owned(),
            format!("Optional; the value is a whole number; the largest one seen is kept. {PATH_SYNTAX}"),
            "usage.prompt_tokens".to_owned(),
        ),
        Field::OutputTokensPath => (
            "Path to the tokens of the reply",
            "Where the service counts the tokens of the reply.".to_owned(),
            format!("Optional; the value is a whole number; the largest one seen is kept. {PATH_SYNTAX}"),
            "usage.completion_tokens".to_owned(),
        ),
        Field::DoneMarker => (
            "Data of the last event",
            "The data of the event that ends a stream, when the service sends one.".to_owned(),
            "Optional, only for a stream: the exact text of the data line, compared as a whole. Without it, \
             the stream is complete when a stop reason came."
                .to_owned(),
            "[DONE]".to_owned(),
        ),
    };

    Explanation {
        title,
        meaning,
        rules,
        example,
    }
}

/// The headers as they may be shown: the names, and in the values
/// only the placeholders (see the top of this file).
fn headers_shown(headers: &str) -> String {
    let lines: Vec<String> = headers
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (name, value) = line.split_once(':').unwrap_or((line, ""));

            if PLAIN_HEADERS.contains(&name.trim().to_ascii_lowercase().as_str()) {
                return format!("{}: {}", name.trim(), value.trim());
            }

            let placeholders: Vec<&str> = [API_KEY, MODEL].into_iter().filter(|placeholder| value.contains(placeholder)).collect();

            if placeholders.is_empty() {
                format!("{}: (value not shown)", name.trim())
            } else {
                format!("{}: (value not shown; uses {})", name.trim(), placeholders.join(" and "))
            }
        })
        .collect();

    if lines.is_empty() { "(none)".to_owned() } else { lines.join("\n") }
}

/// A value for the instruction: as it is, or "(empty)".
fn shown(value: &str) -> &str {
    if value.trim().is_empty() { "(empty)" } else { value }
}

/// The fields that give the chatbot the context of `field`, as they
/// are now. Never a value of a header, the certificate or a key.
fn context(field: Field, connection: &Connection) -> Vec<(&'static str, String)> {
    let lines = vec![
        ("Endpoint", shown(&connection.url).to_owned()),
        ("Model", shown(&connection.model).to_owned()),
        (
            "Answer",
            match connection.answer {
                AnswerMode::Stream => "a stream of server-sent events".to_owned(),
                AnswerMode::Whole => "one JSON when the reply is complete".to_owned(),
            },
        ),
    ];

    // The field itself is shown as "this field now", not here.
    let mut lines: Vec<(&'static str, String)> = lines
        .into_iter()
        .filter(|(name, _)| !matches!((field, *name), (Field::Url, "Endpoint") | (Field::Model, "Model") | (Field::Answer, "Answer")))
        .collect();

    let request = matches!(
        field,
        Field::Url | Field::Model | Field::Headers | Field::Body | Field::MessageTemplate
            | Field::UserRole | Field::ModelRole | Field::SystemRole
    );

    if request {
        if field != Field::Headers {
            lines.push(("Headers", headers_shown(&connection.headers)));
        }

        if field != Field::Body {
            lines.push(("Body", shown(&connection.body).to_owned()));
        }

        if field != Field::MessageTemplate {
            lines.push(("Message template", shown(&connection.message_template).to_owned()));
        }

        lines.push((
            "Roles",
            format!(
                "evaluator \"{}\", model \"{}\", instructions \"{}\"",
                connection.user_role, connection.model_role, connection.system_role
            ),
        ));
    } else if field != Field::SystemPrompt && field != Field::CaCertificate {
        // The fields of the answer: the other paths help.
        let paths = [
            (Field::TextPath, "Path to the text", &connection.text_path),
            (Field::StopPath, "Path to the stop reason", &connection.stop_path),
            (Field::ErrorPath, "Path to an error", &connection.error_path),
            (Field::DoneMarker, "Data of the last event", &connection.done_marker),
        ];

        for (other, name, value) in paths {
            if other != field {
                lines.push((name, shown(value).to_owned()));
            }
        }
    }

    lines
}

/// The current value of the field, as it may be shown.
fn current(field: Field, connection: &Connection) -> String {
    let value = match field {
        Field::Name => &connection.name,
        Field::Url => &connection.url,
        Field::Model => &connection.model,
        Field::CaCertificate => return "(not shown)".to_owned(),
        Field::Headers => return headers_shown(&connection.headers),
        Field::Body => &connection.body,
        Field::MessageTemplate => &connection.message_template,
        Field::UserRole => &connection.user_role,
        Field::ModelRole => &connection.model_role,
        Field::SystemRole => &connection.system_role,
        Field::SystemPrompt => &connection.system_prompt,
        Field::Answer => return connection.answer.name().to_owned(),
        Field::TextPath => &connection.text_path,
        Field::StopPath => &connection.stop_path,
        Field::CutReasons => &connection.cut_reasons,
        Field::RefusedReasons => &connection.refused_reasons,
        Field::ErrorPath => &connection.error_path,
        Field::ModelPath => &connection.model_path,
        Field::InputTokensPath => &connection.input_tokens_path,
        Field::OutputTokensPath => &connection.output_tokens_path,
        Field::DoneMarker => &connection.done_marker,
    };

    shown(value).to_owned()
}

/// The instruction for a chatbot to help fill `field` of `connection`
/// (the form as it is now, saved or not).
pub fn instruction(field: Field, connection: &Connection) -> String {
    let explanation = explanation(field);

    let context: String = context(field, connection)
        .into_iter()
        .map(|(name, value)| {
            if value.contains('\n') {
                format!("- {name}:\n```\n{value}\n```\n")
            } else {
                format!("- {name}: {value}\n")
            }
        })
        .collect();

    format!(
        "{CONTEXT}\n\n\
         The field: {title}\n\
         What it is: {meaning}\n\
         Rules: {rules}\n\
         Example:\n```\n{example}\n```\n\n\
         The connection so far (for context):\n{context}\
         - This field now: {now}\n\n\
         The service I want to use: {SERVICE}\n\n\
         {ANSWER}\n",
        title = explanation.title,
        meaning = explanation.meaning,
        rules = explanation.rules,
        example = explanation.example,
        now = current(field, connection),
    )
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "field_help_tests.rs"]
mod tests;
