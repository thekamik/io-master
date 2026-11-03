//! The templates of the reports (`ReportTemplate`), in the table
//! `report_templates` of the database of the user: create, read, update;
//! never deleted (user, 2026-11-03).
//!
//! ```text
//! report_templates   id, name, extension, text
//! ```

use rusqlite::{Row, named_params};

use crate::infrastructure::storage::error::DbError;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::evaluation::report_template::report_template_model::ReportTemplate;

/// The most templates in the database.
pub const MAX_REPORT_TEMPLATES: usize = 500;

/// The table, made when it is missing (`objects/schema.rs`, `USER_TABLES`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS report_templates (
        id        INTEGER PRIMARY KEY,
        name      TEXT NOT NULL,
        extension TEXT NOT NULL,
        text      TEXT NOT NULL
    );
";

/// Stores a new template ("ETR after the CEM") and gives its id.
///
/// Input: `db`, the open database of the user; `template`, the new
/// template, its `id` 0 (the database gives it), a name and an extension.
///
/// Output: the id of the template (3); an error when it has an id, no name
/// or no extension, or when the database holds [`MAX_REPORT_TEMPLATES`]
/// already.
///
/// Called by: `objects/evaluation/report_template/report_template_rules/templates.rs`: `new_template`.
/// Calls: [`count_report_templates`].
pub fn create_report_template(db: &UserDatabase, template: &ReportTemplate) -> Result<i64, DbError> {
    ensure!(template.id == 0, DbError::Inconsistent("a new template has an id already".to_owned()));
    ensure!(
        !template.name.is_empty() && !template.extension.is_empty(),
        DbError::Inconsistent("a template without a name or an extension".to_owned())
    );
    ensure!(
        count_report_templates(db)? < MAX_REPORT_TEMPLATES,
        DbError::Inconsistent(format!("more than {MAX_REPORT_TEMPLATES} templates"))
    );

    let connection = db.connection();

    connection.execute(
        "INSERT INTO report_templates (name, extension, text) VALUES (:name, :extension, :text)",
        named_params! { ":name": template.name, ":extension": template.extension, ":text": template.text },
    )?;

    Ok(connection.last_insert_rowid())
}

/// The number of the templates in the database.
///
/// Called by: [`create_report_template`].
fn count_report_templates(db: &UserDatabase) -> Result<usize, DbError> {
    let count: i64 = db.connection().query_row("SELECT COUNT(*) FROM report_templates", [], |row| row.get(0))?;

    usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// The template of the row `row`, read by the names of its columns.
///
/// Called by: [`read_report_template`], [`read_report_templates`].
fn template_of(row: &Row<'_>) -> Result<ReportTemplate, DbError> {
    Ok(ReportTemplate {
        id: row.get("id")?,
        name: row.get("name")?,
        extension: row.get("extension")?,
        text: row.get("text")?,
    })
}

/// Reads the template `id`, with its text.
///
/// Input: `db`, the open database of the user; `id`, the id of the template
/// (3).
///
/// Output: the template; `DbError::NoRow` when there is none.
///
/// Called by: [`update_report_template`];
/// `objects/evaluation/report_template/report_template_rules/filling.rs`: `filled_report`;
/// `objects/evaluation/report_template/report_template_rules/templates.rs`: `save_template`;
/// `panels/views/reports/templates.rs`: `report_template_get`.
/// Calls: [`template_of`].
pub fn read_report_template(db: &UserDatabase, id: i64) -> Result<ReportTemplate, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT id, name, extension, text FROM report_templates WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "report_templates", id });
    };

    template_of(row)
}

/// Reads every template, by their names.
///
/// Input: `db`, the open database of the user.
///
/// Output: the templates, with their texts, ordered by name; an error when
/// the table holds more than [`MAX_REPORT_TEMPLATES`] (a database the
/// program did not write).
///
/// Called by: `panels/views/reports/templates.rs`: `report_template_list`.
/// Calls: [`template_of`].
pub fn read_report_templates(db: &UserDatabase) -> Result<Vec<ReportTemplate>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT id, name, extension, text FROM report_templates ORDER BY name, id")?;
    let mut rows = statement.query([])?;
    let mut templates = Vec::new();

    for _ in 0..=MAX_REPORT_TEMPLATES {
        let Some(row) = rows.next()? else {
            return Ok(templates);
        };

        templates.push(template_of(row)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_REPORT_TEMPLATES} templates in the database")))
}

/// Writes the template `template.id` anew: its name, extension and text.
///
/// Input: `db`, the open database of the user; `template`, the template as
/// it is to be kept, its `id` that of a stored one, a name and an extension.
///
/// Output: nothing; an error when it has no name or no extension,
/// `DbError::NoRow` when there is no template `template.id`.
///
/// Called by: `objects/evaluation/report_template/report_template_rules/templates.rs`: `change_template`.
pub fn update_report_template(db: &UserDatabase, template: &ReportTemplate) -> Result<(), DbError> {
    ensure!(
        !template.name.is_empty() && !template.extension.is_empty(),
        DbError::Inconsistent("a template without a name or an extension".to_owned())
    );

    let changed = db.connection().execute(
        "UPDATE report_templates SET name = :name, extension = :extension, text = :text WHERE id = :id",
        named_params! {
            ":id": template.id,
            ":name": template.name,
            ":extension": template.extension,
            ":text": template.text,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "report_templates", id: template.id });

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the templates of the reports: stored, read, changed.

    use super::*;

    use crate::test_support::database::database;

    fn etr() -> ReportTemplate {
        ReportTemplate {
            id: 0,
            name: "ETR".to_owned(),
            extension: "md".to_owned(),
            text: "# {{evaluation_plan.product}}".to_owned(),
        }
    }

    #[test]
    fn a_template_is_stored_read_and_changed() {
        let (_directory, db) = database();
        let id = create_report_template(&db, &etr()).unwrap();

        assert_eq!(read_report_template(&db, id).unwrap(), ReportTemplate { id, ..etr() });

        let changed = ReportTemplate { id, name: "Annex".to_owned(), extension: "tex".to_owned(), ..etr() };

        update_report_template(&db, &changed).unwrap();

        assert_eq!(read_report_templates(&db).unwrap(), vec![changed]);
    }

    #[test]
    fn a_template_without_a_name_or_missing_is_refused() {
        let (_directory, db) = database();

        assert!(matches!(
            create_report_template(&db, &ReportTemplate { name: String::new(), ..etr() }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(
            create_report_template(&db, &ReportTemplate { id: 4, ..etr() }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(read_report_template(&db, 5), Err(DbError::NoRow { .. })));
        assert!(matches!(update_report_template(&db, &ReportTemplate { id: 5, ..etr() }), Err(DbError::NoRow { .. })));
    }
}
