//! The templates of the panel Reports, as the page calls them: the list, a
//! new one, one with its warnings, saved from the editor, imported from a
//! file and written into one; the markers the editor offers.
//!
//! ```text
//! ReportTemplates.svelte ──report_template_list()──────► the templates ──► [TemplateSummaryView]
//!                        ──report_template_create(name)► new_template ──► id
//!                        ──report_template_import()────► the dialog ──► import_template ──► id or null
//! TemplateEditor.svelte  ──report_template_get(id)─────► the template, its warnings ──► TemplateView
//!                        ──report_template_save(...)───► change_template ──► the warnings
//!                        ──report_template_export(id)──► the dialog ──► write_template ──► whether written
//!                        ──report_markers()────────────► the markers, by block ──► [MarkerGroupView]
//! ```

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::objects::evaluation::report_template::report_template_model::ReportTemplate;
use crate::objects::evaluation::report_template::report_template_rules::markers::{
    LISTS, block_tags, element_markers, known_markers, template_warnings,
};
use crate::objects::evaluation::report_template::report_template_rules::templates::{
    MAX_TEMPLATE, change_template, import_template, new_template, write_template,
};
use crate::objects::evaluation::report_template::report_template_store::{read_report_template, read_report_templates};
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

/// The longest name or extension from the page, in bytes (the rules check
/// the characters).
const MAX_FIELD: usize = 1024;

/// A template, as the list shows it.
#[derive(Serialize)]
pub struct TemplateSummaryView {
    id: i64,
    name: String,
    extension: String,
}

/// A template whole, for the editor.
#[derive(Serialize)]
pub struct TemplateView {
    id: i64,
    name: String,
    extension: String,
    text: String,
    /// The unknown markers of its text (`["an unknown marker {{x}}: ..."]`).
    warnings: Vec<String>,
}

/// The markers the editor offers: those of the evaluation, or the block of
/// a list with the markers of its elements.
#[derive(Serialize)]
pub struct MarkerGroupView {
    /// The tags that open and close the block; empty outside every block.
    opening: String,
    closing: String,
    /// The markers of the evaluation outside every block, of the elements of
    /// its list in a block (`["{{evaluation_plan.product}}", ...]`).
    markers: Vec<String>,
}

/// Every template, by their names.
///
/// Input: `session`, the session of the window, someone logged in.
/// Output: the summaries (`[{"id": 3, "name": "ETR", "extension": "md"}]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/reports.ts`: `reportTemplateList`, from
/// `ui/src/views/reports/ReportTemplates.svelte` and `ui/src/views/reports/Reports.svelte`, when they open.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/report_template/report_template_store.rs`: `read_report_templates`.
#[tauri::command]
pub fn report_template_list(session: State<'_, Session>) -> Result<Vec<TemplateSummaryView>, ViewError> {
    let opened = session.lock()?;
    let templates = read_report_templates(database(opened.as_ref())?)?;
    let mut summaries = Vec::with_capacity(templates.len());

    for template in templates {
        summaries.push(TemplateSummaryView { id: template.id, name: template.name, extension: template.extension });
    }

    Ok(summaries)
}

/// A new template named `name`, with the default text; gives its id.
///
/// Input: `session`, the session of the window, someone logged in; `name`,
/// as typed.
/// Output: the id of the template (`3`); `ViewError::NotLoggedIn` when
/// nobody is logged in, the refusals of the template.
///
/// Called by: `ui/src/lib/api/reports.ts`: `reportTemplateCreate`, from
/// `ui/src/views/reports/ReportTemplates.svelte`, "New template".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/report_template/report_template_rules/templates.rs`: `new_template`.
#[tauri::command]
pub fn report_template_create(session: State<'_, Session>, name: String) -> Result<i64, ViewError> {
    ensure!(name.len() <= MAX_FIELD, ViewError::Invalid("the name is too long".to_owned()));

    let opened = session.lock()?;

    Ok(new_template(database(opened.as_ref())?, &name)?)
}

/// The template `id` with the warnings of its markers.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the
/// id of a template.
/// Output: the template (`{"id": 3, "name": "ETR", "text": "# ...", "warnings": []}`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/reports.ts`: `reportTemplateGet`, from
/// `ui/src/views/reports/TemplateEditor.svelte`, when it opens and after a save.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/report_template/report_template_store.rs`: `read_report_template`;
/// `objects/evaluation/report_template/report_template_rules/markers.rs`: `template_warnings`.
#[tauri::command]
pub fn report_template_get(session: State<'_, Session>, id: i64) -> Result<TemplateView, ViewError> {
    let opened = session.lock()?;
    let template = read_report_template(database(opened.as_ref())?, id)?;
    let warnings = template_warnings(&template.text)?;

    Ok(TemplateView { id, name: template.name, extension: template.extension, text: template.text, warnings })
}

/// Saves the template `id` as the editor gives it; gives the warnings of
/// its markers.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the
/// id of a template; `name`, `extension`, `text`, as typed.
/// Output: the warnings (`["an unknown marker {{x}}: ..."]`); `ViewError::Invalid`
/// for a text too long from the page, `ViewError::NotLoggedIn` when nobody
/// is logged in, the refusals of the template (its blocks wrong).
///
/// Called by: `ui/src/lib/api/reports.ts`: `reportTemplateSave`, from
/// `ui/src/views/reports/TemplateEditor.svelte`, "Save".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/report_template/report_template_rules/templates.rs`: `change_template`.
#[tauri::command]
pub fn report_template_save(
    session: State<'_, Session>,
    id: i64,
    name: String,
    extension: String,
    text: String,
) -> Result<Vec<String>, ViewError> {
    ensure!(
        name.len() + extension.len() <= MAX_FIELD,
        ViewError::Invalid("the name or the extension is too long".to_owned())
    );
    // A character takes at most four bytes: the rules count the characters.
    ensure!(text.len() <= 4 * MAX_TEMPLATE, ViewError::Invalid("the template is too long".to_owned()));

    let opened = session.lock()?;
    let template = ReportTemplate { id, name, extension, text };

    Ok(change_template(database(opened.as_ref())?, &template)?)
}

/// A new template of a file of text the user chooses in the dialog of the
/// system; gives its id, or `None` when the dialog was closed. Run apart
/// from the window (`async`): the dialog waits for the user.
///
/// Input: `app`, the handle of the window; `session`, the session of the
/// window, someone logged in.
/// Output: the id of the template (`Some(4)`); `ViewError::NotLoggedIn` when
/// nobody is logged in, the refusals of the file or the template.
///
/// Called by: `ui/src/lib/api/reports.ts`: `reportTemplateImport`, from
/// `ui/src/views/reports/ReportTemplates.svelte`, "Import from a file".
/// Calls: the dialog of the system; `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/report_template/report_template_rules/templates.rs`: `import_template`.
#[tauri::command(async)]
pub fn report_template_import(app: AppHandle, session: State<'_, Session>) -> Result<Option<i64>, ViewError> {
    let Some(file) = app.dialog().file().set_title("Import a template").blocking_pick_file() else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|error| ViewError::Invalid(format!("the file chosen: {error}")))?;
    let opened = session.lock()?;

    Ok(Some(import_template(database(opened.as_ref())?, &path)?))
}

/// Writes the text of the template `id` into a file the user chooses in the
/// dialog of the system; gives whether it was written. Run apart from the
/// window (`async`): the dialog waits for the user.
///
/// Input: `app`, the handle of the window; `session`, the session of the
/// window, someone logged in; `id`, the id of a template.
/// Output: whether a file was chosen and written (`true`); `ViewError::NotLoggedIn`
/// when nobody is logged in, the refusals of the file, an error of the database.
///
/// Called by: `ui/src/lib/api/reports.ts`: `reportTemplateExport`, from
/// `ui/src/views/reports/TemplateEditor.svelte`, "Export to a file".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/report_template/report_template_store.rs`: `read_report_template`; the dialog of the
/// system; `objects/evaluation/report_template/report_template_rules/templates.rs`: `write_template`.
#[tauri::command(async)]
pub fn report_template_export(app: AppHandle, session: State<'_, Session>, id: i64) -> Result<bool, ViewError> {
    let name = {
        let opened = session.lock()?;
        let template = read_report_template(database(opened.as_ref())?, id)?;

        // A slash in a name would be a folder in the name of a file.
        format!("{}.{}", template.name.replace('/', "-"), template.extension)
    };
    let Some(file) = app.dialog().file().set_title("Export the template").set_file_name(name).blocking_save_file()
    else {
        return Ok(false);
    };
    let path = file.into_path().map_err(|error| ViewError::Invalid(format!("the file chosen: {error}")))?;
    let opened = session.lock()?;

    write_template(database(opened.as_ref())?, id, &path)?;

    Ok(true)
}

/// The markers the editor offers: those of the evaluation, known in the
/// whole template, then for each list its block with the markers of its
/// elements, known only inside it.
///
/// Output: the groups (`[{"opening": "", "markers": ["{{evaluation_plan.id}}", ...]},
/// {"opening": "{{#evaluation_plan.work_units}}", ...}]`).
///
/// Called by: `ui/src/lib/api/reports.ts`: `reportMarkers`, from `ui/src/views/reports/TemplateEditor.svelte`,
/// when it opens.
/// Calls: `objects/evaluation/report_template/report_template_rules/markers.rs`: `known_markers`,
/// `block_tags`, `element_markers`.
#[tauri::command]
pub fn report_markers() -> Vec<MarkerGroupView> {
    let mut groups = Vec::with_capacity(LISTS.len() + 1);

    groups.push(MarkerGroupView { opening: String::new(), closing: String::new(), markers: known_markers(None) });

    for list in LISTS {
        let (opening, closing) = block_tags(list);

        groups.push(MarkerGroupView { opening, closing, markers: element_markers(list) });
    }

    groups
}
