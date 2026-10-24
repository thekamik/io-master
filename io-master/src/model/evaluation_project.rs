//! The master project of an evaluation (step 51 of docs/history.md): the
//! catalogs of a standard and all its tasks, each a whole requirement (a
//! control of a catalog). It is made once for a version of a standard and
//! used again and again; the packages handed to the evaluators are taken
//! from it (`work_package`, step 51.5). Plain data and its rules; stored by
//! `db/evaluation_project/`, never written into a document of OSCAL.

use super::text::{Field, Invalid, check_text};

/// The longest title of a project.
pub const TITLE_MAX: usize = 200;

/// The longest description of a project (its scope).
pub const DESCRIPTION_MAX: usize = 4000;

/// The most catalogs a project uses (the parts of a standard, its
/// methodology, Protection Profiles).
pub const MAX_CATALOGS: usize = 20;

/// The most tasks a project holds: a whole standard has some hundreds of
/// requirements.
pub const MAX_TASKS: usize = 5000;

/// The most related elements of one task.
pub const MAX_LINKS: usize = 100;

/// The most projects an evaluator keeps.
pub const MAX_PROJECTS: usize = 500;

/// What the planner writes of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFields {
    pub title: String,
    pub description: String,
}

/// A project as a row of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    pub uuid: String,
    pub title: String,
    pub catalogs: usize,
    pub tasks: usize,
}

/// A project: what the planner wrote and the catalogs it uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationProject {
    pub uuid: String,
    pub fields: ProjectFields,
    /// The UUIDs of its catalogs, in the order they were chosen.
    pub catalogs: Vec<String>,
}

/// A task of a project: a whole requirement, a control of one of its
/// catalogs. Its text is read from the catalog, never copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationTask {
    pub uuid: String,
    pub catalog_uuid: String,
    pub control_id: String,
}

/// A requirement the AI proposes as a task of a project (step 51.4), with
/// its reason; the planner accepts (it becomes a task) or rejects it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskProposal {
    pub catalog_uuid: String,
    pub control_id: String,
    pub reason: String,
}

/// An element of another catalog related to the requirement of a task
/// (step 51.4): a work unit of the CEM, a test case, ... Proposed by the AI
/// with its reason; once the planner accepts it, it travels with the task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLink {
    pub catalog_uuid: String,
    pub control_id: String,
    pub reason: String,
    /// Whether it is only proposed by the AI.
    pub proposed: bool,
}

/// The fields in their normal form: no spaces around them.
///
/// Called by: the commands of the view Evaluation plan, before
/// [`check_fields`].
pub fn normalized(fields: &ProjectFields) -> ProjectFields {
    ProjectFields { title: fields.title.trim().to_owned(), description: fields.description.trim().to_owned() }
}

/// Checks the fields of a project: a title of one line, 1 to
/// [`TITLE_MAX`] characters; a description of at most [`DESCRIPTION_MAX`].
///
/// Called by: `new_project_create`, `project_form_save`
/// (`views/evaluation_plan/evaluation_plan.rs`).
pub fn check_fields(fields: &ProjectFields) -> Result<(), Invalid> {
    if fields.title.is_empty() {
        return Err(Invalid::new(Field::Title, "the project needs a title"));
    }

    check_text(Field::Title, &fields.title, TITLE_MAX, false)?;
    check_text(Field::Description, &fields.description, DESCRIPTION_MAX, true)?;

    Ok(())
}

/// Checks the catalogs chosen for a project: at least one, at most
/// [`MAX_CATALOGS`], none twice.
///
/// Called by: `new_project_create`, `project_form_save`.
pub fn check_catalogs(catalogs: &[String]) -> Result<(), Invalid> {
    if catalogs.is_empty() {
        return Err(Invalid::new(Field::Documents, "choose at least one catalog"));
    }

    if catalogs.len() > MAX_CATALOGS {
        return Err(Invalid::new(Field::Documents, format!("a project uses at most {MAX_CATALOGS} catalogs")));
    }

    for (index, catalog) in catalogs.iter().enumerate() {
        for other in catalogs.iter().skip(index + 1) {
            if other == catalog {
                return Err(Invalid::new(Field::Documents, "a catalog is chosen twice"));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `evaluation_project.rs`.

    use super::*;

    fn fields(title: &str, description: &str) -> ProjectFields {
        ProjectFields { title: title.to_owned(), description: description.to_owned() }
    }

    #[test]
    fn the_title_is_needed_and_of_one_line() {
        assert!(check_fields(&fields("CC:2022 R1", "")).is_ok());
        assert!(check_fields(&fields("", "scope")).is_err());
        assert!(check_fields(&fields("two\nlines", "")).is_err());
        assert!(check_fields(&fields(&"t".repeat(TITLE_MAX + 1), "")).is_err());
        assert!(check_fields(&fields("ok", "line one\nline two")).is_ok());
    }

    #[test]
    fn the_normal_form_has_no_spaces_around() {
        assert_eq!(normalized(&fields("  CC  ", " scope \n")), fields("CC", "scope"));
    }

    #[test]
    fn the_catalogs_are_some_and_each_once() {
        let catalogs = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();

        assert!(check_catalogs(&catalogs(&["a", "b"])).is_ok());
        assert!(check_catalogs(&[]).is_err());
        assert!(check_catalogs(&catalogs(&["a", "a"])).is_err());
    }
}
