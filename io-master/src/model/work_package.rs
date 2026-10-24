//! A work package of a master project (step 51.5 of docs/history.md): the
//! tasks the planner delegates to one evaluator, with a name and a note. A
//! package only points to tasks of its master, so a scheme is made once; a
//! task is in one package at most, so that nothing is delegated twice. Its
//! export takes a picture of the state at that moment. Plain data and its
//! rules; stored by `db/evaluation_project/`.

use super::evaluation_project::MAX_TASKS;
use super::text::{Field, Invalid, check_text};

/// The longest name of a package.
pub const NAME_MAX: usize = 200;

/// The longest name of the evaluator a package goes to.
pub const EVALUATOR_MAX: usize = 200;

/// The longest note of a package.
pub const NOTE_MAX: usize = 4000;

/// The most packages of a project.
pub const MAX_PACKAGES: usize = 200;

/// What the planner writes of a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageFields {
    pub name: String,
    /// The evaluator it goes to, as the laboratory names them.
    pub evaluator: String,
    pub note: String,
}

/// A package: what the planner wrote and the UUIDs of its tasks, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkPackage {
    pub uuid: String,
    pub fields: PackageFields,
    pub tasks: Vec<String>,
}

/// The fields in their normal form: no spaces around them.
///
/// Called by: `work_packages_add`, `work_packages_save` (`views/evaluation_plan/evaluation_plan.rs`).
pub fn normalized(fields: &PackageFields) -> PackageFields {
    PackageFields {
        name: fields.name.trim().to_owned(),
        evaluator: fields.evaluator.trim().to_owned(),
        note: fields.note.trim().to_owned(),
    }
}

/// Checks the fields of a package: a name of one line, the evaluator of
/// one line, a note of at most [`NOTE_MAX`] characters.
///
/// Called by: `work_packages_add`, `work_packages_save`.
pub fn check_fields(fields: &PackageFields) -> Result<(), Invalid> {
    if fields.name.is_empty() {
        return Err(Invalid::new(Field::Name, "the package needs a name"));
    }

    check_text(Field::Name, &fields.name, NAME_MAX, false)?;
    check_text(Field::Evaluator, &fields.evaluator, EVALUATOR_MAX, false)?;
    check_text(Field::Text, &fields.note, NOTE_MAX, true)
}

/// Checks the tasks chosen for a package: at least one, at most
/// [`MAX_TASKS`], none twice.
///
/// Called by: `work_packages_add`, `work_packages_save`.
pub fn check_tasks(tasks: &[String]) -> Result<(), Invalid> {
    if tasks.is_empty() {
        return Err(Invalid::new(Field::Project, "choose at least one task"));
    }

    if tasks.len() > MAX_TASKS {
        return Err(Invalid::new(Field::Project, format!("a package holds at most {MAX_TASKS} tasks")));
    }

    for (index, task) in tasks.iter().enumerate() {
        for other in tasks.iter().skip(index + 1) {
            if other == task {
                return Err(Invalid::new(Field::Project, "a task is chosen twice"));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `work_package.rs`.

    use super::*;

    fn fields(name: &str, evaluator: &str) -> PackageFields {
        PackageFields { name: name.to_owned(), evaluator: evaluator.to_owned(), note: String::new() }
    }

    #[test]
    fn a_package_needs_a_name_and_one_line_for_its_evaluator() {
        assert!(check_fields(&fields("ADV for Anna", "Anna")).is_ok());
        assert!(check_fields(&fields("", "Anna")).is_err());
        assert!(check_fields(&fields("ADV", "two\\nlines")).is_ok(), "an escaped text is one line");
        assert!(check_fields(&fields("ADV", "two\nlines")).is_err());
        assert_eq!(normalized(&fields(" ADV ", " Anna ")), fields("ADV", "Anna"));
    }

    #[test]
    fn the_tasks_are_some_and_each_once() {
        let tasks = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();

        assert!(check_tasks(&tasks(&["t1", "t2"])).is_ok());
        assert!(check_tasks(&[]).is_err());
        assert!(check_tasks(&tasks(&["t1", "t1"])).is_err());
    }
}
