//! The entry of the view Evaluation lab (step 52 of docs/history.md; page:
//! `ui/src/views/evaluation_lab/`, its commands in
//! `ui/src/lib/api/evaluation_lab.ts`): a work package of the Evaluation
//! plan imported and carried out for one product. The evaluator delivers
//! the evidence of the inputs, consults the chat of an input, accepts the
//! fragments of the evidence, decides the results and the verdict of every
//! requirement with the help of its agent, accepts the texts of the outputs,
//! and exports the report. The AI proposes, the evaluator decides (P1);
//! every question is in the journal of the AI (P3). The flow of data:
//! `docs/architecture.md`, section 12.

use tauri::ipc::Channel;
use tauri::{AppHandle, Runtime, State};

use crate::ai::api::tasks;
use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::components::ai_work::view::AiEvent;
use crate::db::evaluation as db_evaluation;
use crate::db::knowledge as db_knowledge;
use crate::db::oscal as db_oscal;
use crate::db::oscal::ids::new_uuid;
use crate::db::user::database::UserDb;
use crate::model::ai::{INSTRUCTIONS_MAX, Task};
use crate::model::evaluation::{
    self, Evaluation, Fragment, Package, TaskAgent, TaskState, check_fragment, check_output, quote_found,
};
use crate::model::oscal::OscalDocument;
use crate::model::task_scheme::BlockKind;

use super::assessment::propose as assessment_ask;
use super::evaluation_list::view::{self as list_view, EvaluationSummaryView};
use super::evidence::add as evidence_add;
use super::evidence::dialog::choose_evidence;
use super::fragments::propose as fragments_ask;
use super::fragments::view::FragmentForm;
use super::input_chat::chat;
use super::lab_task::read as lab_task;
use super::lab_task::view::{LabTaskScreenView, ResultsForm};
use super::lab_tasks::view::{self as tasks_view, EvaluationView};
use super::outputs::draft as outputs_ask;
use super::package_import::dialog::choose_package;
use super::package_import::read::{self as package_read, MAX_FILE};
use super::package_import::view::ImportForm;
use super::report::dialog::choose_folder;
use super::report::export as report_export_files;

// ── The evaluations ───────────────────────────────────────────────────

/// The evaluations, the newest import first, with their progress.
///
/// Called by the page: `EvaluationList.svelte`, when the view opens and after
/// a change (`evaluationLab.evaluationListGet()`).
/// Calls: `db_evaluation::read::evaluations`, `list_view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn evaluation_list_get(state: State<'_, AppState>) -> Result<Vec<EvaluationSummaryView>, AppError> {
    let session = state.session()?;

    Ok(list_view::all(db_evaluation::read::evaluations(&session.db())?))
}

/// Deletes evaluation `uuid` with everything the evaluator added to it.
///
/// Called by the page: `EvaluationList.svelte`, "Delete"
/// (`evaluationLab.evaluationListDelete(uuid)`).
/// Calls: `db_evaluation::delete::evaluation`, `list_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn evaluation_list_delete(
    state: State<'_, AppState>,
    uuid: String,
) -> Result<Vec<EvaluationSummaryView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    ensure!(
        db_evaluation::delete::evaluation(&db, &uuid)?,
        AppError::Invalid("the evaluation does not exist".to_owned())
    );

    Ok(list_view::all(db_evaluation::read::evaluations(&db)?))
}

/// Asks for the file of a work package in a dialog of the system and makes
/// an evaluation of it for the product of `form`; `None` when no file was
/// chosen. Refused when the file is no package, breaks its rules, or is
/// imported already.
///
/// Called by the page: `ImportPackage.svelte`, "Import…"
/// (`evaluationLab.packageImportImport(form)`).
/// Calls: `evaluation::normalized` and `check_fields`, `choose_package`,
/// `package_read::read`, [`store_package`], `list_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn package_import_import<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    form: ImportForm,
) -> Result<Option<Vec<EvaluationSummaryView>>, AppError> {
    let session = state.session()?;
    let fields = evaluation::normalized(&form.fields());

    evaluation::check_fields(&fields).map_err(|error| AppError::Invalid(error.to_string()))?;

    let Some(path) = choose_package(app).await? else {
        return Ok(None);
    };

    let size = tokio::fs::metadata(&path)
        .await
        .map_err(|error| AppError::Failed(format!("the file was not read: {error}")))?
        .len();
    ensure!(size <= MAX_FILE, AppError::Invalid("the file is too large for a work package".to_owned()));

    let text = tokio::fs::read_to_string(&path)
        .await
        .map_err(|error| AppError::Failed(format!("the file was not read: {error}")))?;
    let (package, catalogs) = package_read::read(&text)?;
    let db = session.db();
    let made = Evaluation {
        uuid: new_uuid()?,
        package_uuid: package.package_uuid.clone(),
        project_title: package.project_title.clone(),
        scope: package.scope.clone(),
        package_name: package.name.clone(),
        evaluator: package.evaluator.clone(),
        note: package.note.clone(),
        fields,
    };

    store_package(&db, &made, &package, &catalogs)?;

    Ok(Some(list_view::all(db_evaluation::read::evaluations(&db)?)))
}

/// The evaluation `evaluation` with its tasks and their state.
///
/// Called by the page: `EvaluationScreen.svelte`, when an evaluation opens and
/// after a change (`evaluationLab.labTasksGet(evaluation)`).
/// Calls: [`evaluation_of`], `tasks_view::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn lab_tasks_get(state: State<'_, AppState>, evaluation: String) -> Result<EvaluationView, AppError> {
    let session = state.session()?;
    let db = session.db();

    tasks_view::screen(&db, evaluation_of(&db, &evaluation)?)
}

/// Chooses the agent with knowledge of evaluation `evaluation` (none:
/// `None`), given to the agent of every requirement whose task has no
/// other; the unticked passages of its tasks are forgotten.
///
/// Called by the page: `EvaluationScreen.svelte`, the choice of the agent
/// (`evaluationLab.labTasksAgent(evaluation, agent)`).
/// Calls: [`evaluation_of`], [`existing_agent`], `db_evaluation::update::agent`, `tasks_view::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn lab_tasks_agent(
    state: State<'_, AppState>,
    evaluation: String,
    agent: Option<i64>,
) -> Result<EvaluationView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let found = evaluation_of(&db, &evaluation)?;

    let agent = match agent {
        Some(id) => Some(existing_agent(&db, id)?),
        None => None,
    };
    db_evaluation::update::agent(&db, &evaluation, agent)?;

    tasks_view::screen(&db, found)
}

/// Sets the state of task `task` (`"todo"`, `"working"`, `"done"`).
///
/// Called by the page: `EvaluationScreen.svelte`, the state of a task
/// (`evaluationLab.labTasksState(evaluation, task, progress)`).
/// Calls: `TaskState::from_name`, `db_evaluation::update::task_state`, [`evaluation_of`].
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn lab_tasks_state(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
    progress: String,
) -> Result<EvaluationView, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some(progress) = TaskState::from_name(&progress) else {
        return Err(AppError::Invalid("there is no such state of a task".to_owned()));
    };

    ensure!(
        db_evaluation::update::task_state(&db, &evaluation, &task, progress)?,
        AppError::Invalid("the task does not exist".to_owned())
    );

    tasks_view::screen(&db, evaluation_of(&db, &evaluation)?)
}

// ── A task: its scheme and its evidence ───────────────────────────────

/// The screen of task `task`: its scheme, its requirement, its evidence,
/// fragments, results and outputs.
///
/// Called by the page: `LabTask.svelte`, when a task opens
/// (`evaluationLab.labTaskGet(evaluation, task)`).
/// Calls: `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn lab_task_get(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;

    lab_task::screen(&session.db(), &evaluation, &task)
}

/// Asks for a file of evidence (a PDF or a text) in a dialog of the system
/// and keeps it for input `block`, as delivered; `None` when no file was
/// chosen.
///
/// Called by the page: `InputPanel.svelte`, "Add a file…"
/// (`evaluationLab.evidenceAddFile(evaluation, block)`).
/// Calls: `choose_evidence`, `evidence_add::read_file` (on a thread for
/// blocking work: Poppler reads a PDF), `evidence_add::add_file`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn evidence_add_file<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    evaluation: String,
    block: String,
) -> Result<Option<LabTaskScreenView>, AppError> {
    let session = state.session()?;

    let Some(path) = choose_evidence(app).await? else {
        return Ok(None);
    };

    // spawn_blocking takes a closure: a library place (rule 9 of CLAUDE.md).
    let read = tokio::task::spawn_blocking(move || evidence_add::read_file(&path))
        .await
        .map_err(|_| AppError::Failed("the file was not read".to_owned()))??;
    let db = session.db();
    let task = evidence_add::add_file(&db, &evaluation, &block, read)?;

    Ok(Some(lab_task::screen(&db, &evaluation, &task)?))
}

/// Keeps the text `text` pasted by the evaluator (the notes of an
/// interview, the output of a tool), named `name`, as evidence of input
/// `block`.
///
/// Called by the page: `InputPanel.svelte`, "Add the text"
/// (`evaluationLab.evidenceAddNote(evaluation, block, name, text)`).
/// Calls: `evidence_add::add_note`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn evidence_add_note(
    state: State<'_, AppState>,
    evaluation: String,
    block: String,
    name: String,
    text: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let task = evidence_add::add_note(&db, &evaluation, &block, &name, &text)?;

    lab_task::screen(&db, &evaluation, &task)
}

/// Deletes piece of evidence `evidence` of task `task` with its fragments.
///
/// Called by the page: `InputPanel.svelte`, "Delete" of a piece of evidence
/// (`evaluationLab.evidenceDelete(evaluation, task, evidence)`).
/// Calls: `db_evaluation::delete::evidence`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn evidence_delete(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
    evidence: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();

    ensure!(
        db_evaluation::delete::evidence(&db, &evaluation, &evidence)?,
        AppError::Invalid("the evidence does not exist".to_owned())
    );

    lab_task::screen(&db, &evaluation, &task)
}

// ── The chat and the fragments of an input ────────────────────────────

/// Sends `text` to the chat of input `block`; the events arrive through
/// `on_event`.
///
/// Called by the page: `InputPanel.svelte`, "Send"
/// (`evaluationLab.inputChatSend(evaluation, block, text, onEvent)`).
/// Calls: `AppState::start_work`, `chat::send` (one question through
/// `ai/api/`), `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn input_chat_send(
    state: State<'_, AppState>,
    evaluation: String,
    block: String,
    text: String,
    on_event: Channel<AiEvent>,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let stop = state.start_work();
    let task = chat::send(&session, &evaluation, &block, &text, &stop, &on_event).await?;

    lab_task::screen(&session.db(), &evaluation, &task)
}

/// Clears the chat of input `block`.
///
/// Called by the page: `InputPanel.svelte`, "Clear the chat"
/// (`evaluationLab.inputChatClear(evaluation, block)`).
/// Calls: [`task_of_block`], `db_evaluation::delete::messages`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn input_chat_clear(
    state: State<'_, AppState>,
    evaluation: String,
    block: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let task = task_of_block(&db, &evaluation, &block)?;

    db_evaluation::delete::messages(&db, &evaluation, &block)?;

    lab_task::screen(&db, &evaluation, &task)
}

/// The AI proposes the fragments of the evidence of input `block`; the
/// proposals left undecided are removed first. The events arrive through
/// `on_event`.
///
/// Called by the page: `InputPanel.svelte`, "Find fragments with AI"
/// (`evaluationLab.fragmentsPropose(evaluation, block, onEvent)`).
/// Calls: `AppState::start_work`, `fragments_ask::propose` (one question per
/// part through `ai/api/`), `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn fragments_propose(
    state: State<'_, AppState>,
    evaluation: String,
    block: String,
    on_event: Channel<AiEvent>,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let stop = state.start_work();
    let task = fragments_ask::propose(&session, &evaluation, &block, &stop, &on_event).await?;

    lab_task::screen(&session.db(), &evaluation, &task)
}

/// Accepts fragment `fragment` of task `task`: it takes part in the
/// assessment.
///
/// Called by the page: `InputPanel.svelte`, "Accept" of a fragment
/// (`evaluationLab.fragmentsAccept(evaluation, task, fragment)`).
/// Calls: `db_evaluation::update::fragment_accepted`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn fragments_accept(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
    fragment: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();

    ensure!(
        db_evaluation::update::fragment_accepted(&db, &evaluation, &fragment)?,
        AppError::Invalid("the fragment is not waiting for a decision".to_owned())
    );

    lab_task::screen(&db, &evaluation, &task)
}

/// Removes fragment `fragment` of task `task`: a proposal rejected, or a
/// fragment the evaluator no longer wants.
///
/// Called by the page: `InputPanel.svelte`, "Reject" and "Remove" of a
/// fragment (`evaluationLab.fragmentsRemove(evaluation, task, fragment)`).
/// Calls: `db_evaluation::delete::fragment`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn fragments_remove(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
    fragment: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();

    ensure!(
        db_evaluation::delete::fragment(&db, &evaluation, &fragment)?,
        AppError::Invalid("the fragment does not exist".to_owned())
    );

    lab_task::screen(&db, &evaluation, &task)
}

/// Adds the fragment the evaluator wrote to input `block`: accepted at
/// once, with whether its quote is found in its evidence.
///
/// Called by the page: `InputPanel.svelte`, "Add the fragment"
/// (`evaluationLab.fragmentsAdd(evaluation, block, fragment)`).
/// Calls: [`task_of_block`], `read::evidence_of`, `check_fragment`,
/// `quote_found`, `db_evaluation::create::fragment`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn fragments_add(
    state: State<'_, AppState>,
    evaluation: String,
    block: String,
    fragment: FragmentForm,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let task = task_of_block(&db, &evaluation, &block)?;
    let evidence = db_evaluation::read::evidence_of(&db, &evaluation, &block)?;

    let Some(piece) = evidence.iter().find(|piece| piece.uuid == fragment.evidence) else {
        return Err(AppError::Invalid("the evidence is not of this input".to_owned()));
    };

    let quote = fragment.quote.trim();
    check_fragment(quote, fragment.note.trim()).map_err(|error| AppError::Invalid(error.to_string()))?;

    let written = Fragment {
        uuid: new_uuid()?,
        evidence: piece.uuid.clone(),
        page: fragment.page,
        quote: quote.to_owned(),
        note: fragment.note.trim().to_owned(),
        found: quote_found(&piece.pages, fragment.page, quote),
        accepted: true,
        by_ai: false,
    };

    db_evaluation::create::fragment(&db, &evaluation, &task, &block, &written)?;
    db_evaluation::update::task_started(&db, &evaluation, &task)?;

    lab_task::screen(&db, &evaluation, &task)
}

// ── The requirement: its results and verdict ──────────────────────────

/// The agent of the requirement of task `task` proposes its four results
/// and a verdict, on the accepted fragments; the events arrive through
/// `on_event`.
///
/// Called by the page: `RequirementPanel.svelte`, "Assess with AI"
/// (`evaluationLab.assessmentPropose(evaluation, task, onEvent)`).
/// Calls: `AppState::start_work`, `assessment_ask::propose` (one question
/// through `ai/api/`), `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn assessment_propose(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
    on_event: Channel<AiEvent>,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let stop = state.start_work();

    assessment_ask::propose(&session, &evaluation, &task, &stop, &on_event).await?;

    lab_task::screen(&session.db(), &evaluation, &task)
}

/// Accepts what the agent proposed: its results and its verdict become
/// those of the task (the evaluator may change them afterwards).
///
/// Called by the page: `RequirementPanel.svelte`, "Accept the proposal"
/// (`evaluationLab.assessmentAccept(evaluation, task)`).
/// Calls: `read::results_proposal`, `update::results`, `delete::results_proposal`,
/// `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn assessment_accept(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some((proposed, _)) = db_evaluation::read::results_proposal(&db, &evaluation, &task)? else {
        return Err(AppError::Invalid("the AI proposed nothing for this task".to_owned()));
    };

    db_evaluation::update::results(&db, &evaluation, &task, &proposed)?;
    db_evaluation::delete::results_proposal(&db, &evaluation, &task)?;

    lab_task::screen(&db, &evaluation, &task)
}

/// Rejects what the agent proposed for task `task`.
///
/// Called by the page: `RequirementPanel.svelte`, "Reject the proposal"
/// (`evaluationLab.assessmentReject(evaluation, task)`).
/// Calls: `db_evaluation::delete::results_proposal`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn assessment_reject(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();

    ensure!(
        db_evaluation::delete::results_proposal(&db, &evaluation, &task)?,
        AppError::Invalid("the AI proposed nothing for this task".to_owned())
    );

    lab_task::screen(&db, &evaluation, &task)
}

/// Saves the four results and the verdict the evaluator wrote for task
/// `task`.
///
/// Called by the page: `RequirementPanel.svelte`, "Save"
/// (`evaluationLab.assessmentSave(evaluation, task, results)`).
/// Calls: `lab_task::task`, `evaluation::check_results`, `update::results`,
/// `update::task_started`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn assessment_save(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
    results: ResultsForm,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let written = results.results()?;

    lab_task::task(&db, &evaluation, &task)?;
    evaluation::check_results(&written).map_err(|error| AppError::Invalid(error.to_string()))?;
    db_evaluation::update::results(&db, &evaluation, &task, &written)?;
    db_evaluation::update::task_started(&db, &evaluation, &task)?;

    lab_task::screen(&db, &evaluation, &task)
}

/// Chooses the agent with knowledge of task `task`: `"evaluation"` (that of
/// its evaluation), `"none"`, or `"agent"` with `agent`. Its unticked
/// passages are forgotten.
///
/// Called by the page: `RequirementPanel.svelte`, the choice of the agent
/// (`evaluationLab.assessmentAgent(evaluation, task, choice, agent)`).
/// Calls: [`existing_agent`], `db_evaluation::update::task_agent`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn assessment_agent(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
    choice: String,
    agent: Option<i64>,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();
    lab_task::task(&db, &evaluation, &task)?;

    let chosen = match (choice.as_str(), agent) {
        ("evaluation", _) => TaskAgent::AsEvaluation,
        ("none", _) => TaskAgent::NoAgent,
        ("agent", Some(id)) => TaskAgent::Agent(existing_agent(&db, id)?),
        _ => return Err(AppError::Invalid("there is no such choice of an agent".to_owned())),
    };

    db_evaluation::update::task_agent(&db, &evaluation, &task, chosen)?;

    lab_task::screen(&db, &evaluation, &task)
}

/// Ticks (`chosen`) or unticks passage `passage` of the agent of task
/// `task`: an unticked passage is not given to the question of the assessment.
///
/// Called by the page: `RequirementPanel.svelte`, the box of a passage
/// (`evaluationLab.assessmentContext(evaluation, task, passage, chosen)`).
/// Calls: `db_evaluation::update::context_excluded`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn assessment_context(
    state: State<'_, AppState>,
    evaluation: String,
    task: String,
    passage: i64,
    chosen: bool,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();
    lab_task::task(&db, &evaluation, &task)?;

    db_evaluation::update::context_excluded(&db, &evaluation, &task, passage, !chosen)?;

    lab_task::screen(&db, &evaluation, &task)
}

/// Agent `id` when it exists; refused otherwise.
///
/// Called by: `assessment_agent`, `lab_tasks_agent`.
fn existing_agent(db: &UserDb, id: i64) -> Result<i64, AppError> {
    ensure!(db_knowledge::read::agent(db, id)?.is_some(), AppError::Invalid("the agent does not exist".to_owned()));

    Ok(id)
}

// ── The outputs and the report ────────────────────────────────────────

/// The AI drafts the text of output `block` from the results and the
/// verdict; the events arrive through `on_event`.
///
/// Called by the page: `OutputPanel.svelte`, "Draft with AI"
/// (`evaluationLab.outputsDraft(evaluation, block, onEvent)`).
/// Calls: `AppState::start_work`, `outputs_ask::draft` (one question through
/// `ai/api/`), `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn outputs_draft(
    state: State<'_, AppState>,
    evaluation: String,
    block: String,
    on_event: Channel<AiEvent>,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let stop = state.start_work();
    let task = outputs_ask::draft(&session, &evaluation, &block, &stop, &on_event).await?;

    lab_task::screen(&session.db(), &evaluation, &task)
}

/// Accepts the draft of output `block`: it becomes its text.
///
/// Called by the page: `OutputPanel.svelte`, "Accept the draft"
/// (`evaluationLab.outputsAccept(evaluation, block)`).
/// Calls: [`task_of_block`], `read::output`, `update::output_text`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn outputs_accept(
    state: State<'_, AppState>,
    evaluation: String,
    block: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let task = task_of_block(&db, &evaluation, &block)?;

    let proposal = db_evaluation::read::output(&db, &evaluation, &block)?.and_then(|output| output.proposal);
    let Some(proposal) = proposal else {
        return Err(AppError::Invalid("the AI drafted nothing for this output".to_owned()));
    };

    db_evaluation::update::output_text(&db, &evaluation, &block, &proposal)?;

    lab_task::screen(&db, &evaluation, &task)
}

/// Rejects the draft of output `block`; its text stays.
///
/// Called by the page: `OutputPanel.svelte`, "Reject the draft"
/// (`evaluationLab.outputsReject(evaluation, block)`).
/// Calls: [`task_of_block`], `update::output_rejected`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn outputs_reject(
    state: State<'_, AppState>,
    evaluation: String,
    block: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let task = task_of_block(&db, &evaluation, &block)?;

    ensure!(
        db_evaluation::update::output_rejected(&db, &evaluation, &block)?,
        AppError::Invalid("the AI drafted nothing for this output".to_owned())
    );

    lab_task::screen(&db, &evaluation, &task)
}

/// Saves the text the evaluator wrote for output `block`.
///
/// Called by the page: `OutputPanel.svelte`, "Save"
/// (`evaluationLab.outputsSave(evaluation, block, text)`).
/// Calls: `read::block`, `check_output`, `update::output_text`, `lab_task::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn outputs_save(
    state: State<'_, AppState>,
    evaluation: String,
    block: String,
    text: String,
) -> Result<LabTaskScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some((task, output)) = db_evaluation::read::block(&db, &evaluation, &block)? else {
        return Err(AppError::Invalid("the output does not exist".to_owned()));
    };

    ensure!(output.kind == BlockKind::Output, AppError::Invalid("only an output has a text".to_owned()));
    check_output(&text).map_err(|error| AppError::Invalid(error.to_string()))?;
    db_evaluation::update::output_text(&db, &evaluation, &block, text.trim())?;

    lab_task::screen(&db, &evaluation, &task)
}

/// Writes the report of evaluation `evaluation` into a folder the evaluator
/// chooses in a dialog of the system: one file per output; gives how many
/// files were written, `None` when no folder was chosen. Refused while no
/// output has an accepted text.
///
/// Called by the page: `EvaluationScreen.svelte`, "Export the report…"
/// (`evaluationLab.reportExport(evaluation)`).
/// Calls: `report_export_files::files`, `choose_folder`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn report_export<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    evaluation: String,
) -> Result<Option<usize>, AppError> {
    let session = state.session()?;
    let files = report_export_files::files(&session.db(), &evaluation)?;

    ensure!(!files.is_empty(), AppError::Invalid("no output has an accepted text yet".to_owned()));

    let Some(folder) = choose_folder(app).await? else {
        return Ok(None);
    };

    for file in &files {
        tokio::fs::write(folder.join(&file.name), &file.text)
            .await
            .map_err(|error| AppError::Failed(format!("the file {} was not written: {error}", file.name)))?;
    }

    Ok(Some(files.len()))
}

// ── The works of the AI of the view ───────────────────────────────────

/// Stops the work of the AI of the view, if there is one.
///
/// Called by the page: the panels of a task, "Stop" (`evaluationLab.evaluationLabStop()`).
/// Calls: `AppState::stop_work`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn evaluation_lab_stop(state: State<'_, AppState>) {
    state.stop_work();
}

/// The instructions `which` of a work of the AI of the view
/// (`"input_chat"`, `"evidence_fragments"`, `"assessment"`, `"output_draft"`):
/// those the evaluator changed, or the default ones.
///
/// Called by the page: `LabInstructions.svelte`, when its dialog opens
/// (`evaluationLab.evaluationLabInstructionsGet(which)`).
/// Calls: [`lab_task_of`], `tasks::instructions`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn evaluation_lab_instructions_get(state: State<'_, AppState>, which: String) -> Result<String, AppError> {
    let session = state.session()?;

    Ok(tasks::instructions(&session.db(), lab_task_of(&which)?)?.unwrap_or_default())
}

/// Stores the instructions `which`; an empty text brings the default ones
/// back. Gives the instructions now in force.
///
/// Called by the page: `LabInstructions.svelte`, "Save" and "Default"
/// (`evaluationLab.evaluationLabInstructionsSave(which, text)`).
/// Calls: [`lab_task_of`], `tasks::set_instructions`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn evaluation_lab_instructions_save(
    state: State<'_, AppState>,
    which: String,
    text: String,
) -> Result<String, AppError> {
    let session = state.session()?;
    let task = lab_task_of(&which)?;

    ensure!(text.len() <= INSTRUCTIONS_MAX, AppError::Invalid("the instructions are too long".to_owned()));

    Ok(tasks::set_instructions(&session.db(), task, &text)?.unwrap_or_default())
}

// ── Helpers of the commands above ─────────────────────────────────────

/// Stores the catalogs of `catalogs` that are not stored yet, checks that
/// every catalog the package points to is there, and stores `made` with the
/// frozen copy of `package`; refused when the package is imported already.
///
/// Called by: `package_import_import`.
/// Calls: `read::package_imported`, `db_oscal::read::summary`,
/// `db_oscal::create::document`, `db_evaluation::create::evaluation`.
fn store_package(
    db: &UserDb,
    made: &Evaluation,
    package: &Package,
    catalogs: &[OscalDocument],
) -> Result<(), AppError> {
    ensure!(
        !db_evaluation::read::package_imported(db, &package.package_uuid)?,
        AppError::Invalid("this work package is imported already".to_owned())
    );

    for catalog in catalogs {
        if db_oscal::read::summary(db, catalog.uuid())?.is_none() {
            db_oscal::create::document(db, catalog)?;
        }
    }

    for task in &package.tasks {
        ensure!(
            db_oscal::read::summary(db, &task.task.catalog_uuid)?.is_some(),
            AppError::Invalid(format!("the catalog of the task {} is neither attached nor stored", task.task.label))
        );

        for element in &task.related {
            ensure!(
                db_oscal::read::summary(db, &element.catalog_uuid)?.is_some(),
                AppError::Invalid(format!("the catalog of {} is neither attached nor stored", element.label))
            );
        }
    }

    db_evaluation::create::evaluation(db, made, package)?;

    Ok(())
}

/// Evaluation `uuid`; refused when it does not exist.
///
/// Called by: `lab_tasks_get`, `lab_tasks_state`.
fn evaluation_of(db: &UserDb, uuid: &str) -> Result<Evaluation, AppError> {
    match db_evaluation::read::evaluation(db, uuid)? {
        Some(found) => Ok(found),
        None => Err(AppError::Invalid("the evaluation does not exist".to_owned())),
    }
}

/// The task of block `block` of evaluation `evaluation`; refused when the
/// block does not exist.
///
/// Called by: the commands of an input or an output.
fn task_of_block(db: &UserDb, evaluation: &str, block: &str) -> Result<String, AppError> {
    match db_evaluation::read::block(db, evaluation, block)? {
        Some((task, _)) => Ok(task),
        None => Err(AppError::Invalid("the block does not exist".to_owned())),
    }
}

/// The task of the AI whose instructions are `which`: one of the works of
/// the AI of the view; refused for any other.
///
/// Called by: `evaluation_lab_instructions_get`, `evaluation_lab_instructions_save`.
/// Calls: `Task::from_name`.
fn lab_task_of(which: &str) -> Result<Task, AppError> {
    let Some(task) = Task::from_name(which) else {
        return Err(AppError::Invalid("there are no such instructions".to_owned()));
    };

    ensure!(
        matches!(task, Task::InputChat | Task::EvidenceFragments | Task::Assessment | Task::OutputDraft),
        AppError::Invalid("there are no such instructions".to_owned())
    );

    Ok(task)
}

#[cfg(test)]
mod tests {
    //! Tests of the commands of the view Evaluation lab, called as the page
    //! calls them: by name, through the IPC of Tauri. The import (a dialog
    //! of the system) is tested in `package_import/read.rs` and through
    //! [`store_package`]; the works of the AI in their own files.

    use super::*;

    use serde_json::{Value, json};

    use crate::test_support::db::{basic_catalog, lab_evaluation, lab_package};
    use crate::views::test_support::{TestWindow, logged_in_window};

    /// A window with "anna" logged in and the evaluation "e" of the package
    /// of the tests, its catalog stored by the import.
    fn with_evaluation() -> TestWindow {
        let window = logged_in_window();
        {
            let session = window.session();
            let catalog = basic_catalog();
            let package = lab_package(catalog.uuid());
            store_package(&session.db(), &lab_evaluation(&package), &package, &[catalog]).unwrap();
        }

        window
    }

    #[test]
    fn a_package_is_imported_once_with_its_catalogs() {
        let window = with_evaluation();
        let session = window.session();
        let catalog = basic_catalog();
        let package = lab_package(catalog.uuid());

        let again = Evaluation { uuid: "e2".to_owned(), ..lab_evaluation(&package) };
        assert!(store_package(&session.db(), &again, &package, &[]).is_err(), "imported already");

        let foreign = lab_package("no-such-catalog");
        let other = Evaluation { uuid: "e3".to_owned(), package_uuid: "w3".to_owned(), ..lab_evaluation(&foreign) };
        let mut moved = foreign.clone();
        moved.package_uuid = "w3".to_owned();
        assert!(store_package(&session.db(), &other, &moved, &[]).is_err(), "a catalog neither attached nor stored");
        drop(session);

        let list = window.call("evaluation_list_get", json!({})).unwrap();
        assert_eq!((list.as_array().unwrap().len(), list[0]["product"].clone()), (1, json!("Reader")));
    }

    #[test]
    fn the_screen_of_a_task_shows_its_scheme_requirement_and_outputs() {
        let window = with_evaluation();

        let evaluation = window.call("lab_tasks_get", json!({ "evaluation": "e" })).unwrap();
        assert_eq!(
            (evaluation["tasks"][0]["label"].clone(), evaluation["tasks"][0]["state"].clone()),
            (json!("1.1.1"), json!("todo"))
        );

        let screen = window.call("lab_task_get", json!({ "evaluation": "e", "task": "t" })).unwrap();
        assert_eq!(screen["blocks"].as_array().unwrap().len(), 4);
        assert_eq!(screen["requirement"]["title"], "Information security roles and responsibilities");
        assert_eq!(screen["inputs"][0]["block"], "i");
        assert_eq!(
            (screen["outputs"][0]["applies"].clone(), screen["outputs"][1]["applies"].clone()),
            (json!(true), json!(false))
        );
        assert_eq!(screen["result_titles"][2], "Assessment and verdict proposition");

        let done =
            window.call("lab_tasks_state", json!({ "evaluation": "e", "task": "t", "progress": "done" })).unwrap();
        assert_eq!(done["tasks"][0]["state"], "done");
        let wrong = window.call("lab_tasks_state", json!({ "evaluation": "e", "task": "t", "progress": "half" }));
        assert_eq!(wrong.unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn evidence_and_the_fragments_of_the_evaluator_are_kept() {
        let window = with_evaluation();

        let screen = window
            .call(
                "evidence_add_note",
                json!({ "evaluation": "e", "block": "i", "name": "Interview", "text": "Anna owns the keys." }),
            )
            .unwrap();
        let evidence = screen["inputs"][0]["evidence"][0]["uuid"].clone();
        assert_eq!(screen["state"], "working");

        let fragment = |quote: &str| json!({ "evidence": evidence, "page": 1, "quote": quote, "note": "" });
        window
            .call("fragments_add", json!({ "evaluation": "e", "block": "i", "fragment": fragment("Anna owns") }))
            .unwrap();
        let screen = window
            .call("fragments_add", json!({ "evaluation": "e", "block": "i", "fragment": fragment("Bob owns") }))
            .unwrap();
        let fragments = &screen["inputs"][0]["fragments"];
        assert_eq!((fragments[0]["found"].clone(), fragments[1]["found"].clone()), (json!(true), json!(false)));
        assert_eq!(fragments[0]["accepted"], true, "the evaluator's own fragment is accepted");

        let screen = window
            .call("fragments_remove", json!({ "evaluation": "e", "task": "t", "fragment": fragments[1]["uuid"] }))
            .unwrap();
        assert_eq!(screen["inputs"][0]["fragments"].as_array().unwrap().len(), 1);

        let screen =
            window.call("evidence_delete", json!({ "evaluation": "e", "task": "t", "evidence": evidence })).unwrap();
        assert_eq!(
            (screen["inputs"][0]["evidence"].clone(), screen["inputs"][0]["fragments"].clone()),
            (json!([]), json!([]))
        );
        let output =
            window.call("evidence_add_note", json!({ "evaluation": "e", "block": "o", "name": "x", "text": "y" }));
        assert_eq!(output.unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn the_results_and_the_verdict_decide_which_outputs_are_written() {
        let window = with_evaluation();
        let results = |verdict: Value| json!({ "findings": "Anna owns the keys.", "analysis": "", "assessment": "", "vulnerabilities": "", "verdict": verdict });

        let screen = window
            .call("assessment_save", json!({ "evaluation": "e", "task": "t", "results": results(json!("not_met")) }))
            .unwrap();
        assert_eq!(screen["results"]["verdict"], "not_met");
        assert_eq!(screen["outputs"][1]["applies"], true, "the Observation Report is written");

        let screen = window
            .call("outputs_save", json!({ "evaluation": "e", "block": "n", "text": " An observation. " }))
            .unwrap();
        assert_eq!(screen["outputs"][1]["text"], "An observation.");
        let wrong = window
            .call("assessment_save", json!({ "evaluation": "e", "task": "t", "results": results(json!("pass")) }));
        assert_eq!(wrong.unwrap_err()["kind"], "invalid");
        assert_eq!(
            window.call("assessment_accept", json!({ "evaluation": "e", "task": "t" })).unwrap_err()["kind"],
            "invalid"
        );
        assert_eq!(
            window.call("outputs_accept", json!({ "evaluation": "e", "block": "o" })).unwrap_err()["kind"],
            "invalid"
        );
        let input = window.call("outputs_save", json!({ "evaluation": "e", "block": "i", "text": "x" }));
        assert_eq!(input.unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn a_proposal_of_the_agent_is_accepted_or_rejected() {
        let window = with_evaluation();
        {
            let session = window.session();
            let mut proposed = crate::model::evaluation::Results::empty();
            proposed.findings = "F".to_owned();
            proposed.verdict = Some(crate::model::evaluation::Verdict::Met);
            db_evaluation::update::results_proposal(&session.db(), "e", "t", &proposed, "Why.").unwrap();
            db_evaluation::update::output_proposal(&session.db(), "e", "o", "Drafted.").unwrap();
        }

        let screen = window.call("lab_task_get", json!({ "evaluation": "e", "task": "t" })).unwrap();
        assert_eq!(screen["proposal"]["reason"], "Why.");
        let screen = window.call("assessment_accept", json!({ "evaluation": "e", "task": "t" })).unwrap();
        assert_eq!((screen["results"]["verdict"].clone(), screen["proposal"].clone()), (json!("met"), Value::Null));

        let screen = window.call("outputs_accept", json!({ "evaluation": "e", "block": "o" })).unwrap();
        assert_eq!(
            (screen["outputs"][0]["text"].clone(), screen["outputs"][0]["proposal"].clone()),
            (json!("Drafted."), Value::Null)
        );
        assert_eq!(
            window.call("outputs_reject", json!({ "evaluation": "e", "block": "o" })).unwrap_err()["kind"],
            "invalid"
        );
        assert_eq!(
            window.call("assessment_reject", json!({ "evaluation": "e", "task": "t" })).unwrap_err()["kind"],
            "invalid"
        );
    }

    #[test]
    fn an_evaluation_is_deleted_and_the_instructions_are_changed() {
        let window = with_evaluation();

        assert_eq!(window.call("evaluation_list_delete", json!({ "uuid": "e" })).unwrap(), json!([]));
        assert_eq!(window.call("evaluation_list_delete", json!({ "uuid": "e" })).unwrap_err()["kind"], "invalid");

        let default = window.call("evaluation_lab_instructions_get", json!({ "which": "assessment" })).unwrap();
        assert!(default.as_str().unwrap().contains("four results"));
        let saved =
            window.call("evaluation_lab_instructions_save", json!({ "which": "assessment", "text": "Mine." })).unwrap();
        assert_eq!(saved, "Mine.");
        let other = window.call("evaluation_lab_instructions_get", json!({ "which": "task_scheme" }));
        assert_eq!(other.unwrap_err()["kind"], "invalid", "the instructions of the plan belong to the plan");
    }

    #[test]
    fn an_agent_is_chosen_for_the_evaluation_and_changed_for_a_task() {
        let window = with_evaluation();
        let agent = {
            let session = window.session();
            let fields = crate::views::knowledge::agent_form::view::new_agent("CC agent");
            crate::db::knowledge::create::agent(&session.db(), &fields).unwrap()
        };

        let view = window.call("lab_tasks_agent", json!({ "evaluation": "e", "agent": agent })).unwrap();
        assert_eq!((view["agent"].clone(), view["agents"][0]["name"].clone()), (json!(agent), json!("CC agent")));
        assert_eq!(
            window.call("lab_tasks_agent", json!({ "evaluation": "e", "agent": 99 })).unwrap_err()["kind"],
            "invalid"
        );

        let screen = window.call("lab_task_get", json!({ "evaluation": "e", "task": "t" })).unwrap();
        assert_eq!(
            (screen["knowledge"]["choice"].clone(), screen["knowledge"]["used"].clone()),
            (json!("evaluation"), json!("CC agent"))
        );

        let none = json!({ "evaluation": "e", "task": "t", "choice": "none", "agent": null });
        let screen = window.call("assessment_agent", none).unwrap();
        assert_eq!(
            (screen["knowledge"]["choice"].clone(), screen["knowledge"]["used"].clone()),
            (json!("none"), Value::Null)
        );

        let wrong = json!({ "evaluation": "e", "task": "t", "choice": "agent", "agent": null });
        assert_eq!(window.call("assessment_agent", wrong).unwrap_err()["kind"], "invalid");
        let ticked = json!({ "evaluation": "e", "task": "t", "passage": 5, "chosen": false });
        assert!(window.call("assessment_context", ticked).is_ok());
    }

    #[test]
    fn the_screen_of_an_evaluation_shows_how_far_every_task_is() {
        let window = with_evaluation();

        let view = window.call("lab_tasks_get", json!({ "evaluation": "e" })).unwrap();
        assert_eq!(view["tasks"][0]["verdict"], Value::Null);
        assert_eq!(
            (view["tasks"][0]["outputs_due"].clone(), view["tasks"][0]["outputs_ready"].clone()),
            (json!(1), json!(0))
        );
        assert_eq!((view["progress"]["no_verdict"].clone(), view["progress"]["tasks"].clone()), (json!(1), json!(1)));

        {
            let session = window.session();
            let mut results = crate::model::evaluation::Results::empty();
            results.verdict = Some(crate::model::evaluation::Verdict::NotMet);
            db_evaluation::update::results(&session.db(), "e", "t", &results).unwrap();
            db_evaluation::update::output_text(&session.db(), "e", "o", "The pETR text.").unwrap();
        }

        let view = window.call("lab_tasks_get", json!({ "evaluation": "e" })).unwrap();
        assert_eq!(view["tasks"][0]["verdict"], "not_met");
        // pETR (always) is ready; the Observation Report (only when not met) is due now, not written.
        assert_eq!(
            (view["tasks"][0]["outputs_due"].clone(), view["tasks"][0]["outputs_ready"].clone()),
            (json!(2), json!(1))
        );
        assert_eq!(
            (view["progress"]["not_met"].clone(), view["progress"]["outputs_ready"].clone()),
            (json!(1), json!(1))
        );
    }
}
