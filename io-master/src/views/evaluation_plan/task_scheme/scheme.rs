//! The scheme of a task read for the page: the task with its requirement
//! (from its catalog), its blocks and connections (from
//! `db/evaluation_project/`); and the first scheme of a new task.

use crate::app::error::AppError;
use crate::db::evaluation_project as db_evaluation_project;
use crate::db::oscal as db_oscal;
use crate::db::oscal::ids::new_uuid;
use crate::db::user::database::UserDb;
use crate::model::task_scheme::{self, Block, NewScheme};

use super::super::own_requirement::catalog as own_catalog;
use super::super::standard_browser::requirement;
use super::super::task_list::view as task_view;
use super::view::{BlockView, ConnectionView, SchemeView, result_titles};

/// The scheme of task `task`.
///
/// Called by: `task_scheme_get` and every command that changes a scheme
/// (`evaluation_plan.rs`).
/// Calls: `db_evaluation_project::read::task`, `blocks` and `connections`,
/// `task_view::all`, `requirement::read`, `own_catalog::existing`.
pub fn read(db: &UserDb, task: &str) -> Result<SchemeView, AppError> {
    let Some((_, found)) = db_evaluation_project::read::task(db, task)? else {
        return Err(AppError::Invalid("the task does not exist".to_owned()));
    };

    let blocks = db_evaluation_project::read::blocks(db, task)?;
    let catalog = db_oscal::read::catalog(db, &found.catalog_uuid)?;
    let own = own_catalog::existing(db)?.as_deref() == Some(found.catalog_uuid.as_str());
    let shown = requirement::read(db, &found.catalog_uuid, &catalog, &found.control_id, true, own)?;
    let mut tasks = task_view::all(db, vec![found])?;

    let Some(task_shown) = tasks.pop() else {
        return Err(AppError::Failed("the task was lost".to_owned()));
    };

    let mut block_views = Vec::with_capacity(blocks.len());

    let mut corrections = db_evaluation_project::read::corrections(db, task)?;

    for block in blocks {
        // The correction of this block, taken out of the list. Short
        // closures: the key of the search, and Option::map, which clippy asks for.
        let correction =
            corrections.iter().position(|found| found.block == block.uuid).map(|index| corrections.swap_remove(index));

        block_views.push(BlockView::new(block, correction));
    }

    let mut connections = Vec::new();

    for connection in db_evaluation_project::read::connections(db, task)? {
        connections.push(ConnectionView::new(connection));
    }

    Ok(SchemeView { task: task_shown, requirement: shown, blocks: block_views, connections, results: result_titles() })
}

/// The first scheme of a new task of project `project`: the block of its
/// requirement and a copy of every output of the template of the project
/// (new UUIDs), connected from the requirement.
///
/// Called by: `task_list_add`, `own_requirement_add` (`evaluation_plan.rs`).
/// Calls: `db_evaluation_project::read::standard_outputs`, `new_uuid`,
/// `task_scheme::new_scheme`.
pub fn first(db: &UserDb, project: &str) -> Result<NewScheme, AppError> {
    let template = db_evaluation_project::read::standard_outputs(db, project)?;
    let mut outputs = Vec::with_capacity(template.len());

    for output in template {
        outputs.push((new_uuid()?, output.fields));
    }

    task_scheme::new_scheme(&new_uuid()?, &outputs).map_err(|error| AppError::Invalid(error.to_string()))
}

/// Block `uuid` and its task; refused when it does not exist.
///
/// Called by: the commands that change a block (`evaluation_plan.rs`).
/// Calls: `db_evaluation_project::read::block`.
pub fn block(db: &UserDb, uuid: &str) -> Result<(String, Block), AppError> {
    match db_evaluation_project::read::block(db, uuid)? {
        Some(found) => Ok(found),
        None => Err(AppError::Invalid("the block does not exist".to_owned())),
    }
}
