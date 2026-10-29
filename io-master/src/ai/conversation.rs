//! The conversation as both protocols take it: turns of the user and of
//! the model, in order, starting with the user.
//!
//! The services want the speakers to take turns and the user to start; a
//! chat may break this when an answer failed (two messages of the user in a
//! row). So empty messages are left out, messages of one speaker in a row
//! are joined into one, and an answer of the model at the start is dropped.
//! Nothing of the text is shortened.

use crate::model::ai::chat::{Message, Speaker};

/// One turn of the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub speaker: Speaker,
    pub text: String,
}

/// The turns of the messages `messages`, as described at the top.
///
/// Called by: `ask.rs` ([`crate::ai::ask::prepare`]).
pub fn turns(messages: &[Message]) -> Vec<Turn> {
    let mut turns: Vec<Turn> = Vec::with_capacity(messages.len());

    for message in messages {
        if message.text.trim().is_empty() {
            continue;
        }

        match turns.last_mut() {
            Some(last) if last.speaker == message.speaker => {
                last.text.push_str("\n\n");
                last.text.push_str(&message.text);
            }
            _ => turns.push(Turn { speaker: message.speaker, text: message.text.clone() }),
        }
    }

    if matches!(turns.first(), Some(first) if first.speaker == Speaker::Ai) {
        turns.remove(0);
    }

    turns
}

#[cfg(test)]
mod tests {
    //! Tests of the turns of a conversation.

    use super::*;

    fn message(speaker: Speaker, text: &str) -> Message {
        Message { speaker, text: text.to_owned(), time: 0 }
    }

    #[test]
    fn messages_become_alternating_turns_that_start_with_the_user() {
        let messages = [
            message(Speaker::Ai, "a greeting the model never got"),
            message(Speaker::User, "first"),
            message(Speaker::User, "second, after a failed answer"),
            message(Speaker::Ai, "   "),
            message(Speaker::Ai, "answer"),
            message(Speaker::User, "next"),
        ];

        assert_eq!(
            turns(&messages),
            [
                Turn { speaker: Speaker::User, text: "first\n\nsecond, after a failed answer".to_owned() },
                Turn { speaker: Speaker::Ai, text: "answer".to_owned() },
                Turn { speaker: Speaker::User, text: "next".to_owned() },
            ]
        );
    }
}
