//! Presets: ready settings of well-known AI services, to fill the
//! form of a connection on the tab AI of Settings.
//!
//! **A preset is only data.** It holds the same fields the evaluator
//! would type (`ai/api/connection.rs`), and choosing one only fills the
//! form; nothing is stored until the evaluator checks the fields and
//! saves them. The program treats a connection made from a preset
//! exactly like one typed by hand: there is no code for any provider.
//!
//! The values follow the documentation of each service as of
//! 2026-10-16: the endpoint, the headers, the body, and where the
//! text, the stop reason, the error, the model and the token counts
//! are in its stream. Names of models change often; the evaluator
//! checks the model before saving.

use crate::model::ai::{AnswerMode, Connection, DEFAULT_MESSAGE_TEMPLATE, DEFAULT_SYSTEM_PROMPT};

/// A preset.
#[derive(Debug, Clone)]
pub struct Preset {
    /// A short name for the page: "anthropic".
    pub id: &'static str,
    pub title: &'static str,
    /// What it is for and what to check.
    pub description: &'static str,
    /// The fields it fills; the name is a suggestion.
    pub connection: Connection,
}

/// Every preset, in the order of the page.
pub fn all() -> Vec<Preset> {
    vec![local_gemma(), anthropic(), anthropic_haiku(), openai(), google()]
}

/// The fields of the chat API that most local servers and OpenAI
/// offer (`/v1/chat/completions`, streamed): the template of a new
/// connection with another name, URL and model.
fn completions(name: &str, url: &str, model: &str, headers: &str, body: &str, system_role: &str) -> Connection {
    Connection {
        name: name.to_owned(),
        url: url.to_owned(),
        model: model.to_owned(),
        headers: headers.to_owned(),
        body: body.to_owned(),
        system_role: system_role.to_owned(),
        ..Connection::template()
    }
}

/// Gemma 4 on this computer, served by Ollama on its usual port with
/// its chat API (`/v1/chat/completions`).
fn local_gemma() -> Preset {
    Preset {
        id: "local-gemma",
        title: "Local Gemma (Ollama)",
        description: "Gemma 4 on this computer, served by Ollama on 127.0.0.1:11434 (plain HTTP, which does not \
                      leave the computer; no API key). The model is the tag of Ollama: gemma4 is the 4B model; \
                      gemma4:12b, gemma4:26b and gemma4:31b are larger (see \"ollama list\").",
        connection: completions(
            "Local Gemma",
            "http://127.0.0.1:11434/v1/chat/completions",
            "gemma4",
            "content-type: application/json",
            "{\n  \"model\": \"{{model}}\",\n  \"messages\": \"{{messages}}\",\n  \"stream\": true\n}",
            "system",
        ),
    }
}

/// Anthropic, Messages API, streamed, with Claude Opus 5.5. The
/// server-side fallback asks Anthropic to repeat a request its
/// safety checks declined on another Claude model; it is available
/// for Opus.
fn anthropic() -> Preset {
    Preset {
        id: "anthropic",
        title: "Anthropic (Claude)",
        description: "The Messages API of Anthropic with Claude Opus 5.5. Other models: claude-sonnet-5-5, \
                      claude-haiku-5-5 (a preset of its own); for them remove the header anthropic-beta and the \
                      field \"fallbacks\", which ask for the server-side fallback that only Opus has. Needs an \
                      API key.",
        connection: messages_api("Anthropic", "claude-opus-5-5", true),
    }
}

/// Anthropic, Messages API, streamed, with Claude Haiku 5.5: the
/// small and cheap model, handy for trying the program out. Haiku
/// has no server-side fallback, so the preset does not ask for it
/// (the API would refuse the request).
fn anthropic_haiku() -> Preset {
    Preset {
        id: "anthropic-haiku",
        title: "Anthropic (Claude Haiku)",
        description: "The Messages API of Anthropic with Claude Haiku 5.5, the small and cheap model, good for \
                      tests. Without the server-side fallback, which Haiku does not have. Needs an API key.",
        connection: messages_api("Anthropic Haiku", "claude-haiku-5-5", false),
    }
}

/// The fields of the Messages API of Anthropic, streamed: the
/// connection `name` with `model`. With `fallback`, the request
/// asks for the server-side fallback (the header anthropic-beta and
/// the field "fallbacks"), which only some models accept.
fn messages_api(name: &str, model: &str, fallback: bool) -> Connection {
    let (beta_header, fallbacks_field) = if fallback {
        ("\nanthropic-beta: server-side-fallback-2026-07-01", ",\n  \"fallbacks\": \"default\"")
    } else {
        ("", "")
    };

    Connection {
        name: name.to_owned(),
        url: "https://api.anthropic.com/v1/messages".to_owned(),
        model: model.to_owned(),
        ca_certificate: String::new(),
        headers: format!(
            "content-type: application/json\nx-api-key: {{{{api_key}}}}\nanthropic-version: 2023-06-01{beta_header}"
        ),
        body: format!(
            "{{\n  \"model\": \"{{{{model}}}}\",\n  \"max_tokens\": 64000,\n  \"system\": \"{{{{system}}}}\",\n  \
             \"messages\": \"{{{{messages}}}}\",\n  \"stream\": true{fallbacks_field}\n}}"
        ),
        message_template: DEFAULT_MESSAGE_TEMPLATE.to_owned(),
        user_role: "user".to_owned(),
        model_role: "assistant".to_owned(),
        system_role: String::new(),
        system_prompt: DEFAULT_SYSTEM_PROMPT.to_owned(),
        answer: AnswerMode::Stream,
        text_path: "delta.text".to_owned(),
        stop_path: "delta.stop_reason".to_owned(),
        cut_reasons: "max_tokens, model_context_window_exceeded".to_owned(),
        refused_reasons: "refusal".to_owned(),
        error_path: "error.message".to_owned(),
        model_path: "message.model".to_owned(),
        input_tokens_path: "message.usage.input_tokens".to_owned(),
        output_tokens_path: "usage.output_tokens".to_owned(),
        done_marker: String::new(),
    }
}

/// OpenAI, Chat Completions, streamed, with the token counts in the
/// last event.
fn openai() -> Preset {
    Preset {
        id: "openai",
        title: "OpenAI (GPT)",
        description: "The Chat Completions API of OpenAI with GPT-6 Astra, its flagship model. Other models: \
                      gpt-6.1-sol, gpt-6-luna. Needs an API key.",
        connection: completions(
            "OpenAI",
            "https://api.openai.com/v1/chat/completions",
            "gpt-6-astra",
            "content-type: application/json\nauthorization: Bearer {{api_key}}",
            "{\n  \"model\": \"{{model}}\",\n  \"messages\": \"{{messages}}\",\n  \"stream\": true,\n  \
             \"stream_options\": {\"include_usage\": true}\n}",
            "system",
        ),
    }
}

/// Google, the Gemini API (generateContent, streamed as server-sent
/// events): its own form of messages ("parts") and of the
/// instructions ("systemInstruction"), the model in the URL.
fn google() -> Preset {
    Preset {
        id: "google",
        title: "Google (Gemini)",
        description: "The Gemini API of Google with Gemini 4 Argon. Other models: gemini-3.8-flash, \
                      gemini-3.5-flash-lite. The model is part of the URL. Needs an API key.",
        connection: Connection {
            name: "Google Gemini".to_owned(),
            url: "https://generativelanguage.googleapis.com/v1beta/models/{{model}}:streamGenerateContent?alt=sse"
                .to_owned(),
            model: "gemini-4-argon".to_owned(),
            ca_certificate: String::new(),
            headers: "content-type: application/json\nx-goog-api-key: {{api_key}}".to_owned(),
            body: "{\n  \"systemInstruction\": {\"parts\": [{\"text\": \"{{system}}\"}]},\n  \
                   \"contents\": \"{{messages}}\"\n}"
                .to_owned(),
            message_template: "{\"role\": \"{{role}}\", \"parts\": [{\"text\": \"{{text}}\"}]}".to_owned(),
            user_role: "user".to_owned(),
            model_role: "model".to_owned(),
            system_role: String::new(),
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_owned(),
            answer: AnswerMode::Stream,
            text_path: "candidates[0].content.parts[*].text".to_owned(),
            stop_path: "candidates[0].finishReason".to_owned(),
            cut_reasons: "MAX_TOKENS".to_owned(),
            refused_reasons: "SAFETY, RECITATION, BLOCKLIST, PROHIBITED_CONTENT, SPII".to_owned(),
            error_path: "error.message".to_owned(),
            model_path: "modelVersion".to_owned(),
            input_tokens_path: "usageMetadata.promptTokenCount".to_owned(),
            output_tokens_path: "usageMetadata.candidatesTokenCount".to_owned(),
            done_marker: String::new(),
        },
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
