//! Reading which connection a role of the AI uses.

use rusqlite::OptionalExtension;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::ai::Role;

/// The connection given to `role`; `None` when it has none (it uses
/// the active connection).
pub fn connection(db: &UserDb, role: Role) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT connection_id FROM ai_roles WHERE role = ?1", [role.name()], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .optional()?
        .flatten())
}

#[cfg(test)]
mod tests {
    //! Tests of reading which connection a role of the AI uses.

    use super::*;

    use crate::db::ai::connection::create;
    use crate::db::ai::roles::update;
    use crate::test_support::db::named;
    use crate::test_support::standards::database;

    #[test]
    fn a_role_without_a_connection_of_its_own_reads_none() {
        let test = database();

        assert_eq!(connection(&test.db, Role::Summary).unwrap(), None);
    }

    #[test]
    fn a_role_reads_the_connection_it_was_given() {
        let test = database();
        let id = create::connection(&test.db, &named("Local")).unwrap().id;
        update::connection(&test.db, Role::Summary, Some(id)).unwrap();

        assert_eq!(connection(&test.db, Role::Summary).unwrap(), Some(id));
    }
}
