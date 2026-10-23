//! Giving a role of the AI a connection of its own, or taking it away.

use rusqlite::named_params;

use crate::db::ai::connection::read as connections;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::ai::Role;

/// Gives `role` the connection `id`, or none (`None`: the role uses the
/// active connection). Returns `false` when the connection does not
/// exist; nothing is changed then.
pub fn connection(db: &UserDb, role: Role, id: Option<i64>) -> Result<bool, DbError> {
    if let Some(id) = id
        && connections::get(db, id)?.is_none()
    {
        return Ok(false);
    }

    db.connection().execute(
        "INSERT INTO ai_roles (role, connection_id) VALUES (:role, :connection_id) \
         ON CONFLICT (role) DO UPDATE SET connection_id = excluded.connection_id",
        named_params! { ":role": role.name(), ":connection_id": id },
    )?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    //! Tests of giving a role of the AI a connection of its own, or taking it
    //! away.

    use super::*;

    use crate::db::ai::connection::create;
    use crate::db::ai::roles::read;
    use crate::test_support::db::named;
    use crate::test_support::standards::database;

    #[test]
    fn a_role_gets_a_connection_and_loses_it() {
        let test = database();
        let id = create::connection(&test.db, &named("Local")).unwrap().id;

        assert!(connection(&test.db, Role::Summary, Some(id)).unwrap());
        assert_eq!(read::connection(&test.db, Role::Summary).unwrap(), Some(id));
        assert!(connection(&test.db, Role::Summary, None).unwrap());
        assert_eq!(read::connection(&test.db, Role::Summary).unwrap(), None);
    }

    #[test]
    fn a_connection_that_does_not_exist_is_not_given() {
        let test = database();
        let id = create::connection(&test.db, &named("Local")).unwrap().id;

        assert!(!connection(&test.db, Role::Summary, Some(id + 1)).unwrap());
    }
}
