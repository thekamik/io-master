//! The data of the work packages of a master project (step 51.5 of
//! docs/history.md), as the page gets it and sends it.

use serde::{Deserialize, Serialize};

use crate::model_old::work_package::{PackageFields, WorkPackage};

/// A package.
#[derive(Debug, Serialize)]
pub struct WorkPackageView {
    pub uuid: String,
    pub name: String,
    pub evaluator: String,
    pub note: String,
    /// The UUIDs of its tasks, in order.
    pub tasks: Vec<String>,
}

/// The packages as the page shows them.
///
/// Called by: `work_packages_get` and the commands of the packages (`evaluation_plan.rs`).
pub fn all(packages: Vec<WorkPackage>) -> Vec<WorkPackageView> {
    let mut views = Vec::with_capacity(packages.len());

    for package in packages {
        views.push(WorkPackageView {
            uuid: package.uuid,
            name: package.fields.name,
            evaluator: package.fields.evaluator,
            note: package.fields.note,
            tasks: package.tasks,
        });
    }

    views
}

/// A package, as the page sends it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkPackageForm {
    pub name: String,
    pub evaluator: String,
    pub note: String,
    pub tasks: Vec<String>,
}

impl WorkPackageForm {
    pub fn fields(&self) -> PackageFields {
        PackageFields { name: self.name.clone(), evaluator: self.evaluator.clone(), note: self.note.clone() }
    }
}
