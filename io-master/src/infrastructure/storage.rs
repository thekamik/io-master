//! `infrastructure/storage/`: the two database files (system.v2.db,
//! users/<id>.v2.db): where they are, made, opened, their tables made from
//! the list given by their caller; the help for the lists of a model; their
//! encryption; `DbError`, the error of every function on a database.

pub mod connection;
pub mod encryption;
pub mod error;
pub mod lists;
pub mod system_database;
pub mod user_database;
