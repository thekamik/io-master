//! The entry of the view Evaluation plan: it works only on the documents of
//! OSCAL (page: `ui/src/views/evaluation_plan/`, its commands in
//! `ui/src/lib/api/evaluation_plan.ts`): the master projects (step 51 of
//! docs/todo.md), each a list of tasks, every task a whole requirement of one
//! of its catalogs, chosen in a browser of the standards; and the documents
//! of OSCAL (the list, the import, the export, the deletion). The catalogs
//! are made from the PDFs of a standard in the view Converter
//! (`views/converter/`). The files are chosen in the dialogs of the system,
//! opened in Rust: the page never sees a path. The flow of data:
//! `docs/architecture.md`, section 12.

use tauri::{AppHandle, Runtime, State};

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::conversion as db_conversion;
use crate::db::evaluation_project as db_evaluation_project;
use crate::db::oscal as db_oscal;
use crate::db::oscal::ids::new_uuid;
use crate::db::user::database::UserDb;

use super::oscal_documents::dialog::{choose_file, choose_place};
use super::oscal_documents::file::{self, read_file, safe_name};
use crate::model::evaluation_project::{self, EvaluationTask};
use crate::model::oscal::catalog::Catalog;
use crate::model::own_requirement;
use crate::model::task_scheme::{self, Block, BlockFields, BlockKind, StandardOutput};

use super::new_project::catalogs;
use super::new_project::view::CatalogChoiceView;
use super::oscal_documents::view::{self as oscal_view, OscalDocumentView};
use super::own_requirement::catalog as own_catalog;
use super::own_requirement::view::OwnRequirementForm;
use super::project_form::view::{ProjectForm, ProjectView};
use super::project_list::view::{self as project_view, ProjectSummaryView};
use super::standard_browser::requirement;
use super::standard_browser::search;
use super::standard_browser::view::{HitView, RequirementView};
use super::standard_outputs::view::{self as output_view, StandardOutputForm, StandardOutputView};
use super::task_list::view::{self as task_view, TaskView};
use super::task_scheme::scheme;
use super::task_scheme::view::{BlockForm, SchemeView};

// ── When the view opens ───────────────────────────────────────────────

/// The documents of OSCAL of the evaluator, by model and title.
///
/// Called by the page: `OscalDocuments.svelte`, when the view opens and after
/// a change (`evaluation_plan.oscalDocumentsGet()`).
/// Only the finished documents: a catalog still in conversion is shown only
/// in the view Converter (user, 2026-10-24).
///
/// Calls: `db_oscal::read::summaries`, `db_conversion::read::finished`,
/// `oscal_view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn oscal_documents_get(state: State<'_, AppState>) -> Result<Vec<OscalDocumentView>, AppError> {
    let session = state.session()?;
    let db = session.db();
    let summaries = db_oscal::read::summaries(&db)?;
    let mut unfinished = Vec::new();

    for summary in &summaries {
        if !db_conversion::read::finished(&db, &summary.uuid)? {
            unfinished.push(summary.uuid.clone());
        }
    }

    Ok(oscal_view::all(summaries, &unfinished))
}

/// The master projects of the evaluator, the newest change first.
///
/// Called by the page: `ProjectList.svelte`, when the view opens (the tab
/// Projects) and after a change (`evaluationPlan.projectListGet()`).
/// Calls: `db_evaluation_project::read::projects`, `project_view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn project_list_get(state: State<'_, AppState>) -> Result<Vec<ProjectSummaryView>, AppError> {
    let session = state.session()?;

    Ok(project_view::all(db_evaluation_project::read::projects(&session.db())?))
}

/// Project `uuid` in its form: what the planner wrote, its catalogs, and the
/// finished catalogs it could use.
///
/// Called by the page: `ProjectForm.svelte`, when a project opens and after
/// a change (`evaluationPlan.projectFormGet(uuid)`).
/// Calls: `db_evaluation_project::read::project`, `catalogs::finished`,
/// [`project_view_of`].
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn project_form_get(state: State<'_, AppState>, uuid: String) -> Result<ProjectView, AppError> {
    let session = state.session()?;

    project_view_of(&session.db(), &uuid)
}

/// The tasks of project `project`, in their order, each with its
/// requirement read from its catalog.
///
/// Called by the page: `TaskList.svelte`, when a project opens and after a
/// change (`evaluationPlan.taskListGet(project)`).
/// Calls: `db_evaluation_project::read::tasks`, `task_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_list_get(state: State<'_, AppState>, project: String) -> Result<Vec<TaskView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    task_view::all(&db, db_evaluation_project::read::tasks(&db, &project)?)
}

// ── The actions of the elements ───────────────────────────────────────

/// Asks for a file of OSCAL (JSON) in a dialog of the system and imports
/// it; `None` when no file was chosen.
///
/// Called by the page: `OscalDocuments.svelte`, "Import an OSCAL document"
/// (`evaluation_plan.oscalDocumentsImport()`).
/// Calls: `choose_file` (the dialog), `read_file` (at most `MAX_FILE`),
/// `file::import` (checked against the schema of NIST, stored).
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn oscal_documents_import<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Option<OscalDocumentView>, AppError> {
    let session = state.session()?;

    let Some(path) = choose_file(app).await? else {
        return Ok(None);
    };

    let text = read_file(path).await?;
    let summary = file::import(&session.db(), &text)?;

    Ok(Some(OscalDocumentView::new(summary)))
}

/// Asks where to write document `uuid` in a dialog of the system (the
/// name proposed: `file_name`) and writes it; `false` when no place was
/// chosen.
///
/// Called by the page: `OscalDocuments.svelte`, "Export"
/// (`evaluation_plan.oscalDocumentsExport(...)`).
/// Calls: `file::export`, `safe_name`, `choose_place` (the dialog),
/// `tokio::fs::write`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn oscal_documents_export<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    uuid: String,
    file_name: String,
) -> Result<bool, AppError> {
    let session = state.session()?;
    let text = file::export(&session.db(), &uuid)?;

    let Some(path) = choose_place(app, safe_name(&file_name)).await? else {
        return Ok(false);
    };

    match tokio::fs::write(&path, text).await {
        Ok(()) => Ok(true),
        Err(error) => Err(AppError::Failed(format!("the file was not written: {error}"))),
    }
}

/// Deletes document `uuid` with its evaluation support.
///
/// Called by the page: `OscalDocuments.svelte`, "Delete"
/// (`evaluation_plan.oscalDocumentsDelete(uuid)`).
/// Refused while a master project uses the catalog.
/// Calls: `db_evaluation_project::read::catalog_in_use`, `file::delete` (the
/// document, its evaluation support and its conversion).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn oscal_documents_delete(state: State<'_, AppState>, uuid: String) -> Result<(), AppError> {
    let session = state.session()?;
    let db = session.db();

    ensure!(
        !db_evaluation_project::read::catalog_in_use(&db, &uuid)?,
        AppError::Invalid("a master project uses this catalog: remove it from the project first".to_owned())
    );
    ensure!(file::delete(&db, &uuid)?, AppError::Invalid("the OSCAL document does not exist".to_owned()));

    Ok(())
}

/// The finished catalogs a new project can use.
///
/// Called by the page: `NewProject.svelte`, when its dialog opens
/// (`evaluationPlan.newProjectCatalogs()`).
/// Calls: `catalogs::finished`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_project_catalogs(state: State<'_, AppState>) -> Result<Vec<CatalogChoiceView>, AppError> {
    let session = state.session()?;

    catalogs::finished(&session.db())
}

/// Makes a new master project of the fields and catalogs of `project`.
///
/// Called by the page: `NewProject.svelte`, "Make"
/// (`evaluationPlan.newProjectCreate(project)`).
/// Calls: `evaluation_project::normalized` and `check_fields`,
/// `catalogs::check`, `new_uuid`, `db_evaluation_project::create::project`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_project_create(state: State<'_, AppState>, project: ProjectForm) -> Result<ProjectSummaryView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let fields = evaluation_project::normalized(&project.fields());

    evaluation_project::check_fields(&fields).map_err(|error| AppError::Invalid(error.to_string()))?;
    catalogs::check(&db, &project.catalogs)?;

    let uuid = new_uuid()?;
    db_evaluation_project::create::project(&db, &uuid, &fields, &project.catalogs)?;

    Ok(ProjectSummaryView { uuid, title: fields.title, catalogs: project.catalogs.len(), tasks: 0 })
}

/// Deletes project `uuid` with its tasks; its catalogs stay.
///
/// Called by the page: `ProjectList.svelte`, "Delete"
/// (`evaluationPlan.projectListDelete(uuid)`).
/// Calls: `db_evaluation_project::delete::project`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn project_list_delete(state: State<'_, AppState>, uuid: String) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(
        db_evaluation_project::delete::project(&session.db(), &uuid)?,
        AppError::Invalid("the project does not exist".to_owned())
    );

    Ok(())
}

/// Saves the fields and the catalogs of project `uuid`; a catalog a task
/// still points to cannot leave the project.
///
/// Called by the page: `ProjectForm.svelte`, "Save"
/// (`evaluationPlan.projectFormSave(uuid, project)`).
/// Calls: `evaluation_project::normalized` and `check_fields`,
/// `catalogs::check`, `db_evaluation_project::read`, `update::project`,
/// [`project_view_of`].
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn project_form_save(
    state: State<'_, AppState>,
    uuid: String,
    project: ProjectForm,
) -> Result<ProjectView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let fields = evaluation_project::normalized(&project.fields());

    evaluation_project::check_fields(&fields).map_err(|error| AppError::Invalid(error.to_string()))?;
    catalogs::check(&db, &project.catalogs)?;

    let Some(current) = db_evaluation_project::read::project(&db, &uuid)? else {
        return Err(AppError::Invalid("the project does not exist".to_owned()));
    };

    for catalog in &current.catalogs {
        ensure!(
            project.catalogs.contains(catalog) || !db_evaluation_project::read::catalog_has_tasks(&db, &uuid, catalog)?,
            AppError::Invalid("a catalog with tasks cannot leave the project: remove its tasks first".to_owned())
        );
    }

    db_evaluation_project::update::project(&db, &uuid, &fields, &project.catalogs)?;

    project_view_of(&db, &uuid)
}

/// Adds control `control` of catalog `catalog` as a task at the end of the
/// list of project `project`: a whole requirement, once in a project.
///
/// Called by the page: `StandardBrowser.svelte`, "Add as a task"
/// (`evaluationPlan.taskListAdd(project, catalog, control)`).
/// Calls: [`project_catalog`], `Catalog::locate`, `new_uuid`, `scheme::first`
/// (the outputs of the template), `db_evaluation_project::create::task`,
/// `task_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_list_add(
    state: State<'_, AppState>,
    project: String,
    catalog: String,
    control: String,
) -> Result<Vec<TaskView>, AppError> {
    let session = state.session()?;
    let db = session.db();
    let found = project_catalog(&db, &project, &catalog)?;

    let is_control = match found.locate(&control)? {
        Some(location) => found.control_at(&location).is_some(),
        None => false,
    };

    ensure!(is_control, AppError::Invalid("the catalog has no such requirement".to_owned()));

    for task in db_evaluation_project::read::tasks(&db, &project)? {
        ensure!(
            !(task.catalog_uuid == catalog && task.control_id == control),
            AppError::Invalid("the requirement is a task of the project already".to_owned())
        );
    }

    let task = EvaluationTask { uuid: new_uuid()?, catalog_uuid: catalog, control_id: control };
    db_evaluation_project::create::task(&db, &project, &task, &scheme::first(&db, &project)?)?;

    task_view::all(&db, db_evaluation_project::read::tasks(&db, &project)?)
}

/// Removes task `task` from its project; gives the tasks left.
///
/// Called by the page: `TaskList.svelte`, "✕" (`evaluationPlan.taskListRemove(task)`).
/// Calls: `db_evaluation_project::read::task`, `delete::task`, `task_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_list_remove(state: State<'_, AppState>, task: String) -> Result<Vec<TaskView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some((project, _)) = db_evaluation_project::read::task(&db, &task)? else {
        return Err(AppError::Invalid("the task does not exist".to_owned()));
    };

    ensure!(db_evaluation_project::delete::task(&db, &task)?, AppError::Invalid("the task does not exist".to_owned()));

    task_view::all(&db, db_evaluation_project::read::tasks(&db, &project)?)
}

/// Moves task `task` one place up or down in its list; gives the tasks.
///
/// Called by the page: `TaskList.svelte`, "↑" and "↓"
/// (`evaluationPlan.taskListMove(task, up)`).
/// Calls: `db_evaluation_project::read::task`, `update::move_task`,
/// `task_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_list_move(state: State<'_, AppState>, task: String, up: bool) -> Result<Vec<TaskView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some((project, _)) = db_evaluation_project::read::task(&db, &task)? else {
        return Err(AppError::Invalid("the task does not exist".to_owned()));
    };

    db_evaluation_project::update::move_task(&db, &task, up)?;

    task_view::all(&db, db_evaluation_project::read::tasks(&db, &project)?)
}

/// Requirement `id` of catalog `catalog` of project `project`, as the
/// browser shows it, with whether it is a task already.
///
/// Called by the page: `StandardBrowser.svelte`, a requirement chosen in the
/// tree or in the hits (`evaluationPlan.standardBrowserGet(project, catalog, id)`).
/// Calls: [`project_catalog`], `db_evaluation_project::read::tasks`,
/// `own_catalog::existing`, `requirement::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_browser_get(
    state: State<'_, AppState>,
    project: String,
    catalog: String,
    id: String,
) -> Result<RequirementView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let found = project_catalog(&db, &project, &catalog)?;
    let mut in_project = false;

    for task in db_evaluation_project::read::tasks(&db, &project)? {
        if task.catalog_uuid == catalog && task.control_id == id {
            in_project = true;
        }
    }

    let own = own_catalog::existing(&db)?.as_deref() == Some(catalog.as_str());

    requirement::read(&db, &catalog, &found, &id, in_project, own)
}

/// The requirements of the catalogs of project `project` that hold `text`.
///
/// Called by the page: `StandardBrowser.svelte`, "Search"
/// (`evaluationPlan.standardBrowserSearch(project, text)`).
/// Calls: `db_evaluation_project::read::project`, `db_oscal::read::catalog`,
/// `search::search`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_browser_search(
    state: State<'_, AppState>,
    project: String,
    text: String,
) -> Result<Vec<HitView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some(found) = db_evaluation_project::read::project(&db, &project)? else {
        return Err(AppError::Invalid("the project does not exist".to_owned()));
    };

    let mut catalogs = Vec::with_capacity(found.catalogs.len());

    for uuid in found.catalogs {
        let catalog = db_oscal::read::catalog(&db, &uuid)?;
        catalogs.push((uuid, catalog));
    }

    search::search(&catalogs, &text)
}

/// Writes `requirement`, a requirement of the laboratory's own, into the
/// shared catalog of the laboratory (made now if there is none), adds that
/// catalog to project `project` and the requirement as a task at the end of
/// its list. Its identifier is the planner's, whatever they chose.
///
/// Called by the page: `OwnRequirement.svelte`, "Add"
/// (`evaluationPlan.ownRequirementAdd(project, requirement)`).
/// Calls: `own_requirement::normalized` and `check`, `own_catalog::ensure`
/// and `add`, `db_evaluation_project::update::add_catalog`, `scheme::first`,
/// `db_evaluation_project::create::task`, `task_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn own_requirement_add(
    state: State<'_, AppState>,
    project: String,
    requirement: OwnRequirementForm,
) -> Result<Vec<TaskView>, AppError> {
    let session = state.session()?;
    let db = session.db();
    let written = own_requirement::normalized(&requirement.requirement());

    own_requirement::check(&written).map_err(|error| AppError::Invalid(error.to_string()))?;
    ensure!(
        db_evaluation_project::read::project(&db, &project)?.is_some(),
        AppError::Invalid("the project does not exist".to_owned())
    );

    let catalog = own_catalog::ensure(&db)?;
    let control = own_catalog::add(&db, &catalog, &written)?;

    db_evaluation_project::update::add_catalog(&db, &project, &catalog)?;

    let task = EvaluationTask { uuid: new_uuid()?, catalog_uuid: catalog, control_id: control };
    db_evaluation_project::create::task(&db, &project, &task, &scheme::first(&db, &project)?)?;

    task_view::all(&db, db_evaluation_project::read::tasks(&db, &project)?)
}

/// Changes requirement `id` of the laboratory: for every project whose task
/// points to it. Its id stays, so the tasks keep pointing to it.
///
/// Called by the page: `OwnRequirement.svelte`, "Save"
/// (`evaluationPlan.ownRequirementSave(id, requirement)`).
/// Calls: `own_requirement::normalized` and `check`, `own_catalog::existing`
/// and `save`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn own_requirement_save(
    state: State<'_, AppState>,
    id: String,
    requirement: OwnRequirementForm,
) -> Result<(), AppError> {
    let session = state.session()?;
    let db = session.db();
    let written = own_requirement::normalized(&requirement.requirement());

    own_requirement::check(&written).map_err(|error| AppError::Invalid(error.to_string()))?;

    let Some(catalog) = own_catalog::existing(&db)? else {
        return Err(AppError::Invalid("there is no requirement of the laboratory yet".to_owned()));
    };

    own_catalog::save(&db, &catalog, &id, &written)
}

/// Deletes requirement `id` of the laboratory; refused while a task of any
/// project points to it.
///
/// Called by the page: `StandardBrowser.svelte`, "Delete" of a requirement of
/// the laboratory (`evaluationPlan.ownRequirementDelete(id)`).
/// Calls: `own_catalog::existing`, `db_evaluation_project::read::control_in_use`,
/// `own_catalog::remove`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn own_requirement_delete(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some(catalog) = own_catalog::existing(&db)? else {
        return Err(AppError::Invalid("there is no requirement of the laboratory yet".to_owned()));
    };

    ensure!(
        !db_evaluation_project::read::control_in_use(&db, &catalog, &id)?,
        AppError::Invalid("a task points to this requirement: remove the task first".to_owned())
    );

    own_catalog::remove(&db, &catalog, &id)
}

/// The template of the outputs of project `project` ("Standard outputs",
/// step 51.3): the default outputs of every new task, in order.
///
/// Called by the page: `StandardOutputs.svelte`, when a project opens
/// (`evaluationPlan.standardOutputsGet(project)`).
/// Calls: `db_evaluation_project::read::standard_outputs`, `output_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_outputs_get(state: State<'_, AppState>, project: String) -> Result<Vec<StandardOutputView>, AppError> {
    let session = state.session()?;

    Ok(output_view::all(db_evaluation_project::read::standard_outputs(&session.db(), &project)?))
}

/// Adds `output` at the end of the template of project `project`; the tasks
/// added from now on get it.
///
/// Called by the page: `StandardOutputs.svelte`, "Add"
/// (`evaluationPlan.standardOutputsAdd(project, output)`).
/// Calls: `task_scheme::normalized` and `check_fields`, `new_uuid`,
/// `db_evaluation_project::create::standard_output`, `output_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_outputs_add(
    state: State<'_, AppState>,
    project: String,
    output: StandardOutputForm,
) -> Result<Vec<StandardOutputView>, AppError> {
    let session = state.session()?;
    let db = session.db();
    let fields = task_scheme::normalized(&output.fields()?);

    task_scheme::check_fields(BlockKind::Output, &fields).map_err(|error| AppError::Invalid(error.to_string()))?;
    ensure!(
        db_evaluation_project::read::project(&db, &project)?.is_some(),
        AppError::Invalid("the project does not exist".to_owned())
    );

    db_evaluation_project::create::standard_output(&db, &project, &StandardOutput { uuid: new_uuid()?, fields })?;

    Ok(output_view::all(db_evaluation_project::read::standard_outputs(&db, &project)?))
}

/// Saves the fields of standard output `output`; the tasks made before keep
/// their copies.
///
/// Called by the page: `StandardOutputs.svelte`, "Save"
/// (`evaluationPlan.standardOutputsSave(output, fields)`).
/// Calls: `task_scheme::normalized` and `check_fields`,
/// [`standard_output_project`], `db_evaluation_project::update::standard_output`,
/// `output_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_outputs_save(
    state: State<'_, AppState>,
    output: String,
    fields: StandardOutputForm,
) -> Result<Vec<StandardOutputView>, AppError> {
    let session = state.session()?;
    let db = session.db();
    let fields = task_scheme::normalized(&fields.fields()?);

    task_scheme::check_fields(BlockKind::Output, &fields).map_err(|error| AppError::Invalid(error.to_string()))?;
    let project = standard_output_project(&db, &output)?;
    db_evaluation_project::update::standard_output(&db, &output, &fields)?;

    Ok(output_view::all(db_evaluation_project::read::standard_outputs(&db, &project)?))
}

/// Deletes standard output `output` from its template; the tasks keep their
/// copies.
///
/// Called by the page: `StandardOutputs.svelte`, "Delete"
/// (`evaluationPlan.standardOutputsDelete(output)`).
/// Calls: [`standard_output_project`],
/// `db_evaluation_project::delete::standard_output`, `output_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_outputs_delete(
    state: State<'_, AppState>,
    output: String,
) -> Result<Vec<StandardOutputView>, AppError> {
    let session = state.session()?;
    let db = session.db();
    let project = standard_output_project(&db, &output)?;

    ensure!(
        db_evaluation_project::delete::standard_output(&db, &output)?,
        AppError::Invalid("the standard output does not exist".to_owned())
    );

    Ok(output_view::all(db_evaluation_project::read::standard_outputs(&db, &project)?))
}

/// The scheme of task `task`: its requirement as the standard writes it,
/// its blocks and connections (step 51.2 of docs/todo.md).
///
/// Called by the page: `TaskScheme.svelte`, when a task opens and after a
/// change (`evaluationPlan.taskSchemeGet(task)`).
/// Calls: `scheme::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_scheme_get(state: State<'_, AppState>, task: String) -> Result<SchemeView, AppError> {
    let session = state.session()?;

    scheme::read(&session.db(), &task)
}

/// Adds a block of `kind` (`"input"` or `"output"`) at `x`, `y` to the
/// scheme of task `task`; the planner names and connects it.
///
/// Called by the page: `TaskScheme.svelte`, "+ Input" and "+ Output"
/// (`evaluationPlan.taskSchemeAddBlock(task, kind, x, y)`).
/// Calls: `task_scheme::check_position` and `check_new_block`,
/// `db_evaluation_project::create::block`, `scheme::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_scheme_add_block(
    state: State<'_, AppState>,
    task: String,
    kind: String,
    x: i64,
    y: i64,
) -> Result<SchemeView, AppError> {
    let session = state.session()?;
    let db = session.db();

    let kind = match BlockKind::from_name(&kind) {
        Some(kind @ (BlockKind::Input | BlockKind::Output)) => kind,
        _ => return Err(AppError::Invalid("only an input or an output is added".to_owned())),
    };

    task_scheme::check_position(x, y).map_err(|error| AppError::Invalid(error.to_string()))?;
    let blocks = db_evaluation_project::read::blocks(&db, &task)?;
    task_scheme::check_new_block(&blocks, kind).map_err(|error| AppError::Invalid(error.to_string()))?;

    let mut fields = BlockFields::empty();
    fields.name = if kind == BlockKind::Input { "Input".to_owned() } else { "Output".to_owned() };

    let block = Block { uuid: new_uuid()?, kind, x, y, fields };
    db_evaluation_project::create::block(&db, &task, &block)?;

    scheme::read(&db, &task)
}

/// Saves the fields of block `block`.
///
/// Called by the page: `BlockForm.svelte`, "Save"
/// (`evaluationPlan.taskSchemeSaveBlock(block, fields)`).
/// Calls: `scheme::block`, `task_scheme::normalized` and `check_fields`,
/// `db_evaluation_project::update::block_fields`, `scheme::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_scheme_save_block(
    state: State<'_, AppState>,
    block: String,
    fields: BlockForm,
) -> Result<SchemeView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let (task, found) = scheme::block(&db, &block)?;
    let fields = task_scheme::normalized(&fields.fields()?);

    task_scheme::check_fields(found.kind, &fields).map_err(|error| AppError::Invalid(error.to_string()))?;
    db_evaluation_project::update::block_fields(&db, &block, &fields)?;

    scheme::read(&db, &task)
}

/// Places block `block` at `x`, `y`: where a drag ended (only the end is
/// sent, never the steps of the drag).
///
/// Called by the page: `TaskScheme.svelte`, the end of a drag
/// (`evaluationPlan.taskSchemeMoveBlock(block, x, y)`).
/// Calls: `task_scheme::check_position`, `scheme::block`,
/// `db_evaluation_project::update::block_position`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_scheme_move_block(state: State<'_, AppState>, block: String, x: i64, y: i64) -> Result<(), AppError> {
    let session = state.session()?;
    let db = session.db();

    task_scheme::check_position(x, y).map_err(|error| AppError::Invalid(error.to_string()))?;
    scheme::block(&db, &block)?;
    db_evaluation_project::update::block_position(&db, &block, x, y)?;

    Ok(())
}

/// Removes block `block` (an input or an output) with its connections; the
/// requirement stays.
///
/// Called by the page: `BlockForm.svelte`, "Remove"
/// (`evaluationPlan.taskSchemeRemoveBlock(block)`).
/// Calls: `scheme::block`, `db_evaluation_project::delete::block`,
/// `scheme::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_scheme_remove_block(state: State<'_, AppState>, block: String) -> Result<SchemeView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let (task, found) = scheme::block(&db, &block)?;

    ensure!(found.kind != BlockKind::Requirement, AppError::Invalid("the requirement stays in its task".to_owned()));
    ensure!(
        db_evaluation_project::delete::block(&db, &block)?,
        AppError::Invalid("the block does not exist".to_owned())
    );

    scheme::read(&db, &task)
}

/// Connects block `from` to block `to` of one task: an input to the
/// requirement, or the requirement to an output.
///
/// Called by the page: `TaskScheme.svelte`, a connection drawn
/// (`evaluationPlan.taskSchemeConnect(from, to)`).
/// Calls: `scheme::block`, `task_scheme::may_connect`,
/// `db_evaluation_project::create::connection`, `scheme::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_scheme_connect(state: State<'_, AppState>, from: String, to: String) -> Result<SchemeView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let (task, start) = scheme::block(&db, &from)?;
    let (other_task, end) = scheme::block(&db, &to)?;

    ensure!(task == other_task, AppError::Invalid("the blocks belong to two tasks".to_owned()));
    ensure!(
        task_scheme::may_connect(start.kind, end.kind),
        AppError::Invalid("an input connects to the requirement, the requirement to an output".to_owned())
    );

    db_evaluation_project::create::connection(&db, &task, &from, &to)?;

    scheme::read(&db, &task)
}

/// Removes the connection of block `from` to block `to`.
///
/// Called by the page: `TaskScheme.svelte`, a connection removed
/// (`evaluationPlan.taskSchemeDisconnect(from, to)`).
/// Calls: `scheme::block`, `db_evaluation_project::delete::connection`,
/// `scheme::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn task_scheme_disconnect(state: State<'_, AppState>, from: String, to: String) -> Result<SchemeView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let (task, _) = scheme::block(&db, &from)?;

    db_evaluation_project::delete::connection(&db, &from, &to)?;

    scheme::read(&db, &task)
}

// ── Helpers of the commands above ─────────────────────────────────────

/// The project of standard output `uuid`; refused when it does not exist.
///
/// Called by: `standard_outputs_save`, `standard_outputs_delete`.
/// Calls: `db_evaluation_project::read::standard_output`.
fn standard_output_project(db: &UserDb, uuid: &str) -> Result<String, AppError> {
    match db_evaluation_project::read::standard_output(db, uuid)? {
        Some((project, _)) => Ok(project),
        None => Err(AppError::Invalid("the standard output does not exist".to_owned())),
    }
}

/// Project `uuid` in its form.
///
/// Called by: `project_form_get`, `project_form_save`.
/// Calls: `db_evaluation_project::read::project`, `catalogs::finished`.
fn project_view_of(db: &UserDb, uuid: &str) -> Result<ProjectView, AppError> {
    let Some(project) = db_evaluation_project::read::project(db, uuid)? else {
        return Err(AppError::Invalid("the project does not exist".to_owned()));
    };

    let available = catalogs::finished(db)?;
    let mut chosen = Vec::with_capacity(project.catalogs.len());

    for catalog in &project.catalogs {
        for choice in &available {
            if &choice.uuid == catalog {
                chosen.push(choice.clone());
            }
        }
    }

    Ok(ProjectView {
        uuid: project.uuid,
        title: project.fields.title,
        description: project.fields.description,
        catalogs: chosen,
        available,
    })
}

/// Catalog `catalog`, read, when project `project` uses it.
///
/// Called by: `task_list_add`, `standard_browser_get`.
/// Calls: `db_evaluation_project::read::project`, `db_oscal::read::catalog`.
fn project_catalog(db: &UserDb, project: &str, catalog: &str) -> Result<Catalog, AppError> {
    let Some(found) = db_evaluation_project::read::project(db, project)? else {
        return Err(AppError::Invalid("the project does not exist".to_owned()));
    };

    ensure!(
        found.catalogs.contains(&catalog.to_owned()),
        AppError::Invalid("the project does not use this catalog".to_owned())
    );

    Ok(db_oscal::read::catalog(db, catalog)?)
}

#[cfg(test)]
mod tests {
    //! Tests of the commands of the view Evaluation plan, called as the page
    //! calls them: by name, through the IPC of Tauri. The import and the
    //! export are tested in `oscal_documents/file.rs`: the dialogs of the
    //! system cannot run in the mock window.

    use serde_json::json;

    use crate::views::evaluation_plan::oscal_documents::file;
    use crate::views::test_support::{logged_in_window, logged_out_window};

    #[test]
    fn a_document_is_listed_and_deleted() {
        let window = logged_in_window();
        let catalog = include_str!("../../../oscal/examples/basic-catalog.json");
        let uuid = file::import(&window.session().db(), catalog).unwrap().uuid;

        let listed = window.call("oscal_documents_get", json!({})).unwrap();
        assert_eq!(listed[0]["uuid"], uuid);
        assert_eq!(listed[0]["model"], "catalog");

        window.call("oscal_documents_delete", json!({ "uuid": uuid })).unwrap();
        assert_eq!(window.call("oscal_documents_get", json!({})).unwrap(), json!([]));
        assert_eq!(window.call("oscal_documents_delete", json!({ "uuid": uuid })).unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn the_documents_need_a_login() {
        assert_eq!(logged_out_window().call("oscal_documents_get", json!({})).unwrap_err()["kind"], "not_logged_in");
    }

    /// A window with the example catalog of NIST stored (imported, so
    /// finished) and a master project using it; gives the window, the
    /// catalog and the project.
    fn with_project() -> (crate::views::test_support::TestWindow, String, String) {
        let window = logged_in_window();
        let catalog = {
            let session = window.session();
            let document = crate::test_support::db::basic_catalog();
            crate::db::oscal::create::document(&session.db(), &document).unwrap();
            document.uuid().to_owned()
        };
        let form = json!({ "title": " CC:2022 R1 ", "description": "All of it", "catalogs": [catalog] });
        let project = window.call("new_project_create", json!({ "project": form })).unwrap()["uuid"].clone();

        (window, catalog, project.as_str().unwrap().to_owned())
    }

    #[test]
    fn a_project_is_made_from_finished_catalogs_and_listed() {
        let (window, catalog, project) = with_project();

        let list = window.call("project_list_get", json!({})).unwrap();
        assert_eq!(list[0]["title"], "CC:2022 R1");
        assert_eq!((list[0]["catalogs"].clone(), list[0]["tasks"].clone()), (json!(1), json!(0)));

        let form = window.call("project_form_get", json!({ "uuid": project })).unwrap();
        assert_eq!(form["catalogs"][0]["uuid"], catalog);
        assert_eq!(window.call("new_project_catalogs", json!({})).unwrap()[0]["uuid"], catalog);

        // A catalog in conversion, or none at all, is refused.
        let unfinished = {
            let session = window.session();
            let source = crate::test_support::standards::load(&session.db(), b"%PDF plan");
            crate::views::converter::new_catalog::made::create(&session.db(), "Draft", "", &[source]).unwrap().uuid
        };
        for catalogs in [json!([unfinished]), json!([])] {
            let form = json!({ "title": "X", "description": "", "catalogs": catalogs });
            assert_eq!(window.call("new_project_create", json!({ "project": form })).unwrap_err()["kind"], "invalid");
        }
    }

    #[test]
    fn requirements_become_tasks_once_and_are_ordered_by_hand() {
        let (window, catalog, project) = with_project();
        let add = |control: &str| {
            window.call("task_list_add", json!({ "project": project, "catalog": catalog, "control": control }))
        };

        add("s1.1.1").unwrap();
        let tasks = add("s2.1.1").unwrap();

        assert_eq!(tasks[0]["label"], "1.1.1");
        assert_eq!(tasks[1]["title"], "Access control policy");
        assert_eq!(add("s1.1.1").unwrap_err()["kind"], "invalid");
        assert_eq!(add("s1.1").unwrap_err()["kind"], "invalid", "a group is no requirement");

        let moved = window.call("task_list_move", json!({ "task": tasks[1]["uuid"], "up": true })).unwrap();
        assert_eq!(moved[0]["control_id"], "s2.1.1");

        let left = window.call("task_list_remove", json!({ "task": moved[0]["uuid"] })).unwrap();
        assert_eq!(left.as_array().unwrap().len(), 1);
        assert_eq!(window.call("task_list_get", json!({ "project": project })).unwrap(), left);
    }

    #[test]
    fn the_browser_shows_a_requirement_and_finds_others() {
        let (window, catalog, project) = with_project();
        window.call("task_list_add", json!({ "project": project, "catalog": catalog, "control": "s1.1.1" })).unwrap();

        let requirement = window
            .call("standard_browser_get", json!({ "project": project, "catalog": catalog, "id": "s1.1.1" }))
            .unwrap();
        assert_eq!(requirement["title"], "Information security roles and responsibilities");
        assert_eq!(requirement["in_project"], true);
        assert!(!requirement["parts"].as_array().unwrap().is_empty());

        let hits =
            window.call("standard_browser_search", json!({ "project": project, "text": "ACCESS CONTROL" })).unwrap();
        assert!(hits.as_array().unwrap().iter().any(|hit| hit["id"] == "s2.1.1"));
        let empty = window.call("standard_browser_search", json!({ "project": project, "text": "  " })).unwrap_err();
        assert_eq!(empty["kind"], "invalid");
    }

    #[test]
    fn a_catalog_in_use_stays_and_a_catalog_with_tasks_stays_in_the_project() {
        let (window, catalog, project) = with_project();
        window.call("task_list_add", json!({ "project": project, "catalog": catalog, "control": "s1.1.1" })).unwrap();

        let error = window.call("oscal_documents_delete", json!({ "uuid": catalog })).unwrap_err();
        assert_eq!(error["kind"], "invalid");

        let other = {
            let session = window.session();
            let mut document = crate::test_support::db::basic_catalog();
            let text = serde_json::to_string(&document)
                .unwrap()
                .replace(document.uuid(), "11111111-2222-4333-8444-555555555555");
            document = crate::model::oscal::OscalDocument::from_json(&text).unwrap();
            crate::db::oscal::create::document(&session.db(), &document).unwrap();
            document.uuid().to_owned()
        };
        let form = json!({ "title": "CC", "description": "", "catalogs": [other] });
        let refused = window.call("project_form_save", json!({ "uuid": project, "project": form })).unwrap_err();
        assert_eq!(refused["kind"], "invalid");

        let form = json!({ "title": "CC 2", "description": "", "catalogs": [catalog, other] });
        let saved = window.call("project_form_save", json!({ "uuid": project, "project": form })).unwrap();
        assert_eq!(saved["catalogs"].as_array().unwrap().len(), 2);

        window.call("project_list_delete", json!({ "uuid": project })).unwrap();
        window.call("oscal_documents_delete", json!({ "uuid": catalog })).unwrap();
    }

    #[test]
    fn a_requirement_of_the_laboratory_becomes_a_task_and_serves_other_projects() {
        let (window, catalog, project) = with_project();
        let written =
            json!({ "identifier": " Net config 1 ", "title": "Network configuration", "statement": "Review it." });

        let tasks = window.call("own_requirement_add", json!({ "project": project, "requirement": written })).unwrap();

        // The identifier as the planner wrote it; the catalog of the laboratory joined the project.
        assert_eq!(tasks[0]["label"], "Net config 1");
        let lab = tasks[0]["catalog_uuid"].as_str().unwrap().to_owned();
        let control = tasks[0]["control_id"].as_str().unwrap().to_owned();
        let form = window.call("project_form_get", json!({ "uuid": project })).unwrap();
        assert_eq!(form["catalogs"].as_array().unwrap().len(), 2);

        // The same identifier twice is refused.
        let again =
            window.call("own_requirement_add", json!({ "project": project, "requirement": written })).unwrap_err();
        assert_eq!(again["kind"], "invalid");

        // Another project takes it from the shared catalog, as any requirement.
        let other_form = json!({ "title": "Other", "description": "", "catalogs": [catalog, lab] });
        let other = window.call("new_project_create", json!({ "project": other_form })).unwrap()["uuid"].clone();
        window.call("task_list_add", json!({ "project": other, "catalog": lab, "control": control })).unwrap();

        // Shown with its fields, and changed for all.
        let shown =
            window.call("standard_browser_get", json!({ "project": other, "catalog": lab, "id": control })).unwrap();
        assert_eq!(shown["own"]["statement"], "Review it.");
        let changed =
            json!({ "identifier": "Net config 1", "title": "Network configuration review", "statement": "All of it." });
        window.call("own_requirement_save", json!({ "id": control, "requirement": changed })).unwrap();
        assert_eq!(
            window.call("task_list_get", json!({ "project": project })).unwrap()[0]["title"],
            "Network configuration review"
        );
        let standard =
            window.call("standard_browser_get", json!({ "project": project, "catalog": catalog, "id": "s1.1.1" }));
        assert_eq!(standard.unwrap()["own"], json!(null));
    }

    #[test]
    fn a_requirement_of_the_laboratory_is_deleted_only_without_tasks() {
        let (window, _, project) = with_project();
        let written = json!({ "identifier": "R-7", "title": "A requirement", "statement": "Do it." });
        let tasks = window.call("own_requirement_add", json!({ "project": project, "requirement": written })).unwrap();
        let control = tasks[0]["control_id"].clone();

        let refused = window.call("own_requirement_delete", json!({ "id": control })).unwrap_err();
        assert_eq!(refused["kind"], "invalid");

        window.call("task_list_remove", json!({ "task": tasks[0]["uuid"] })).unwrap();
        window.call("own_requirement_delete", json!({ "id": control })).unwrap();

        let empty = json!({ "identifier": "  ", "title": "t", "statement": "s" });
        let error =
            window.call("own_requirement_add", json!({ "project": project, "requirement": empty })).unwrap_err();
        assert_eq!(error["kind"], "invalid");
    }

    /// A project with one task (`s1.1.1`); gives the window and the task.
    fn with_task() -> (crate::views::test_support::TestWindow, String) {
        let (window, catalog, project) = with_project();
        let tasks = window
            .call("task_list_add", json!({ "project": project, "catalog": catalog, "control": "s1.1.1" }))
            .unwrap();

        (window, tasks[0]["uuid"].as_str().unwrap().to_owned())
    }

    fn fields(name: &str, format: &str, condition: &str) -> serde_json::Value {
        json!({ "name": name, "evidence": "The Security Target, section 6.", "instructions": "Check it is complete.",
                "target": "pETR, section 4", "format": format, "condition": condition })
    }

    #[test]
    fn a_task_opens_with_only_its_requirement_from_the_standard() {
        let (window, task) = with_task();

        let scheme = window.call("task_scheme_get", json!({ "task": task })).unwrap();

        assert_eq!(scheme["blocks"].as_array().unwrap().len(), 1);
        assert_eq!(scheme["blocks"][0]["kind"], "requirement");
        assert_eq!(scheme["requirement"]["title"], "Information security roles and responsibilities");
        assert_eq!(scheme["connections"], json!([]));
        assert_eq!(scheme["results"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn inputs_and_outputs_are_added_named_connected_and_removed() {
        let (window, task) = with_task();
        let add = |kind: &str| {
            window.call("task_scheme_add_block", json!({ "task": task, "kind": kind, "x": -300, "y": 20 }))
        };

        let requirement = window.call("task_scheme_get", json!({ "task": task })).unwrap()["blocks"][0]["uuid"].clone();
        let input = add("input").unwrap()["blocks"][1]["uuid"].clone();
        let output = add("output").unwrap()["blocks"][2]["uuid"].clone();
        assert_eq!(add("requirement").unwrap_err()["kind"], "invalid");

        let saved = window
            .call(
                "task_scheme_save_block",
                json!({ "block": output, "fields": fields("Observation Report", "latex", "not_met") }),
            )
            .unwrap();
        assert_eq!(saved["blocks"][2]["condition"], "not_met");
        assert_eq!(saved["blocks"][2]["format"], "latex");
        let nameless =
            window.call("task_scheme_save_block", json!({ "block": input, "fields": fields(" ", "text", "always") }));
        assert_eq!(nameless.unwrap_err()["kind"], "invalid");

        window.call("task_scheme_connect", json!({ "from": input, "to": requirement })).unwrap();
        let connected = window.call("task_scheme_connect", json!({ "from": requirement, "to": output })).unwrap();
        assert_eq!(connected["connections"].as_array().unwrap().len(), 2);
        let wrong = window.call("task_scheme_connect", json!({ "from": input, "to": output }));
        assert_eq!(wrong.unwrap_err()["kind"], "invalid");

        window.call("task_scheme_move_block", json!({ "block": input, "x": -400, "y": 60 })).unwrap();
        let moved = window.call("task_scheme_disconnect", json!({ "from": input, "to": requirement })).unwrap();
        assert_eq!((moved["blocks"][1]["x"].clone(), moved["connections"].as_array().unwrap().len()), (json!(-400), 1));

        let left = window.call("task_scheme_remove_block", json!({ "block": output })).unwrap();
        assert_eq!((left["blocks"].as_array().unwrap().len(), left["connections"].clone()), (2, json!([])));
        assert_eq!(
            window.call("task_scheme_remove_block", json!({ "block": requirement })).unwrap_err()["kind"],
            "invalid"
        );
    }

    fn output(name: &str, condition: &str) -> serde_json::Value {
        json!({ "name": name, "instructions": "Fill the template.", "target": "section 4", "format": "latex",
                "condition": condition })
    }

    #[test]
    fn the_standard_outputs_are_kept_with_all_four_operations() {
        let (window, _, project) = with_project();

        window
            .call("standard_outputs_add", json!({ "project": project, "output": output(" pETR ", "always") }))
            .unwrap();
        let outputs = window
            .call("standard_outputs_add", json!({ "project": project, "output": output("OR", "not_met") }))
            .unwrap();
        assert_eq!((outputs[0]["name"].clone(), outputs[1]["condition"].clone()), (json!("pETR"), json!("not_met")));

        let saved = window
            .call(
                "standard_outputs_save",
                json!({ "output": outputs[1]["uuid"], "fields": output("Observation Report", "not_met") }),
            )
            .unwrap();
        assert_eq!(saved[1]["name"], "Observation Report");
        assert_eq!(window.call("standard_outputs_get", json!({ "project": project })).unwrap(), saved);

        let left = window.call("standard_outputs_delete", json!({ "output": saved[0]["uuid"] })).unwrap();
        assert_eq!(left.as_array().unwrap().len(), 1);

        let unnamed =
            window.call("standard_outputs_add", json!({ "project": project, "output": output(" ", "always") }));
        assert_eq!(unnamed.unwrap_err()["kind"], "invalid");
        let wrong = window.call("standard_outputs_add", json!({ "project": project, "output": output("X", "maybe") }));
        assert_eq!(wrong.unwrap_err()["kind"], "invalid");
        assert_eq!(window.call("standard_outputs_delete", json!({ "output": "none" })).unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn a_new_task_gets_a_copy_of_the_standard_outputs_which_it_may_change() {
        let (window, catalog, project) = with_project();
        window.call("standard_outputs_add", json!({ "project": project, "output": output("pETR", "always") })).unwrap();
        window.call("standard_outputs_add", json!({ "project": project, "output": output("OR", "not_met") })).unwrap();

        let tasks = window
            .call("task_list_add", json!({ "project": project, "catalog": catalog, "control": "s1.1.1" }))
            .unwrap();
        let scheme = window.call("task_scheme_get", json!({ "task": tasks[0]["uuid"] })).unwrap();

        assert_eq!(scheme["blocks"].as_array().unwrap().len(), 3);
        assert_eq!(
            (scheme["blocks"][1]["name"].clone(), scheme["blocks"][2]["condition"].clone()),
            (json!("pETR"), json!("not_met"))
        );
        assert_eq!(scheme["connections"][0]["from"], scheme["blocks"][0]["uuid"]);

        // The copy is the task's own: removed in the task, it stays in the template.
        window.call("task_scheme_remove_block", json!({ "block": scheme["blocks"][2]["uuid"] })).unwrap();
        assert_eq!(
            window.call("standard_outputs_get", json!({ "project": project })).unwrap().as_array().unwrap().len(),
            2
        );
    }
}
