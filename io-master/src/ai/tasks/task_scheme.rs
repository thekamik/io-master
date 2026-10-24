//! The task `task_scheme` of the AI (role *text analysis*, step 51.3 of
//! docs/todo.md): the question with a requirement word for word, the scope
//! of the evaluation and the blocks of the scheme of its task, and the
//! reading of the answer: the inputs the AI proposes and its corrections of
//! the blocks already there. The planner accepts or rejects every one
//! (`views/evaluation_plan/task_scheme/propose.rs` stores them as
//! proposals); the AI proposes no outputs, which the template of the
//! project gives.
//!
//! The blocks are named for the model by short ids ("requirement",
//! "input-1", ...), never by their UUIDs, which a model may copy wrong.

use serde_json::{Value, json};

use crate::model::oscal::catalog::Control;
use crate::model::task_scheme::{Block, BlockFields, BlockKind, Correction, MAX_BLOCKS, MAX_INPUTS};

use super::candidates::requirement_text;
use super::json::{json_object, text_of};

/// The instructions when the evaluator has not changed them.
pub const DEFAULT_INSTRUCTIONS: &str = r#"You help the planner of a security evaluation (Common Criteria, ETSI EN 303 645 or another standard) to design how one requirement is evaluated. The evaluation of a requirement is a scheme: inputs (the evidence the evaluator delivers: a part of the Security Target, of the guidance or of the design, the output of a test or a tool, the notes of an interview) are connected to the requirement, whose agent verifies them and proposes whether the requirement is met. The outputs (the reports) are given by the laboratory; do not propose them.

Below are the requirement word for word, the scope of the evaluation and the blocks of the scheme now. Propose:
1. The inputs that are missing. For each: its name (short, one line), the evidence expected (exactly what the evaluator has to deliver), the instructions for its agent (what to check before the evidence is ready to be assessed) and the reason (which part of the requirement needs it).
2. Corrections of the blocks already there, only where they are wrong, incomplete or unclear. For each: the id of the block, its whole new name, evidence and instructions, and the reason. For the block "requirement" only the instructions count: what to check in the inputs and how to decide whether the requirement is met.
Do not repeat an input that is already there. Propose nothing rather than something vague.

Answer with one JSON object and nothing else, in this form:
{"inputs": [{"name": "...", "evidence": "...", "instructions": "...", "reason": "..."}],
 "corrections": [{"id": "input-1", "name": "...", "evidence": "...", "instructions": "...", "reason": "..."}]}"#;

/// The answer, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemeAnswer {
    /// The inputs proposed: their fields (name, evidence, instructions) and
    /// the reason, in the order of the answer.
    pub inputs: Vec<(BlockFields, String)>,
    /// The corrections, each naming the UUID of its block.
    pub corrections: Vec<Correction>,
    /// What could not be read, and why.
    pub dropped: Vec<String>,
}

/// The blocks the model sees, each with its short id: the requirement, then
/// the inputs in their order; the outputs are left out.
///
/// Called by: [`prompt`], [`read`].
fn named(blocks: &[Block]) -> Vec<(String, &Block)> {
    let mut found = Vec::with_capacity(blocks.len());
    let mut inputs = 0;

    for block in blocks {
        match block.kind {
            BlockKind::Requirement => found.push(("requirement".to_owned(), block)),
            BlockKind::Input => {
                inputs += 1;
                found.push((format!("input-{inputs}"), block));
            }
            BlockKind::Output => {}
        }
    }

    found
}

/// The message to the model: the instructions, the requirement `control`
/// (labelled `label`) with all its parts word for word, the scope of the
/// evaluation and the blocks of the scheme. Fails when the parts of the
/// control nest deeper than a catalog may.
///
/// Called by: `views/evaluation_plan/task_scheme/propose.rs`.
/// Calls: `requirement_text`, [`named`].
pub fn prompt(
    instructions: &str,
    label: &str,
    control: &Control,
    scope: &str,
    blocks: &[Block],
) -> Result<String, String> {
    let mut text = format!("{}\n\nThe requirement {}", instructions.trim(), requirement_text(label, control)?);

    if !scope.trim().is_empty() {
        text.push_str(&format!("\nThe scope of the evaluation:\n{}\n", scope.trim()));
    }

    let mut shown = Vec::new();

    for (id, block) in named(blocks) {
        shown.push(json!({
            "id": id,
            "name": block.fields.name,
            "evidence": block.fields.evidence,
            "instructions": block.fields.instructions,
        }));
    }

    text.push_str(&format!("\nThe blocks of the scheme now:\n{}\n", Value::Array(shown)));

    Ok(text)
}

/// Reads the answer of the model for a scheme of `blocks`: at most
/// [`MAX_INPUTS`] inputs and one correction per block. An item that cannot
/// be read (no name, an id of no block) is dropped with the reason; the
/// rules of the fields are checked by the caller.
///
/// Called by: `views/evaluation_plan/task_scheme/propose.rs`.
/// Calls: [`named`], `json_object`, `text_of`.
pub fn read(answer: &str, blocks: &[Block]) -> Result<SchemeAnswer, String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;
    let mut read = SchemeAnswer { inputs: Vec::new(), corrections: Vec::new(), dropped: Vec::new() };

    for (index, item) in
        value.get("inputs").and_then(Value::as_array).into_iter().flatten().take(MAX_INPUTS).enumerate()
    {
        let mut fields = BlockFields::empty();
        fields.name = text_of(item, "name");
        fields.evidence = text_of(item, "evidence");
        fields.instructions = text_of(item, "instructions");

        if fields.name.is_empty() {
            read.dropped.push(format!("input {}: no name", index + 1));
            continue;
        }

        read.inputs.push((fields, text_of(item, "reason")));
    }

    let named = named(blocks);

    for item in value.get("corrections").and_then(Value::as_array).into_iter().flatten().take(MAX_BLOCKS) {
        let id = text_of(item, "id");

        let Some((_, block)) = named.iter().find(|(candidate, _)| *candidate == id) else {
            read.dropped.push(format!("a correction of \"{id}\": there is no such block"));
            continue;
        };

        if read.corrections.iter().any(|correction| correction.block == block.uuid) {
            read.dropped.push(format!("a second correction of \"{id}\""));
            continue;
        }

        read.corrections.push(Correction {
            block: block.uuid.clone(),
            name: text_of(item, "name"),
            evidence: text_of(item, "evidence"),
            instructions: text_of(item, "instructions"),
            reason: text_of(item, "reason"),
        });
    }

    Ok(read)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `task_scheme.rs`.

    use super::*;

    fn block(uuid: &str, kind: BlockKind, name: &str) -> Block {
        let mut fields = BlockFields::empty();
        fields.name = name.to_owned();

        Block { uuid: uuid.to_owned(), kind, x: 0, y: 0, fields, proposal: None }
    }

    fn scheme() -> Vec<Block> {
        vec![
            block("u-r", BlockKind::Requirement, ""),
            block("u-in", BlockKind::Input, "Security Target"),
            block("u-out", BlockKind::Output, "pETR"),
        ]
    }

    fn control() -> Control {
        let control = json!({
            "id": "adv_fsp.1",
            "title": "Basic functional specification",
            "parts": [{ "name": "statement", "prose": "The developer shall provide a functional specification." }]
        });

        serde_json::from_value(control).unwrap()
    }

    #[test]
    fn the_question_holds_the_requirement_the_scope_and_the_blocks_by_short_ids() {
        let text = prompt("Propose.", "ADV_FSP.1", &control(), "A smart card.", &scheme()).unwrap();

        assert!(text.contains("The requirement ADV_FSP.1: Basic functional specification"));
        assert!(text.contains("The developer shall provide a functional specification."));
        assert!(text.contains("A smart card."));
        assert!(text.contains("\"input-1\""));
        assert!(!text.contains("u-in"), "no UUID is shown to the model");
        assert!(!text.contains("pETR"), "the outputs are left out");
    }

    #[test]
    fn inputs_and_corrections_are_read_and_a_correction_of_no_block_is_dropped() {
        let answer = r#"Here: {"inputs": [{"name": "Guidance", "evidence": "AGD_OPE", "instructions": "Check it.", "reason": "Needed."},
                                  {"name": "", "evidence": "x"}],
                       "corrections": [{"id": "input-1", "name": "ST", "evidence": "Section 6", "instructions": "", "reason": "Narrower."},
                                       {"id": "input-1", "name": "Again"},
                                       {"id": "output-1", "name": "pETR"}]}"#;

        let read = read(answer, &scheme()).unwrap();

        assert_eq!(read.inputs.len(), 1);
        assert_eq!((read.inputs[0].0.name.as_str(), read.inputs[0].1.as_str()), ("Guidance", "Needed."));
        assert_eq!(read.corrections.len(), 1);
        assert_eq!((read.corrections[0].block.as_str(), read.corrections[0].name.as_str()), ("u-in", "ST"));
        assert_eq!(read.dropped.len(), 3);
    }

    #[test]
    fn an_answer_without_json_is_refused() {
        assert!(read("I cannot help.", &scheme()).is_err());
        assert_eq!(
            read("{}", &scheme()).unwrap(),
            SchemeAnswer { inputs: Vec::new(), corrections: Vec::new(), dropped: Vec::new() }
        );
    }
}
