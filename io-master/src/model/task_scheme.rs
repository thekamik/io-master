//! The scheme of a task of a master project (step 51.2 of docs/todo.md):
//! the blocks the evaluator meets in the Evaluation lab, as the planner
//! designs them. A task starts with only the block of its requirement
//! (inherited from OSCAL); the planner adds inputs (what the evaluator
//! delivers) and outputs (where the result goes, and when) and connects
//! them: an input to the requirement, the requirement to an output.
//!
//! ```text
//!  input ──┐
//!  input ──┼──► requirement ──┬──► output (always)
//!  input ──┘                  └──► output (only when the requirement is not met)
//! ```
//!
//! The outputs of a new task are copied from the template of its project
//! ("Standard outputs", step 51.3): the default shape of the outputs, which
//! the planner may change in each task.
//!
//! Plain data and its rules; stored by `db/evaluation_project/`.

use super::text::{Field, Invalid, check_text};

/// The most inputs of a task.
pub const MAX_INPUTS: usize = 20;

/// The most outputs of a task.
pub const MAX_OUTPUTS: usize = 10;

/// The most blocks of a task: its requirement, its inputs, its outputs.
pub const MAX_BLOCKS: usize = 1 + MAX_INPUTS + MAX_OUTPUTS;

/// The longest name of a block.
pub const NAME_MAX: usize = 200;

/// The longest text of a field of a block (an expected evidence, the
/// instructions for an agent, a target).
pub const TEXT_MAX: usize = 10_000;

/// The farthest a block may be placed from the origin of the drawing, in
/// either direction.
pub const POSITION_MAX: i64 = 100_000;

/// Where the outputs of a new task are placed: right of the requirement.
pub const OUTPUT_X: i64 = 320;

/// The distance of two blocks one below another in a new task.
pub const ROW: i64 = 130;

/// The kind of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// The requirement of the task: one, made with the task.
    Requirement,
    /// What the evaluator has to deliver.
    Input,
    /// Where the result goes.
    Output,
}

impl BlockKind {
    pub const ALL: [BlockKind; 3] = [BlockKind::Requirement, BlockKind::Input, BlockKind::Output];

    /// The name kept in the database and sent to the page.
    pub fn name(self) -> &'static str {
        match self {
            BlockKind::Requirement => "requirement",
            BlockKind::Input => "input",
            BlockKind::Output => "output",
        }
    }

    pub fn from_name(name: &str) -> Option<BlockKind> {
        BlockKind::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

/// What an output writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// A plain text file.
    Text,
    /// A template of the laboratory in LaTeX.
    Latex,
}

impl OutputFormat {
    pub const ALL: [OutputFormat; 2] = [OutputFormat::Text, OutputFormat::Latex];

    pub fn name(self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Latex => "latex",
        }
    }

    pub fn from_name(name: &str) -> Option<OutputFormat> {
        OutputFormat::ALL.into_iter().find(|format| format.name() == name)
    }
}

/// When an output is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCondition {
    /// Whatever the result.
    Always,
    /// Only when the requirement is not met (an Observation Report).
    NotMet,
    /// Only when the requirement is met.
    Met,
}

impl OutputCondition {
    pub const ALL: [OutputCondition; 3] = [OutputCondition::Always, OutputCondition::NotMet, OutputCondition::Met];

    pub fn name(self) -> &'static str {
        match self {
            OutputCondition::Always => "always",
            OutputCondition::NotMet => "not_met",
            OutputCondition::Met => "met",
        }
    }

    pub fn from_name(name: &str) -> Option<OutputCondition> {
        OutputCondition::ALL.into_iter().find(|condition| condition.name() == name)
    }
}

/// What the planner writes in a block. The fields a kind does not use are
/// empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockFields {
    /// The name of an input or an output; empty for the requirement (its
    /// title comes from its catalog).
    pub name: String,
    /// An input: the evidence the evaluator has to deliver.
    pub evidence: String,
    /// The instructions for the agent of the AI of the block: what to check
    /// in an input, what to assess in the requirement, how to fill an output.
    pub instructions: String,
    /// An output: where the result goes (a file, a template, its section).
    pub target: String,
    /// An output: what it writes.
    pub format: OutputFormat,
    /// An output: when it is filled.
    pub condition: OutputCondition,
}

impl BlockFields {
    /// The empty fields of a new block.
    pub fn empty() -> BlockFields {
        BlockFields {
            name: String::new(),
            evidence: String::new(),
            instructions: String::new(),
            target: String::new(),
            format: OutputFormat::Text,
            condition: OutputCondition::Always,
        }
    }
}

/// A block of a scheme, with its place in the drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub uuid: String,
    pub kind: BlockKind,
    pub x: i64,
    pub y: i64,
    pub fields: BlockFields,
}

/// A connection of two blocks: an input to the requirement, or the
/// requirement to an output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub from: String,
    pub to: String,
}

/// An output of the template of a project ("Standard outputs"): the default
/// shape of an output of every new task of the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardOutput {
    pub uuid: String,
    /// The fields of an output; `evidence` is empty.
    pub fields: BlockFields,
}

/// The scheme of a new task, before it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewScheme {
    pub blocks: Vec<Block>,
    pub connections: Vec<Connection>,
}

/// The fields in their normal form: no spaces around them.
///
/// Called by: `task_scheme_save_block`, `standard_outputs_add` and
/// `standard_outputs_save` (`views/evaluation_plan/evaluation_plan.rs`).
pub fn normalized(fields: &BlockFields) -> BlockFields {
    BlockFields {
        name: fields.name.trim().to_owned(),
        evidence: fields.evidence.trim().to_owned(),
        instructions: fields.instructions.trim().to_owned(),
        target: fields.target.trim().to_owned(),
        format: fields.format,
        condition: fields.condition,
    }
}

/// Checks the fields of a block of `kind`: an input and an output need a
/// name of one line; every text has its limit.
///
/// Called by: `task_scheme_save_block`.
pub fn check_fields(kind: BlockKind, fields: &BlockFields) -> Result<(), Invalid> {
    if kind != BlockKind::Requirement && fields.name.is_empty() {
        return Err(Invalid::new(Field::Name, "the block needs a name"));
    }

    check_text(Field::Name, &fields.name, NAME_MAX, false)?;
    check_text(Field::Text, &fields.evidence, TEXT_MAX, true)?;
    check_text(Field::Instructions, &fields.instructions, TEXT_MAX, true)?;
    check_text(Field::Place, &fields.target, TEXT_MAX, true)?;

    Ok(())
}

/// Checks a place in the drawing.
///
/// Called by: `task_scheme_add_block`, `task_scheme_move_block`.
pub fn check_position(x: i64, y: i64) -> Result<(), Invalid> {
    if x.abs() > POSITION_MAX || y.abs() > POSITION_MAX {
        return Err(Invalid::new(Field::Position, "the block is placed too far"));
    }

    Ok(())
}

/// Whether a block of kind `from` may be connected to one of kind `to`: an
/// input to the requirement, the requirement to an output.
///
/// Called by: `task_scheme_connect`.
pub fn may_connect(from: BlockKind, to: BlockKind) -> bool {
    matches!((from, to), (BlockKind::Input, BlockKind::Requirement) | (BlockKind::Requirement, BlockKind::Output))
}

/// The scheme of a new task: the block of its requirement `requirement` at
/// the origin and, in a column on its right, the outputs copied from the
/// template of the project, each connected from the requirement. `outputs`
/// are the UUIDs of the new blocks with the fields of the template; at most
/// [`MAX_OUTPUTS`].
///
/// Called by: `task_scheme/scheme.rs` (`first`), when a task is added.
pub fn new_scheme(requirement: &str, outputs: &[(String, BlockFields)]) -> Result<NewScheme, Invalid> {
    ensure!(
        outputs.len() <= MAX_OUTPUTS,
        Invalid::new(Field::Kind, format!("a task has at most {MAX_OUTPUTS} outputs"))
    );

    let mut blocks = Vec::with_capacity(1 + outputs.len());
    let mut connections = Vec::with_capacity(outputs.len());
    let first =
        Block { uuid: requirement.to_owned(), kind: BlockKind::Requirement, x: 0, y: 0, fields: BlockFields::empty() };

    blocks.push(first);

    // The column of the outputs is centred on the requirement.
    let mut y = -ROW * i64::try_from(outputs.len().saturating_sub(1)).unwrap_or(0) / 2;

    for (uuid, fields) in outputs {
        ensure!(uuid != requirement, Invalid::new(Field::Kind, "two blocks have one UUID"));

        blocks.push(Block { uuid: uuid.clone(), kind: BlockKind::Output, x: OUTPUT_X, y, fields: fields.clone() });
        connections.push(Connection { from: requirement.to_owned(), to: uuid.clone() });
        y += ROW;
    }

    Ok(NewScheme { blocks, connections })
}

/// Checks that a scheme with `blocks` may get one more block of `kind`:
/// never a second requirement; at most [`MAX_INPUTS`] inputs and
/// [`MAX_OUTPUTS`] outputs.
///
/// Called by: `task_scheme_add_block`.
pub fn check_new_block(blocks: &[Block], kind: BlockKind) -> Result<(), Invalid> {
    let mut count = 0;

    for block in blocks {
        if block.kind == kind {
            count += 1;
        }
    }

    let max = match kind {
        BlockKind::Requirement => 1,
        BlockKind::Input => MAX_INPUTS,
        BlockKind::Output => MAX_OUTPUTS,
    };

    if count >= max {
        return Err(Invalid::new(Field::Kind, format!("a task has at most {max} blocks of the kind {}", kind.name())));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `task_scheme.rs`.

    use super::*;

    fn block(kind: BlockKind) -> Block {
        Block { uuid: "b".to_owned(), kind, x: 0, y: 0, fields: BlockFields::empty() }
    }

    #[test]
    fn the_names_read_back() {
        for kind in BlockKind::ALL {
            assert_eq!(BlockKind::from_name(kind.name()), Some(kind));
        }
        for format in OutputFormat::ALL {
            assert_eq!(OutputFormat::from_name(format.name()), Some(format));
        }
        for condition in OutputCondition::ALL {
            assert_eq!(OutputCondition::from_name(condition.name()), Some(condition));
        }
        assert_eq!(BlockKind::from_name("tile"), None);
    }

    #[test]
    fn inputs_and_outputs_need_a_name_the_requirement_does_not() {
        let mut fields = BlockFields::empty();

        assert!(check_fields(BlockKind::Requirement, &fields).is_ok());
        assert!(check_fields(BlockKind::Input, &fields).is_err());

        fields.name = "Security Target".to_owned();
        assert!(check_fields(BlockKind::Input, &fields).is_ok());
        fields.name = "two\nlines".to_owned();
        assert!(check_fields(BlockKind::Output, &fields).is_err());
    }

    #[test]
    fn only_an_input_to_the_requirement_and_the_requirement_to_an_output_connect() {
        assert!(may_connect(BlockKind::Input, BlockKind::Requirement));
        assert!(may_connect(BlockKind::Requirement, BlockKind::Output));
        assert!(!may_connect(BlockKind::Input, BlockKind::Output));
        assert!(!may_connect(BlockKind::Output, BlockKind::Requirement));
        assert!(!may_connect(BlockKind::Input, BlockKind::Input));
    }

    #[test]
    fn one_requirement_and_the_limits_of_the_inputs() {
        let one = [block(BlockKind::Requirement)];

        assert!(check_new_block(&one, BlockKind::Requirement).is_err());
        assert!(check_new_block(&one, BlockKind::Input).is_ok());

        let full: Vec<Block> = (0..MAX_INPUTS).map(|_| block(BlockKind::Input)).collect();
        assert!(check_new_block(&full, BlockKind::Input).is_err());
        assert!(check_new_block(&full, BlockKind::Output).is_ok());
    }

    #[test]
    fn a_new_task_has_its_requirement_and_the_outputs_of_the_template_connected() {
        let mut report = BlockFields::empty();
        report.name = "Observation Report".to_owned();
        report.condition = OutputCondition::NotMet;
        let outputs = [("o1".to_owned(), BlockFields::empty()), ("o2".to_owned(), report.clone())];

        let scheme = new_scheme("r", &outputs).unwrap();

        assert_eq!(scheme.blocks.len(), 3);
        assert_eq!(scheme.blocks[0].kind, BlockKind::Requirement);
        assert_eq!((scheme.blocks[2].kind, &scheme.blocks[2].fields), (BlockKind::Output, &report));
        assert_eq!((scheme.blocks[1].y, scheme.blocks[2].y), (-ROW / 2, ROW / 2));
        assert_eq!(scheme.connections[1], Connection { from: "r".to_owned(), to: "o2".to_owned() });
        assert_eq!(new_scheme("r", &[]).unwrap().blocks.len(), 1);
    }

    #[test]
    fn a_new_task_takes_no_more_outputs_than_a_task_holds() {
        let outputs: Vec<(String, BlockFields)> =
            (0..=MAX_OUTPUTS).map(|index| (format!("o{index}"), BlockFields::empty())).collect();

        assert!(new_scheme("r", &outputs).is_err());
        assert!(new_scheme("r", &[("r".to_owned(), BlockFields::empty())]).is_err());
    }

    #[test]
    fn a_block_stays_near_the_origin() {
        assert!(check_position(10, -20).is_ok());
        assert!(check_position(POSITION_MAX + 1, 0).is_err());
    }
}
