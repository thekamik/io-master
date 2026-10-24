//! The data of the communication with AI services: a connection as
//! the evaluator sets it up, its fields and placeholders, the tasks and
//! the roles of the AI, and a conversation. The rules a connection
//! must keep, and everything that talks to a service, are in `ai/`
//! (`ai/api/connection.rs` checks a connection).

use std::fmt;

use reqwest::Url;

/// The placeholder of the conversation.
pub const MESSAGES: &str = "{{messages}}";
/// The placeholder of the instructions of the program.
pub const SYSTEM: &str = "{{system}}";
/// The placeholder of the name of the model.
pub const MODEL: &str = "{{model}}";
/// The placeholder of the API key.
pub const API_KEY: &str = "{{api_key}}";
/// The placeholder of the role of a speaker, in the message template.
pub const ROLE: &str = "{{role}}";
/// The placeholder of the text of a message, in the message template.
pub const TEXT: &str = "{{text}}";

/// The message template of a new connection.
pub const DEFAULT_MESSAGE_TEMPLATE: &str = "{\"role\": \"{{role}}\", \"content\": \"{{text}}\"}";

/// The longest name of a connection, in characters.
pub const MAX_NAME: usize = 60;

/// Headers the program sets itself, or that would change how the
/// request travels; a connection cannot set them.
pub const RESERVED_HEADERS: [&str; 6] = ["host", "content-length", "transfer-encoding", "connection", "upgrade", "te"];

/// The instructions of the program, the default of a new connection.
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are the assistant in IO Master, a tool used by Common \
Criteria evaluators in an evaluation laboratory. The person you talk to is an evaluator. Answer \
precisely and say plainly when you are not sure. When you refer to the Common Criteria (ISO/IEC \
15408) or the CEM (ISO/IEC 18045), name the part and the element, for example ADV_FSP.4.";

/// How the service sends its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerMode {
    /// A stream of server-sent events while the reply is written
    /// (`ai/api/provider/sse.rs`); every event is JSON, read with the paths.
    Stream,
    /// One JSON when the reply is complete, read with the paths.
    Whole,
}

impl AnswerMode {
    /// The name kept in the database and used by the page.
    pub fn name(self) -> &'static str {
        match self {
            AnswerMode::Stream => "stream",
            AnswerMode::Whole => "whole",
        }
    }

    /// The mode from its name.
    pub fn from_name(name: &str) -> Option<AnswerMode> {
        match name {
            "stream" => Some(AnswerMode::Stream),
            "whole" => Some(AnswerMode::Whole),
            _ => None,
        }
    }
}

/// A connection as it was entered. Every text is kept as written;
/// an empty text means "not used" where a field is optional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// The name the evaluator gave it: "Ollama on this computer".
    pub name: String,
    /// The full address of the endpoint; may hold `{{model}}`.
    pub url: String,
    /// The name of the model, for `{{model}}`; may be empty when no
    /// template uses it.
    pub model: String,
    /// The CA certificate (PEM) that alone is trusted for this
    /// service; empty: the trusted certificates of the system.
    pub ca_certificate: String,
    /// The headers, one per line: `name: value`.
    pub headers: String,
    /// The JSON template of the body.
    pub body: String,
    /// The JSON template of one message in `{{messages}}`.
    pub message_template: String,
    /// The role of the evaluator in `{{messages}}`: "user".
    pub user_role: String,
    /// The role of the model in `{{messages}}`: "assistant".
    pub model_role: String,
    /// The role under which the instructions are put first into
    /// `{{messages}}` ("system"); empty: they are not put there
    /// (a service that takes them in a field of its own uses
    /// `{{system}}`).
    pub system_role: String,
    /// The instructions of the program for the model.
    pub system_prompt: String,
    /// How the answer comes.
    pub answer: AnswerMode,
    /// Where the text is: in every event of a stream, or in the
    /// whole answer.
    pub text_path: String,
    /// Where the reason of the end of the reply is (optional).
    pub stop_path: String,
    /// The reasons that mean the reply was cut (comma-separated).
    pub cut_reasons: String,
    /// The reasons that mean the model declined (comma-separated).
    pub refused_reasons: String,
    /// Where the explanation of an error is (optional).
    pub error_path: String,
    /// Where the name of the model that answered is (optional).
    pub model_path: String,
    /// Where the count of tokens of the request is (optional).
    pub input_tokens_path: String,
    /// Where the count of tokens of the reply is (optional).
    pub output_tokens_path: String,
    /// The data of the event that ends a stream (optional): "[DONE]".
    pub done_marker: String,
}

/// A connection as it is stored, with its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredConnection {
    pub id: i64,
    pub connection: Connection,
    /// Whether an API key is stored for it (never the key itself).
    pub has_key: bool,
}

/// What saving a connection did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedConnection {
    pub id: i64,
    /// The API key was removed because the address changed.
    pub key_removed: bool,
}

impl Connection {
    /// The normal form of what is stored: the fields of one line
    /// without spaces around them; the headers, the body, the message
    /// template and the instructions as they were written. The only
    /// place where a connection is trimmed (step 40).
    pub fn normalized(&self) -> Connection {
        let line = |text: &String| text.trim().to_owned();

        Connection {
            name: line(&self.name),
            url: line(&self.url),
            model: line(&self.model),
            ca_certificate: line(&self.ca_certificate),
            user_role: line(&self.user_role),
            model_role: line(&self.model_role),
            system_role: line(&self.system_role),
            text_path: line(&self.text_path),
            stop_path: line(&self.stop_path),
            cut_reasons: line(&self.cut_reasons),
            refused_reasons: line(&self.refused_reasons),
            error_path: line(&self.error_path),
            model_path: line(&self.model_path),
            input_tokens_path: line(&self.input_tokens_path),
            output_tokens_path: line(&self.output_tokens_path),
            done_marker: line(&self.done_marker),
            ..self.clone()
        }
    }

    /// The starting point of a new connection: the request and the
    /// answer of the chat API that most local servers offer
    /// (`/v1/chat/completions`, streamed). Every field can be
    /// changed; nothing in the program depends on these values.
    pub fn template() -> Connection {
        Connection {
            name: String::new(),
            url: "http://127.0.0.1:11434/v1/chat/completions".to_owned(),
            model: String::new(),
            ca_certificate: String::new(),
            headers: "content-type: application/json".to_owned(),
            body: "{\n  \"model\": \"{{model}}\",\n  \"messages\": \"{{messages}}\",\n  \"stream\": true\n}".to_owned(),
            message_template: DEFAULT_MESSAGE_TEMPLATE.to_owned(),
            user_role: "user".to_owned(),
            model_role: "assistant".to_owned(),
            system_role: "system".to_owned(),
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_owned(),
            answer: AnswerMode::Stream,
            text_path: "choices[0].delta.content".to_owned(),
            stop_path: "choices[0].finish_reason".to_owned(),
            cut_reasons: "length".to_owned(),
            refused_reasons: "content_filter".to_owned(),
            error_path: "error.message".to_owned(),
            model_path: "model".to_owned(),
            input_tokens_path: "usage.prompt_tokens".to_owned(),
            output_tokens_path: "usage.completion_tokens".to_owned(),
            done_marker: "[DONE]".to_owned(),
        }
    }
}

/// A field of a connection: to say which one breaks a rule, and to
/// explain one (`ai/presets/field_help.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Url,
    Model,
    CaCertificate,
    Headers,
    Body,
    MessageTemplate,
    UserRole,
    ModelRole,
    SystemRole,
    SystemPrompt,
    Answer,
    TextPath,
    StopPath,
    CutReasons,
    RefusedReasons,
    ErrorPath,
    ModelPath,
    InputTokensPath,
    OutputTokensPath,
    DoneMarker,
}

impl Field {
    /// Every field, in the order of the form.
    pub const ALL: [Field; 21] = [
        Field::Name,
        Field::Url,
        Field::Model,
        Field::CaCertificate,
        Field::Headers,
        Field::Body,
        Field::MessageTemplate,
        Field::UserRole,
        Field::ModelRole,
        Field::SystemRole,
        Field::SystemPrompt,
        Field::Answer,
        Field::TextPath,
        Field::StopPath,
        Field::CutReasons,
        Field::RefusedReasons,
        Field::ErrorPath,
        Field::ModelPath,
        Field::InputTokensPath,
        Field::OutputTokensPath,
        Field::DoneMarker,
    ];

    /// The field from its name; `None` for a name no field has.
    pub fn from_name(name: &str) -> Option<Field> {
        Field::ALL.into_iter().find(|field| field.name() == name)
    }

    /// The name of the field, as the page knows it.
    pub fn name(self) -> &'static str {
        match self {
            Field::Name => "name",
            Field::Url => "url",
            Field::Model => "model",
            Field::CaCertificate => "ca_certificate",
            Field::Headers => "headers",
            Field::Body => "body",
            Field::MessageTemplate => "message_template",
            Field::UserRole => "user_role",
            Field::ModelRole => "model_role",
            Field::SystemRole => "system_role",
            Field::SystemPrompt => "system_prompt",
            Field::Answer => "answer",
            Field::TextPath => "text_path",
            Field::StopPath => "stop_path",
            Field::CutReasons => "cut_reasons",
            Field::RefusedReasons => "refused_reasons",
            Field::ErrorPath => "error_path",
            Field::ModelPath => "model_path",
            Field::InputTokensPath => "input_tokens_path",
            Field::OutputTokensPath => "output_tokens_path",
            Field::DoneMarker => "done_marker",
        }
    }
}

/// Why a connection cannot be used: the field, and what is wrong
/// with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionError {
    pub field: Field,
    pub reason: String,
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl std::error::Error for ConnectionError {}

impl Connection {
    /// The scheme, host and port of the URL: the address the API
    /// key is bound to (`db/ai/connection/`). `None` when the URL is
    /// not valid.
    pub fn origin(&self) -> Option<String> {
        let url = Url::parse(&self.url.trim().replace(MODEL, "model")).ok()?;

        Some(url.origin().ascii_serialization())
    }
}

/// A role of the AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Analysis of a text and pointing to its fragments.
    TextAnalysis,
    /// Summarising parts of a text.
    Summary,
    /// Consultation: the chat.
    Consultation,
    /// Formal analysis: the assessment of a requirement.
    FormalAnalysis,
}

/// The form of the answer a role expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerForm {
    /// One JSON object, read by the task (the tasks of the catalogs, step 41).
    Json,
    /// Free text.
    Text,
    /// Text for named fields (the four results of a requirement).
    Fields,
}

/// What the memory may do with the text a role works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRule {
    /// Word for word, never shortened: quotes of a standard,
    /// fragments of a document.
    Verbatim,
    /// May be shortened or summarised, only as its parameters say.
    ByParameters,
}

impl Role {
    /// Every role.
    pub const ALL: [Role; 4] = [Role::TextAnalysis, Role::Summary, Role::Consultation, Role::FormalAnalysis];

    /// The name kept in the database.
    pub fn name(self) -> &'static str {
        match self {
            Role::TextAnalysis => "text_analysis",
            Role::Summary => "summary",
            Role::Consultation => "consultation",
            Role::FormalAnalysis => "formal_analysis",
        }
    }

    pub fn from_name(name: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|role| role.name() == name)
    }

    /// The title shown to the evaluator.
    pub fn title(self) -> &'static str {
        match self {
            Role::TextAnalysis => "Analysis of a text and its fragments",
            Role::Summary => "Summary",
            Role::Consultation => "Consultation",
            Role::FormalAnalysis => "Formal analysis",
        }
    }

    pub fn answer(self) -> AnswerForm {
        match self {
            Role::TextAnalysis => AnswerForm::Json,
            Role::Summary | Role::Consultation => AnswerForm::Text,
            Role::FormalAnalysis => AnswerForm::Fields,
        }
    }

    /// Whether the quotes of its answers are checked against their
    /// pages.
    pub fn quotes_checked(self) -> bool {
        matches!(self, Role::TextAnalysis | Role::FormalAnalysis)
    }

    pub fn memory(self) -> MemoryRule {
        match self {
            Role::TextAnalysis | Role::FormalAnalysis => MemoryRule::Verbatim,
            Role::Summary | Role::Consultation => MemoryRule::ByParameters,
        }
    }
}

/// A task of the AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Task {
    /// The pages of the table of contents of a document of a standard
    /// (step 50.2).
    CatalogContents,
    /// The groups of an OSCAL catalog, from a table of contents.
    CatalogOutline,
    /// The controls of a group of an OSCAL catalog, from its pages.
    CatalogSection,
    /// A message of the chat.
    Chat,
    /// The check of a connection.
    Check,
}

impl Task {
    pub const ALL: [Task; 5] =
        [Task::CatalogContents, Task::CatalogOutline, Task::CatalogSection, Task::Chat, Task::Check];

    /// The name kept in the database and in the journal.
    pub fn name(self) -> &'static str {
        match self {
            Task::CatalogContents => "catalog_contents",
            Task::CatalogOutline => "catalog_outline",
            Task::CatalogSection => "catalog_section",
            Task::Chat => "chat",
            Task::Check => "check",
        }
    }

    pub fn from_name(name: &str) -> Option<Task> {
        Task::ALL.into_iter().find(|task| task.name() == name)
    }

    /// The role it is asked in; `None` for the check of a connection.
    pub fn role(self) -> Option<Role> {
        match self {
            Task::CatalogContents | Task::CatalogOutline | Task::CatalogSection => Some(Role::TextAnalysis),
            Task::Chat => Some(Role::Consultation),
            Task::Check => None,
        }
    }
}

/// Who said something in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    /// The evaluator (the "user role" of the connection).
    Evaluator,
    /// The model (the "model role" of the connection).
    Model,
}

/// One message of a conversation, as it is sent to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub speaker: Speaker,
    pub text: String,
}
