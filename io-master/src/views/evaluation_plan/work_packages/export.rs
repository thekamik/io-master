//! The export of a work package (step 51.5 of docs/history.md, format b of the
//! user, 2026-10-24): one JSON file with our format and, as attachments, the
//! documents of OSCAL it points to, whole and unchanged. A picture of the
//! state at the moment of the export; the import is the work of the
//! Evaluation lab (step 52).
//!
//! ```text
//! { "format": "io-master-work-package", "version": 1, "exported": <seconds since 1970>,
//!   "project": { uuid, title, scope },  "package": { uuid, name, evaluator, note },
//!   "results": [ the four titles ],
//!   "tasks": [ { uuid, catalog_uuid, control_id, label, title,
//!                blocks (accepted only), connections (between them), related (accepted only) } ],
//!   "attachments": [ the documents of OSCAL of every catalog named above ] }
//! ```

use serde_json::{Value, json};

use crate::app::error::AppError;
use crate::db_old::evaluation_project as db_evaluation_project;
use crate::db_old::oscal as db_oscal;
use crate::db_old::user::database::UserDb;
use crate::model_old::evaluation_project::EvaluationTask;
use crate::model_old::time::now;

use super::super::task_list::view as task_view;
use super::super::task_scheme::view::{BlockView, result_titles};

/// The name of the format, written into every file.
pub const FORMAT: &str = "io-master-work-package";

/// The version of the format.
pub const FORMAT_VERSION: u32 = 1;

/// The name of package `package` and the text of its file.
///
/// Called by: `work_packages_export` (`evaluation_plan.rs`).
/// Calls: `read::work_package`, `read::project`, [`task_json`], [`attachments`].
pub fn file_text(db: &UserDb, package: &str) -> Result<(String, String), AppError> {
    let Some((project, found)) = db_evaluation_project::read::work_package(db, package)? else {
        return Err(AppError::Invalid("the work package does not exist".to_owned()));
    };
    let Some(project) = db_evaluation_project::read::project(db, &project)? else {
        return Err(AppError::Invalid("the project does not exist".to_owned()));
    };

    let mut catalogs: Vec<String> = Vec::new();
    let mut tasks = Vec::with_capacity(found.tasks.len());

    for task in &found.tasks {
        tasks.push(task_json(db, task, &mut catalogs)?);
    }

    let file = json!({
        "format": FORMAT,
        "version": FORMAT_VERSION,
        "exported": now(),
        "project": { "uuid": project.uuid, "title": project.fields.title, "scope": project.fields.description },
        "package": {
            "uuid": found.uuid,
            "name": found.fields.name,
            "evaluator": found.fields.evaluator,
            "note": found.fields.note,
        },
        "results": result_titles(),
        "tasks": tasks,
        "attachments": attachments(db, &catalogs)?,
    });

    let text = serde_json::to_string_pretty(&file).map_err(|error| AppError::Failed(error.to_string()))?;

    Ok((found.fields.name, text))
}

/// Task `uuid` as the file writes it: its requirement, its accepted blocks
/// and the connections between them, its accepted related elements. The
/// catalogs it names are added to `catalogs`.
///
/// Called by: [`file_text`].
/// Calls: `read::task`, `task_view::all`, `read::blocks`, `read::connections`,
/// `read::task_links`, [`related_json`], [`remember`].
fn task_json(db: &UserDb, uuid: &str, catalogs: &mut Vec<String>) -> Result<Value, AppError> {
    let Some((_, task)) = db_evaluation_project::read::task(db, uuid)? else {
        return Err(AppError::Invalid("a task of the package does not exist".to_owned()));
    };

    remember(catalogs, &task.catalog_uuid);

    let Some(requirement) = task_view::all(db, vec![task.clone()])?.pop() else {
        return Err(AppError::Failed("the requirement of a task was lost".to_owned()));
    };

    let mut blocks = Vec::new();
    let mut kept = Vec::new();

    for block in db_evaluation_project::read::blocks(db, uuid)? {
        if block.proposal.is_none() {
            kept.push(block.uuid.clone());
            blocks.push(BlockView::new(block, None));
        }
    }

    let mut connections = Vec::new();

    for connection in db_evaluation_project::read::connections(db, uuid)? {
        if kept.contains(&connection.from) && kept.contains(&connection.to) {
            connections.push(json!({ "from": connection.from, "to": connection.to }));
        }
    }

    let mut related = Vec::new();

    for link in db_evaluation_project::read::task_links(db, uuid)? {
        if !link.proposed {
            remember(catalogs, &link.catalog_uuid);
            related.push(EvaluationTask {
                uuid: String::new(),
                catalog_uuid: link.catalog_uuid,
                control_id: link.control_id,
            });
        }
    }

    Ok(json!({
        "uuid": task.uuid,
        "catalog_uuid": task.catalog_uuid,
        "control_id": task.control_id,
        "label": requirement.label,
        "title": requirement.title,
        "blocks": blocks,
        "connections": connections,
        "related": related_json(db, related)?,
    }))
}

/// The related elements as the file writes them.
///
/// Called by: [`task_json`].
/// Calls: `task_view::all`.
fn related_json(db: &UserDb, related: Vec<EvaluationTask>) -> Result<Vec<Value>, AppError> {
    let mut found = Vec::with_capacity(related.len());

    for element in task_view::all(db, related)? {
        found.push(json!({
            "catalog_uuid": element.catalog_uuid,
            "control_id": element.control_id,
            "label": element.label,
            "title": element.title,
        }));
    }

    Ok(found)
}

/// Adds catalog `uuid` to `catalogs`, once.
///
/// Called by: [`task_json`].
fn remember(catalogs: &mut Vec<String>, uuid: &str) {
    if !catalogs.iter().any(|known| known == uuid) {
        catalogs.push(uuid.to_owned());
    }
}

/// The documents of OSCAL of `catalogs`, whole, as OSCAL defines them.
///
/// Called by: [`file_text`].
/// Calls: `db_oscal::read::document`, `OscalDocument::to_json`.
fn attachments(db: &UserDb, catalogs: &[String]) -> Result<Vec<Value>, AppError> {
    let mut found = Vec::with_capacity(catalogs.len());

    for uuid in catalogs {
        let Some(document) = db_oscal::read::document(db, uuid)? else {
            return Err(AppError::Invalid("a catalog of the package does not exist".to_owned()));
        };

        found.push(document.to_json().map_err(|error| AppError::Failed(error.to_string()))?);
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    //! Tests of `export.rs`.

    use super::*;

    use crate::model_old::evaluation_project::{ProjectFields, TaskLink};
    use crate::model_old::task_scheme::{Block, BlockFields, BlockKind, new_scheme};
    use crate::model_old::work_package::{PackageFields, WorkPackage};
    use crate::test_support::standards::database;

    #[test]
    fn a_package_is_written_with_what_was_accepted_and_its_catalogs_attached() {
        let test = database();
        let catalog = crate::test_support::db::basic_catalog();
        crate::db_old::oscal::create::document(&test.db, &catalog).unwrap();
        let uuid = catalog.uuid().to_owned();
        let fields = ProjectFields { title: "CC".to_owned(), description: "A smart card.".to_owned() };
        db_evaluation_project::create::project(&test.db, "p", &fields, std::slice::from_ref(&uuid)).unwrap();
        let task = EvaluationTask { uuid: "t".to_owned(), catalog_uuid: uuid.clone(), control_id: "s1.1.1".to_owned() };
        db_evaluation_project::create::task(&test.db, "p", &task, &new_scheme("r", &[]).unwrap()).unwrap();
        for (block, proposal) in [("accepted", None), ("proposed", Some("Why.".to_owned()))] {
            let mut written = BlockFields::empty();
            written.name = block.to_owned();
            let input =
                Block { uuid: block.to_owned(), kind: BlockKind::Input, x: -320, y: 0, fields: written, proposal };
            db_evaluation_project::create::block(&test.db, "t", &input).unwrap();
            db_evaluation_project::create::connection(&test.db, "t", block, "r").unwrap();
        }
        for (control, proposed) in [("s1.1.2", false), ("s2.1.1", true)] {
            let link = TaskLink {
                catalog_uuid: uuid.clone(),
                control_id: control.to_owned(),
                reason: String::new(),
                proposed,
            };
            db_evaluation_project::create::task_link(&test.db, "t", &link).unwrap();
        }
        let package = WorkPackage {
            uuid: "w".to_owned(),
            fields: PackageFields { name: "Roles".to_owned(), evaluator: "Anna".to_owned(), note: String::new() },
            tasks: vec!["t".to_owned()],
        };
        db_evaluation_project::create::work_package(&test.db, "p", &package).unwrap();

        let (name, text) = file_text(&test.db, "w").unwrap();
        let file: Value = serde_json::from_str(&text).unwrap();

        assert_eq!(name, "Roles");
        assert_eq!((file["format"].clone(), file["version"].clone()), (json!(FORMAT), json!(FORMAT_VERSION)));
        assert_eq!(file["package"]["evaluator"], "Anna");
        assert_eq!(file["results"].as_array().unwrap().len(), 4);
        let exported = &file["tasks"][0];
        assert_eq!(exported["label"], "1.1.1");
        let names: Vec<&str> =
            exported["blocks"].as_array().unwrap().iter().map(|block| block["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["", "accepted"], "the proposed block stays home");
        assert_eq!(exported["connections"].as_array().unwrap().len(), 1);
        assert_eq!(exported["related"][0]["title"], "Segregation of duties");
        assert_eq!(exported["related"].as_array().unwrap().len(), 1);
        assert_eq!(file["attachments"].as_array().unwrap().len(), 1);
        assert_eq!(file["attachments"][0]["catalog"]["uuid"], json!(uuid));
    }
}
