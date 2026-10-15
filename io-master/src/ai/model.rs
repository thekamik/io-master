//! The Claude models an evaluator can choose for the chat.
//!
//! Only the current model of each family is offered: Opus (the most
//! capable, the default), Sonnet (fast and capable) and Haiku (the
//! fastest and cheapest). Their names are the plain names of the
//! API, without a date, so they always mean the current version.

/// A Claude model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Opus,
    Sonnet,
    Haiku,
}

impl Model {
    /// Every model, in the order of the list on the page.
    pub const ALL: [Model; 3] = [Model::Opus, Model::Sonnet, Model::Haiku];

    /// The model used until the evaluator chooses another.
    pub const DEFAULT: Model = Model::Opus;

    /// The name of the model in the API, also what is stored in the
    /// database of the evaluator.
    pub fn id(self) -> &'static str {
        match self {
            Model::Opus => "claude-opus-5-5",
            Model::Sonnet => "claude-sonnet-5-5",
            Model::Haiku => "claude-haiku-5-5",
        }
    }

    /// The model with the given API name; `None` for a name this
    /// version does not offer (for example one stored by an older
    /// version).
    pub fn from_id(id: &str) -> Option<Model> {
        Model::ALL.into_iter().find(|model| model.id() == id)
    }

    /// The name shown on the pages.
    pub fn title(self) -> &'static str {
        match self {
            Model::Opus => "Claude Opus 5.5",
            Model::Sonnet => "Claude Sonnet 5.5",
            Model::Haiku => "Claude Haiku 5.5",
        }
    }

    /// One line about the model, for the list on the page.
    pub fn description(self) -> &'static str {
        match self {
            Model::Opus => "the most capable; best for analysis of evaluation evidence",
            Model::Sonnet => "fast and capable",
            Model::Haiku => "the fastest and cheapest; for simple questions",
        }
    }

    /// Whether Anthropic can repeat a request on another model when
    /// its safety checks decline it, the "server-side fallback" of
    /// the API. Security questions, which an evaluator asks often,
    /// are sometimes declined by mistake; the fallback keeps such a
    /// mistake from ending the conversation. Haiku has no fallback;
    /// for Sonnet it is not documented, so it is not asked for.
    pub fn has_fallback(self) -> bool {
        matches!(self, Model::Opus)
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
