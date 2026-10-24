//! The data of the template of the outputs of a master project ("Standard
//! outputs", step 51.3 of docs/history.md), as the page gets it and sends it.

use serde::{Deserialize, Serialize};

use crate::app::error::AppError;
use crate::model::task_scheme::{BlockFields, OutputCondition, OutputFormat, StandardOutput};

/// An output of the template.
#[derive(Debug, Serialize)]
pub struct StandardOutputView {
    pub uuid: String,
    pub name: String,
    pub instructions: String,
    pub target: String,
    /// `"text"` or `"latex"`.
    pub format: &'static str,
    /// `"always"`, `"not_met"` or `"met"`.
    pub condition: &'static str,
}

/// The template as the page shows it, in its order.
///
/// Called by: `standard_outputs_get` and the commands that change the
/// template (`evaluation_plan.rs`).
pub fn all(outputs: Vec<StandardOutput>) -> Vec<StandardOutputView> {
    let mut views = Vec::with_capacity(outputs.len());

    for output in outputs {
        views.push(StandardOutputView {
            uuid: output.uuid,
            name: output.fields.name,
            instructions: output.fields.instructions,
            target: output.fields.target,
            format: output.fields.format.name(),
            condition: output.fields.condition.name(),
        });
    }

    views
}

/// The fields of an output of the template, as the page sends them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StandardOutputForm {
    pub name: String,
    pub instructions: String,
    pub target: String,
    pub format: String,
    pub condition: String,
}

impl StandardOutputForm {
    /// The fields of the model (an output has no evidence); refused when the
    /// format or the condition is one the program does not know.
    pub fn fields(&self) -> Result<BlockFields, AppError> {
        let Some(format) = OutputFormat::from_name(&self.format) else {
            return Err(AppError::Invalid("there is no such format of an output".to_owned()));
        };
        let Some(condition) = OutputCondition::from_name(&self.condition) else {
            return Err(AppError::Invalid("there is no such condition of an output".to_owned()));
        };

        Ok(BlockFields {
            name: self.name.clone(),
            evidence: String::new(),
            instructions: self.instructions.clone(),
            target: self.target.clone(),
            format,
            condition,
        })
    }
}
