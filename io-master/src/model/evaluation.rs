//! An evaluation of the Evaluation lab (step 52 of docs/todo.md): a work
//! package imported from the Evaluation plan and carried out for one
//! product. The tasks, their schemes and their related elements are a frozen
//! copy of the package; what the evaluator adds is the evidence of the
//! inputs (kept exactly as delivered), the chat of each input, the fragments
//! (exact quotes found in the evidence, accepted by the evaluator), the four
//! results and the verdict of each requirement, and the texts of the outputs.
//!
//! ```text
//! evidence (raw) ──► fragments (quotes found in it, accepted) ──► the agent of the requirement
//!   ──► four results + a verdict proposed ──► the evaluator decides ──► the outputs whose condition holds
//! ```
//!
//! Plain data and its rules; stored by `db/evaluation/`.

use super::evaluation_project::{MAX_LINKS, MAX_TASKS};
use super::task_scheme::{
    self, Block, BlockKind, Connection, MAX_BLOCKS, OutputCondition, check_position, may_connect,
};
use super::text::{Field, Invalid, check_text};

/// The longest name of a product and of its version.
pub const PRODUCT_MAX: usize = 200;

/// The most evaluations an evaluator keeps.
pub const MAX_EVALUATIONS: usize = 200;

/// The most pieces of evidence of one input.
pub const MAX_EVIDENCE: usize = 20;

/// The largest PDF taken as evidence, in bytes.
pub const PDF_MAX: u64 = 64 * 1024 * 1024;

/// The largest text file taken as evidence, in bytes.
pub const TEXT_FILE_MAX: u64 = 8 * 1024 * 1024;

/// The longest pasted text, in characters.
pub const NOTE_MAX: usize = 1_000_000;

/// The most pages of a PDF read.
pub const MAX_PAGES: u32 = 5000;

/// The longest name of a piece of evidence.
pub const EVIDENCE_NAME_MAX: usize = 300;

/// The most messages of the chat of one input.
pub const MAX_MESSAGES: usize = 200;

/// The longest message of a chat.
pub const MESSAGE_MAX: usize = 20_000;

/// The most fragments of one input.
pub const MAX_FRAGMENTS: usize = 200;

/// The longest quote of a fragment, and its note.
pub const QUOTE_MAX: usize = 4000;

/// The longest text of one of the four results.
pub const RESULT_MAX: usize = 50_000;

/// The longest text of an output.
pub const OUTPUT_MAX: usize = 200_000;

/// The verdict of a requirement: set by the evaluator, proposed by the AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Met,
    NotMet,
    Inconclusive,
}

impl Verdict {
    pub const ALL: [Verdict; 3] = [Verdict::Met, Verdict::NotMet, Verdict::Inconclusive];

    /// The name kept in the database and sent to the page.
    pub fn name(self) -> &'static str {
        match self {
            Verdict::Met => "met",
            Verdict::NotMet => "not_met",
            Verdict::Inconclusive => "inconclusive",
        }
    }

    pub fn from_name(name: &str) -> Option<Verdict> {
        Verdict::ALL.into_iter().find(|verdict| verdict.name() == name)
    }
}

/// How far a task is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Todo,
    Working,
    Done,
}

impl TaskState {
    pub const ALL: [TaskState; 3] = [TaskState::Todo, TaskState::Working, TaskState::Done];

    pub fn name(self) -> &'static str {
        match self {
            TaskState::Todo => "todo",
            TaskState::Working => "working",
            TaskState::Done => "done",
        }
    }

    pub fn from_name(name: &str) -> Option<TaskState> {
        TaskState::ALL.into_iter().find(|state| state.name() == name)
    }
}

/// The form a piece of evidence was delivered in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    /// A PDF file; its text is read page by page.
    Pdf,
    /// A text file.
    Text,
    /// A text pasted by the evaluator: the notes of an interview, the output
    /// of a tool.
    Note,
}

impl EvidenceKind {
    pub const ALL: [EvidenceKind; 3] = [EvidenceKind::Pdf, EvidenceKind::Text, EvidenceKind::Note];

    pub fn name(self) -> &'static str {
        match self {
            EvidenceKind::Pdf => "pdf",
            EvidenceKind::Text => "text",
            EvidenceKind::Note => "note",
        }
    }

    pub fn from_name(name: &str) -> Option<EvidenceKind> {
        EvidenceKind::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

/// What the evaluator writes at the import: the product and its version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationFields {
    pub product: String,
    pub product_version: String,
}

/// An evaluation: what the package says of itself and the product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub uuid: String,
    pub package_uuid: String,
    pub project_title: String,
    pub scope: String,
    pub package_name: String,
    pub evaluator: String,
    pub note: String,
    pub fields: EvaluationFields,
}

/// An evaluation as a row of the list, with its progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationSummary {
    pub uuid: String,
    pub project_title: String,
    pub package_name: String,
    pub product: String,
    pub tasks: usize,
    pub done: usize,
}

/// A task of an evaluation: a whole requirement, as the package named it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabTask {
    pub uuid: String,
    pub catalog_uuid: String,
    pub control_id: String,
    pub label: String,
    pub title: String,
    pub state: TaskState,
}

/// An element of another catalog related to a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Related {
    pub catalog_uuid: String,
    pub control_id: String,
    pub label: String,
    pub title: String,
}

/// A task as a package brings it: the task, its scheme, its related
/// elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageTask {
    pub task: LabTask,
    pub blocks: Vec<Block>,
    pub connections: Vec<Connection>,
    pub related: Vec<Related>,
}

/// A work package as its file brings it (without its attachments).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub package_uuid: String,
    pub project_title: String,
    pub scope: String,
    pub name: String,
    pub evaluator: String,
    pub note: String,
    pub tasks: Vec<PackageTask>,
}

/// A piece of evidence of an input: how it was delivered and its text, page
/// by page (one page for a text); the bytes themselves are kept apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub uuid: String,
    pub block: String,
    pub kind: EvidenceKind,
    pub name: String,
    pub pages: Vec<String>,
    pub added: i64,
}

/// A fragment of the evidence of an input: an exact quote, found by the
/// program in the text of its evidence (or not), and a separate note of the
/// AI, never shown as a quote. Only an accepted fragment is assessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    pub uuid: String,
    pub evidence: String,
    /// The page of the evidence, counted from 1; `None` when not known.
    pub page: Option<u32>,
    pub quote: String,
    pub note: String,
    pub found: bool,
    pub accepted: bool,
    pub by_ai: bool,
}

/// The four results of a requirement and its verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Results {
    pub findings: String,
    pub analysis: String,
    pub assessment: String,
    pub vulnerabilities: String,
    pub verdict: Option<Verdict>,
}

impl Results {
    /// No result written, no verdict.
    pub fn empty() -> Results {
        Results {
            findings: String::new(),
            analysis: String::new(),
            assessment: String::new(),
            vulnerabilities: String::new(),
            verdict: None,
        }
    }
}

/// The text of an output: as the evaluator accepted it, and what the AI
/// proposed and is not decided yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputText {
    pub block: String,
    pub text: String,
    pub proposal: Option<String>,
}

/// The fields in their normal form: no spaces around them.
///
/// Called by: `package_import_import` (`views/evaluation_lab/evaluation_lab.rs`).
pub fn normalized(fields: &EvaluationFields) -> EvaluationFields {
    EvaluationFields {
        product: fields.product.trim().to_owned(),
        product_version: fields.product_version.trim().to_owned(),
    }
}

/// Checks the product and its version: one line each, at most
/// [`PRODUCT_MAX`] characters; both may be empty.
///
/// Called by: `package_import_import`.
pub fn check_fields(fields: &EvaluationFields) -> Result<(), Invalid> {
    check_text(Field::Product, &fields.product, PRODUCT_MAX, false)?;
    check_text(Field::ProductVersion, &fields.product_version, PRODUCT_MAX, false)
}

/// Checks a package read from its file: at least one task and at most
/// `MAX_TASKS`, every scheme by the rules of the Evaluation plan.
///
/// Called by: `package_import/read.rs`.
/// Calls: [`check_scheme`].
pub fn check_package(package: &Package) -> Result<(), Invalid> {
    ensure!(!package.tasks.is_empty(), Invalid::new(Field::Project, "the package has no task"));
    ensure!(
        package.tasks.len() <= MAX_TASKS,
        Invalid::new(Field::Project, format!("a package holds at most {MAX_TASKS} tasks"))
    );

    for task in &package.tasks {
        check_scheme(task)?;
    }

    Ok(())
}

/// Checks the scheme of one task of a package: one requirement, at most
/// `MAX_BLOCKS` blocks, each by the rules of its kind and placed near the
/// origin, the connections between its blocks as the plan allows them, at
/// most `MAX_LINKS` related elements.
///
/// Called by: [`check_package`].
fn check_scheme(task: &PackageTask) -> Result<(), Invalid> {
    let requirements = task.blocks.iter().filter(|block| block.kind == BlockKind::Requirement).count();

    ensure!(
        requirements == 1,
        Invalid::new(Field::Kind, format!("the task {} has no single requirement", task.task.label))
    );
    ensure!(task.blocks.len() <= MAX_BLOCKS, Invalid::new(Field::Kind, "a task holds too many blocks"));
    ensure!(task.related.len() <= MAX_LINKS, Invalid::new(Field::Kind, "a task has too many related elements"));

    for block in &task.blocks {
        task_scheme::check_fields(block.kind, &block.fields)?;
        check_position(block.x, block.y)?;
    }

    for connection in &task.connections {
        let from = task.blocks.iter().find(|block| block.uuid == connection.from);
        let to = task.blocks.iter().find(|block| block.uuid == connection.to);

        let allowed = match (from, to) {
            (Some(from), Some(to)) => may_connect(from.kind, to.kind),
            _ => false,
        };

        ensure!(
            allowed,
            Invalid::new(Field::Kind, format!("the task {} has a connection the plan does not allow", task.task.label))
        );
    }

    Ok(())
}

/// Checks the four results: each of at most [`RESULT_MAX`] characters.
///
/// Called by: `assessment_save` (`views/evaluation_lab/evaluation_lab.rs`), `assessment/propose.rs`.
pub fn check_results(results: &Results) -> Result<(), Invalid> {
    check_text(Field::Text, &results.findings, RESULT_MAX, true)?;
    check_text(Field::Text, &results.analysis, RESULT_MAX, true)?;
    check_text(Field::Text, &results.assessment, RESULT_MAX, true)?;
    check_text(Field::Text, &results.vulnerabilities, RESULT_MAX, true)
}

/// Checks a fragment written by the evaluator or proposed by the AI: a
/// quote of 1 to [`QUOTE_MAX`] characters and a note of at most as many.
///
/// Called by: `fragments_add` (`evaluation_lab.rs`), `fragments/propose.rs`.
pub fn check_fragment(quote: &str, note: &str) -> Result<(), Invalid> {
    ensure!(!quote.trim().is_empty(), Invalid::new(Field::Quote, "the fragment needs a quote"));
    check_text(Field::Quote, quote, QUOTE_MAX, true)?;
    check_text(Field::Text, note, QUOTE_MAX, true)
}

/// Checks a message of the chat of an input: 1 to [`MESSAGE_MAX`]
/// characters.
///
/// Called by: `input_chat_send` (`evaluation_lab.rs`).
pub fn check_message(text: &str) -> Result<(), Invalid> {
    ensure!(!text.trim().is_empty(), Invalid::new(Field::Text, "the message is empty"));
    check_text(Field::Text, text, MESSAGE_MAX, true)
}

/// Checks the text of an output: at most [`OUTPUT_MAX`] characters.
///
/// Called by: `outputs_save` (`evaluation_lab.rs`), `outputs/draft.rs`.
pub fn check_output(text: &str) -> Result<(), Invalid> {
    check_text(Field::Text, text, OUTPUT_MAX, true)
}

/// Whether an output with `condition` is filled for `verdict`: "always"
/// always; "met" and "not met" only for that verdict, never while the
/// verdict is inconclusive or not set.
///
/// Called by: `lab_task/read.rs`, `outputs/draft.rs`, `report/export.rs`.
pub fn output_applies(condition: OutputCondition, verdict: Option<Verdict>) -> bool {
    match condition {
        OutputCondition::Always => true,
        OutputCondition::Met => verdict == Some(Verdict::Met),
        OutputCondition::NotMet => verdict == Some(Verdict::NotMet),
    }
}

/// Whether `quote` is in the text of evidence `pages`: on `page` when it is
/// given (counted from 1), else on any page. Spaces and line breaks count as
/// one space, so a quote copied over the end of a line is found; the letters
/// must match exactly.
///
/// Called by: `fragments/propose.rs`, `fragments_add` (`evaluation_lab.rs`).
/// Calls: [`squeezed`].
pub fn quote_found(pages: &[String], page: Option<u32>, quote: &str) -> bool {
    let wanted = squeezed(quote);

    if wanted.is_empty() {
        return false;
    }

    match page {
        Some(number) => match usize::try_from(number).ok().and_then(|number| number.checked_sub(1)) {
            Some(index) => pages.get(index).is_some_and(|text| squeezed(text).contains(&wanted)),
            None => false,
        },
        None => pages.iter().any(|text| squeezed(text).contains(&wanted)),
    }
}

/// `text` with every run of white space as one space, none around.
///
/// Called by: [`quote_found`].
fn squeezed(text: &str) -> String {
    let mut squeezed = String::with_capacity(text.len());

    for word in text.split_whitespace() {
        if !squeezed.is_empty() {
            squeezed.push(' ');
        }

        squeezed.push_str(word);
    }

    squeezed
}

/// The inputs of a scheme, in their order.
///
/// Called by: the elements of the Evaluation lab that go through the inputs of a task.
pub fn inputs(blocks: &[Block]) -> Vec<&Block> {
    let mut found = Vec::new();

    for block in blocks {
        if block.kind == BlockKind::Input {
            found.push(block);
        }
    }

    found
}

#[cfg(test)]
mod tests {
    //! Unit tests of `evaluation.rs`.

    use super::*;

    use crate::model::task_scheme::{BlockFields, new_scheme};

    fn package_task() -> PackageTask {
        let mut output = BlockFields::empty();
        output.name = "pETR".to_owned();
        let scheme = new_scheme("r", &[("o".to_owned(), output)]).unwrap();

        PackageTask {
            task: LabTask {
                uuid: "t".to_owned(),
                catalog_uuid: "c".to_owned(),
                control_id: "s1".to_owned(),
                label: "1".to_owned(),
                title: "Roles".to_owned(),
                state: TaskState::Todo,
            },
            blocks: scheme.blocks,
            connections: scheme.connections,
            related: Vec::new(),
        }
    }

    fn package(tasks: Vec<PackageTask>) -> Package {
        Package {
            package_uuid: "w".to_owned(),
            project_title: "CC".to_owned(),
            scope: String::new(),
            name: "Roles".to_owned(),
            evaluator: "Anna".to_owned(),
            note: String::new(),
            tasks,
        }
    }

    #[test]
    fn the_names_read_back() {
        for verdict in Verdict::ALL {
            assert_eq!(Verdict::from_name(verdict.name()), Some(verdict));
        }
        for state in TaskState::ALL {
            assert_eq!(TaskState::from_name(state.name()), Some(state));
        }
        for kind in EvidenceKind::ALL {
            assert_eq!(EvidenceKind::from_name(kind.name()), Some(kind));
        }
        assert_eq!(Verdict::from_name("pass"), None);
    }

    #[test]
    fn a_package_needs_tasks_with_a_scheme_the_plan_allows() {
        assert!(check_package(&package(vec![package_task()])).is_ok());
        assert!(check_package(&package(Vec::new())).is_err());

        let mut two = package_task();
        two.blocks.push(Block { uuid: "r2".to_owned(), ..two.blocks[0].clone() });
        assert!(check_package(&package(vec![two])).is_err(), "two requirements");

        let mut wrong = package_task();
        wrong.connections.push(Connection { from: "o".to_owned(), to: "r".to_owned() });
        assert!(check_package(&package(vec![wrong])).is_err(), "an output to the requirement");

        let mut lost = package_task();
        lost.connections.push(Connection { from: "nowhere".to_owned(), to: "r".to_owned() });
        assert!(check_package(&package(vec![lost])).is_err());
    }

    #[test]
    fn an_output_waits_for_the_verdict_it_needs() {
        assert!(output_applies(OutputCondition::Always, None));
        assert!(output_applies(OutputCondition::NotMet, Some(Verdict::NotMet)));
        assert!(!output_applies(OutputCondition::NotMet, Some(Verdict::Inconclusive)));
        assert!(!output_applies(OutputCondition::Met, None));
        assert!(output_applies(OutputCondition::Met, Some(Verdict::Met)));
    }

    #[test]
    fn a_quote_is_found_over_the_ends_of_lines_and_on_its_page() {
        let pages = vec!["The TOE shall\nauthenticate   every user.".to_owned(), "Second page.".to_owned()];

        assert!(quote_found(&pages, None, "shall authenticate every user"));
        assert!(quote_found(&pages, Some(1), "The TOE shall authenticate"));
        assert!(!quote_found(&pages, Some(2), "The TOE"));
        assert!(!quote_found(&pages, Some(0), "The TOE"));
        assert!(!quote_found(&pages, None, "the toe shall"), "the letters must match");
        assert!(!quote_found(&pages, None, "  "));
    }

    #[test]
    fn the_texts_have_their_limits() {
        assert!(check_fragment("A quote.", "").is_ok());
        assert!(check_fragment(" ", "note").is_err());
        assert!(check_message("Is it enough?").is_ok());
        assert!(check_message("").is_err());
        let mut results = Results::empty();
        assert!(check_results(&results).is_ok());
        results.findings = "x".repeat(RESULT_MAX + 1);
        assert!(check_results(&results).is_err());
        assert!(
            check_fields(&EvaluationFields { product: "Reader".to_owned(), product_version: "2\n0".to_owned() })
                .is_err()
        );
    }
}
