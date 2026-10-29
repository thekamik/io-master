//! The instructions of the tasks of the AI, as the settings of the AI show
//! and change them. A task without instructions of the user has the
//! default ones of the program, written here; the user may change them,
//! and the changed text is stored.

use crate::db::ai::instructions::{MAX_INSTRUCTIONS, create_instructions, read_instructions, update_instructions};
use crate::db::database::Database;
use crate::model::ai::instructions::{AiTask, Instructions};
use crate::panels::logic::settings::error::SettingsError;

/// Every task, in the order the panel shows them.
pub const TASKS: [AiTask; 2] = [AiTask::Assessment, AiTask::Consultation];

/// The default instructions of the assessment of a work unit.
const ASSESSMENT: &str = "You assess one work unit of a security evaluation against its standard. \
Base every statement on the documents and the knowledge given with the question; quote the text you rely on \
word for word and name its document. Give your findings, your analysis and a verdict from the verdicts \
allowed. When the evidence is not enough, say what is missing and give no verdict.";

/// The default instructions of a consultation (a message of a chat).
const CONSULTATION: &str = "You answer the questions of a security evaluator about a standard and an \
evaluation. Base the answer on the knowledge given with the question, and say when it does not cover the \
question. Quote a standard word for word, never shortened.";

/// The default instructions of the task `task`.
///
/// Called by: [`task_instructions`].
fn default_text(task: AiTask) -> &'static str {
    match task {
        AiTask::Assessment => ASSESSMENT,
        AiTask::Consultation => CONSULTATION,
    }
}

/// The instructions of the task `task`: the stored ones, else the default
/// ones.
///
/// Called by: `panels/views/settings/instructions.rs`; the questions of the
/// assessment and of the chat.
/// Calls: `read_instructions`, [`default_text`].
pub fn task_instructions(db: &Database, task: AiTask) -> Result<Instructions, SettingsError> {
    match read_instructions(db, task)? {
        Some(stored) => Ok(stored),
        None => Ok(Instructions { task, text: default_text(task).to_owned() }),
    }
}

/// Stores the instructions of a task, trimmed: made the first time, written
/// anew after.
///
/// Called by: `panels/views/settings/instructions.rs`.
/// Calls: `read_instructions`, `create_instructions`, `update_instructions`.
pub fn save_instructions(db: &Database, instructions: &Instructions) -> Result<(), SettingsError> {
    let text = instructions.text.trim();

    ensure!(!text.is_empty(), SettingsError::Refused("empty instructions".to_owned()));
    ensure!(
        text.chars().count() <= MAX_INSTRUCTIONS,
        SettingsError::Refused(format!("instructions over {MAX_INSTRUCTIONS} characters"))
    );

    let trimmed = Instructions { task: instructions.task, text: text.to_owned() };

    match read_instructions(db, instructions.task)? {
        Some(_) => Ok(update_instructions(db, &trimmed)?),
        None => Ok(create_instructions(db, &trimmed)?),
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the instructions: the default ones, then the stored ones.

    use super::*;

    use crate::test_support::database::database;

    #[test]
    fn a_task_has_the_default_instructions_until_they_are_changed() {
        let (_directory, db) = database();

        assert_eq!(task_instructions(&db, AiTask::Consultation).unwrap().text, CONSULTATION);

        let changed = Instructions { task: AiTask::Consultation, text: "  Answer in Polish.  ".to_owned() };

        save_instructions(&db, &changed).unwrap();
        save_instructions(&db, &Instructions { text: "Answer in English.".to_owned(), ..changed.clone() }).unwrap();

        assert_eq!(task_instructions(&db, AiTask::Consultation).unwrap().text, "Answer in English.");
        assert_eq!(task_instructions(&db, AiTask::Assessment).unwrap().text, ASSESSMENT);
        assert!(matches!(
            save_instructions(&db, &Instructions { text: " ".to_owned(), ..changed }),
            Err(SettingsError::Refused(_))
        ));
    }
}
