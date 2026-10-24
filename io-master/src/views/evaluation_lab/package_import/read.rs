//! A file of a work package read for the Evaluation lab (step 52 of
//! docs/todo.md): its format and version checked, our part turned into a
//! `Package` of `model/evaluation.rs` (checked by its rules), and its
//! attachments read as documents of OSCAL (checked against the schema of
//! NIST). The format is written by the Evaluation plan
//! (`views/evaluation_plan/work_packages/export.rs`; the two views share no
//! code, so the names of the format are written here again).

use serde_json::Value;

use crate::app::error::AppError;
use crate::model::evaluation::{self, LabTask, Package, PackageTask, Related, TaskState};
use crate::model::evaluation_project::{MAX_LINKS, MAX_TASKS};
use crate::model::oscal::{OscalDocument, OscalModel};
use crate::model::task_scheme::{Block, BlockFields, BlockKind, Connection, MAX_BLOCKS, OutputCondition, OutputFormat};

/// The name of the format a package file must carry.
const FORMAT: &str = "io-master-work-package";

/// The version of the format this program reads.
const FORMAT_VERSION: u64 = 1;

/// The most catalogs attached to a package.
const MAX_ATTACHMENTS: usize = 50;

/// The largest file of a package, in bytes (a few catalogs of some MiB).
pub const MAX_FILE: u64 = 256 * 1024 * 1024;

/// The package in `text` and its attached catalogs.
///
/// Called by: `package_import_import` (`evaluation_lab.rs`).
/// Calls: [`package`], [`attachments`], `evaluation::check_package`.
pub fn read(text: &str) -> Result<(Package, Vec<OscalDocument>), AppError> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| AppError::Invalid(format!("the file is no JSON: {error}")))?;

    ensure!(
        value.get("format").and_then(Value::as_str) == Some(FORMAT),
        AppError::Invalid("the file is no work package of IO Master".to_owned())
    );
    ensure!(
        value.get("version").and_then(Value::as_u64) == Some(FORMAT_VERSION),
        AppError::Invalid("the work package is of a version this program does not read".to_owned())
    );

    let found = package(&value)?;

    evaluation::check_package(&found).map_err(|error| AppError::Invalid(error.to_string()))?;

    Ok((found, attachments(&value)?))
}

/// The text of `name` in `value`; empty when missing.
fn text(value: &Value, name: &str) -> String {
    value.get(name).and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// The array `name` of `value`, at most `max` items; refused when longer.
fn array<'a>(value: &'a Value, name: &str, max: usize) -> Result<&'a [Value], AppError> {
    let items = value.get(name).and_then(Value::as_array).map_or(&[][..], Vec::as_slice);

    ensure!(items.len() <= max, AppError::Invalid(format!("the work package has too many {name}")));

    Ok(items)
}

/// Our part of the file: the project, the package, its tasks.
///
/// Called by: [`read`].
/// Calls: [`task`].
fn package(value: &Value) -> Result<Package, AppError> {
    let project = value.get("project").unwrap_or(&Value::Null);
    let header = value.get("package").unwrap_or(&Value::Null);
    let items = array(value, "tasks", MAX_TASKS)?;
    let mut tasks = Vec::with_capacity(items.len());

    for item in items {
        tasks.push(task(item)?);
    }

    let package_uuid = text(header, "uuid");
    ensure!(!package_uuid.is_empty(), AppError::Invalid("the work package has no UUID".to_owned()));

    Ok(Package {
        package_uuid,
        project_title: text(project, "title"),
        scope: text(project, "scope"),
        name: text(header, "name"),
        evaluator: text(header, "evaluator"),
        note: text(header, "note"),
        tasks,
    })
}

/// One task of the file: its requirement, its blocks, its connections, its
/// related elements.
///
/// Called by: [`package`].
/// Calls: [`block`].
fn task(item: &Value) -> Result<PackageTask, AppError> {
    let mut blocks = Vec::new();
    let mut connections = Vec::new();
    let mut related = Vec::new();

    for block_item in array(item, "blocks", MAX_BLOCKS)? {
        blocks.push(block(block_item)?);
    }

    for connection in array(item, "connections", MAX_BLOCKS * 2)? {
        connections.push(Connection { from: text(connection, "from"), to: text(connection, "to") });
    }

    for element in array(item, "related", MAX_LINKS)? {
        related.push(Related {
            catalog_uuid: text(element, "catalog_uuid"),
            control_id: text(element, "control_id"),
            label: text(element, "label"),
            title: text(element, "title"),
        });
    }

    let task = LabTask {
        uuid: text(item, "uuid"),
        catalog_uuid: text(item, "catalog_uuid"),
        control_id: text(item, "control_id"),
        label: text(item, "label"),
        title: text(item, "title"),
        state: TaskState::Todo,
    };

    ensure!(
        !task.uuid.is_empty() && !task.catalog_uuid.is_empty() && !task.control_id.is_empty(),
        AppError::Invalid("a task of the work package names no requirement".to_owned())
    );

    Ok(PackageTask { task, blocks, connections, related })
}

/// One block of the file.
///
/// Called by: [`task`].
fn block(item: &Value) -> Result<Block, AppError> {
    let Some(kind) = BlockKind::from_name(&text(item, "kind")) else {
        return Err(AppError::Invalid("a block of the work package is of no known kind".to_owned()));
    };
    let Some(format) = OutputFormat::from_name(&text(item, "format")) else {
        return Err(AppError::Invalid("an output of the work package has no known format".to_owned()));
    };
    let Some(condition) = OutputCondition::from_name(&text(item, "condition")) else {
        return Err(AppError::Invalid("an output of the work package has no known condition".to_owned()));
    };

    Ok(Block {
        uuid: text(item, "uuid"),
        kind,
        x: item.get("x").and_then(Value::as_i64).unwrap_or(0),
        y: item.get("y").and_then(Value::as_i64).unwrap_or(0),
        fields: BlockFields {
            name: text(item, "name"),
            evidence: text(item, "evidence"),
            instructions: text(item, "instructions"),
            target: text(item, "target"),
            format,
            condition,
        },
        proposal: None,
    })
}

/// The attachments of the file: documents of OSCAL, catalogs only, each
/// checked against the schema of NIST.
///
/// Called by: [`read`].
/// Calls: `OscalDocument::from_json`.
fn attachments(value: &Value) -> Result<Vec<OscalDocument>, AppError> {
    let items = array(value, "attachments", MAX_ATTACHMENTS)?;
    let mut found = Vec::with_capacity(items.len());

    for item in items {
        let document = OscalDocument::from_json(&item.to_string())
            .map_err(|error| AppError::Invalid(format!("an attachment of the work package is no OSCAL: {error}")))?;

        ensure!(
            document.model() == OscalModel::Catalog,
            AppError::Invalid("an attachment of the work package is no catalog".to_owned())
        );

        found.push(document);
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `read.rs`.

    use super::*;

    use serde_json::json;

    fn file(tasks: Value) -> String {
        let catalog = crate::test_support::db::basic_catalog().to_json().unwrap();

        json!({
            "format": FORMAT,
            "version": 1,
            "project": { "uuid": "p", "title": "CC", "scope": "A reader." },
            "package": { "uuid": "w", "name": "Roles", "evaluator": "Anna", "note": "" },
            "results": [],
            "tasks": tasks,
            "attachments": [catalog],
        })
        .to_string()
    }

    fn one_task() -> Value {
        let block = |uuid: &str, kind: &str, name: &str| {
            json!({ "uuid": uuid, "kind": kind, "x": 0, "y": 0, "name": name, "evidence": "", "instructions": "",
                    "target": "", "format": "text", "condition": "always" })
        };

        json!([{
            "uuid": "t", "catalog_uuid": "c", "control_id": "s1.1.1", "label": "1.1.1", "title": "Roles",
            "blocks": [block("r", "requirement", ""), block("i", "input", "Roles document"), block("o", "output", "pETR")],
            "connections": [{ "from": "i", "to": "r" }, { "from": "r", "to": "o" }],
            "related": [],
        }])
    }

    #[test]
    fn a_package_is_read_with_its_catalogs() {
        let (package, catalogs) = read(&file(one_task())).unwrap();

        assert_eq!((package.package_uuid.as_str(), package.evaluator.as_str()), ("w", "Anna"));
        assert_eq!(package.tasks[0].blocks.len(), 3);
        assert_eq!(catalogs.len(), 1);
    }

    #[test]
    fn what_is_no_package_is_refused() {
        assert!(read("not json").is_err());
        assert!(read(&json!({ "format": "other", "version": 1 }).to_string()).is_err());
        assert!(read(&file(one_task()).replace("\"version\":1", "\"version\":2")).is_err());
        assert!(read(&file(json!([]))).is_err(), "a package without tasks");

        let mut broken = one_task();
        broken[0]["connections"] = json!([{ "from": "o", "to": "r" }]);
        assert!(read(&file(broken)).is_err(), "a connection the plan does not allow");
    }
}
