//! Entry point of IO Master.
//!
//! The program is used through its web user interface, which is
//! built in step 1 of `docs/todo.md`. Until then `main` only says
//! so; the modules below are the parts that already exist.

mod devices {
    // The file is devices/devices.rs by choice of the project
    // layout; clippy would prefer a different name for it.
    #[allow(clippy::module_inception)]
    pub mod devices;
}

mod keyring {
    // Same layout as devices/devices.rs.
    #[allow(clippy::module_inception)]
    pub mod keyring;

    pub mod database_key;
}

mod db {
    pub mod generate_key;
}

fn main() {
    // TODO (step 1 of docs/todo.md): start the web server here.
    println!("IO Master: the web user interface is not implemented yet.");
}
