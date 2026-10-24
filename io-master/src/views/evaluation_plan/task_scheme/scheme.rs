//! The scheme of a task read for the page: the task with its requirement
//! (from its catalog), its blocks and connections (from
//! `db/evaluation_project/`). A task made before the scheme existed gets its
//! block of the requirement here.

use crate::app::error::AppError;
use crate::db::evaluation_project as db_evaluation_project;
use crate::db::oscal as db_oscal;
use crate::db::oscal::ids::new_uuid;
use crate::db::user::database::UserDb;
use crate::model::task_scheme::{Block, BlockFields, BlockKind};

use super::super::own_requirement::catalog as own_catalog;
use super::super::standard_browser::requirement;
use super::super::task_list::view as task_view;
use super::view::{BlockView, ConnectionView, SchemeView};

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

    let mut blocks = db_evaluation_project::read::blocks(db, task)?;
    let mut has_requirement = false;

    for block in &blocks {
        if block.kind == BlockKind::Requirement {
            has_requirement = true;
        }
    }

    if !has_requirement {
        let block = Block { uuid: new_uuid()?, kind: BlockKind::Requirement, x: 0, y: 0, fields: BlockFields::empty() };

        db_evaluation_project::create::block(db, task, &block)?;
        blocks = db_evaluation_project::read::blocks(db, task)?;
    }

    let catalog = db_oscal::read::catalog(db, &found.catalog_uuid)?;
    let own = own_catalog::existing(db)?.as_deref() == Some(found.catalog_uuid.as_str());
    let shown = requirement::read(db, &found.catalog_uuid, &catalog, &found.control_id, true, own)?;
    let mut tasks = task_view::all(db, vec![found])?;

    let Some(task_shown) = tasks.pop() else {
        return Err(AppError::Failed("the task was lost".to_owned()));
    };

    let mut block_views = Vec::with_capacity(blocks.len());

    for block in blocks {
        block_views.push(BlockView::new(block));
    }

    let mut connections = Vec::new();

    for connection in db_evaluation_project::read::connections(db, task)? {
        connections.push(ConnectionView::new(connection));
    }

    Ok(SchemeView { task: task_shown, requirement: shown, blocks: block_views, connections })
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
