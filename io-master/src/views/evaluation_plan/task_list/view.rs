//! The data of the list of tasks of a master project, as the page gets it:
//! every task with the text of its requirement read from its catalog.

use serde::Serialize;

use crate::app::error::AppError;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::model::evaluation_project::EvaluationTask;
use crate::model::oscal::catalog::Catalog;

/// A task: a whole requirement.
#[derive(Debug, Serialize)]
pub struct TaskView {
    pub uuid: String,
    pub catalog_uuid: String,
    pub catalog_title: String,
    pub control_id: String,
    /// The identifier the standard writes (`ADV_FSP.4`), or the id.
    pub label: String,
    /// The title of the requirement; empty when the control was removed
    /// from its catalog.
    pub title: String,
}

/// The views of `tasks`, their catalogs read once each.
///
/// Called by: the commands of the list of tasks (`evaluation_plan.rs`).
/// Calls: `db_oscal::read::catalog`, `Catalog::locate`, `Control::label`.
pub fn all(db: &UserDb, tasks: Vec<EvaluationTask>) -> Result<Vec<TaskView>, AppError> {
    let mut catalogs: Vec<(String, Catalog)> = Vec::new();
    let mut views = Vec::with_capacity(tasks.len());

    for task in tasks {
        let mut index = None;

        for (position, (uuid, _)) in catalogs.iter().enumerate() {
            if *uuid == task.catalog_uuid {
                index = Some(position);
            }
        }

        let index = match index {
            Some(index) => index,
            None => {
                catalogs.push((task.catalog_uuid.clone(), db_oscal::read::catalog(db, &task.catalog_uuid)?));
                catalogs.len() - 1
            }
        };

        let Some((_, catalog)) = catalogs.get(index) else {
            return Err(AppError::Failed("a catalog of a task was lost".to_owned()));
        };

        views.push(view(catalog, task)?);
    }

    Ok(views)
}

/// The view of one task, its requirement read from `catalog`.
///
/// Called by: [`all`].
fn view(catalog: &Catalog, task: EvaluationTask) -> Result<TaskView, AppError> {
    let location = catalog.locate(&task.control_id)?;
    let control = match &location {
        Some(location) => catalog.control_at(location),
        None => None,
    };

    let (label, title) = match control {
        Some(control) => (control.label().unwrap_or(&control.id).to_owned(), control.title.clone()),
        None => (task.control_id.clone(), String::new()),
    };

    Ok(TaskView {
        uuid: task.uuid,
        catalog_uuid: task.catalog_uuid,
        catalog_title: catalog.metadata.title.clone(),
        control_id: task.control_id,
        label,
        title,
    })
}
