//! Entry point of IO Master.
//!
//! IO Master is a desktop program: it is started without parameters
//! and opens a window. All configuration is made in the window, also
//! the network functions of a later step (the reset service of a
//! master and its client, step 11 of docs/todo.md), which are
//! started from there and not from the command line.
//!
//! `main` has two tasks, and does each by calling the module that
//! owns it:
//!
//! 1. checks the installation of this Linux account
//!    (`installation/verify.rs`); a damaged installation is a
//!    critical error and the program does not start;
//! 2. opens the window ([`tauri_startup`] below) and runs until it
//!    is closed.
//!
//! At the first start nothing is installed: the window asks for the
//! user and the role of the installation.
//!
//! TODO (step 13 of docs/todo.md): the location of the installation
//! and the secret of the application come from the keyring of the
//! Linux account; one Linux account holds one installation with one
//! user (docs/architecture.md, sections 3 and 5.6).

// The check `ensure!` (rule 5 of CLAUDE.md), declared before the
// folders so that every one of them knows it by its name alone.
#[macro_use]
mod checks;

// ── The map of the program ──────────────────────────────────────────
// Every folder of the program, in the order of its layers from the bottom
// up: a folder uses only the folders above it in this list
// (docs/code_structure.md, section 1; checked by architecture_tests.rs).
// The comment of a group says which part of the project it is; what each
// file holds is said at its functions. Since step 45 of docs/todo.md the
// modules of every layer are written here; components/ and views/ still
// declare theirs in their mod.rs (step 46), and a file with code that has
// modules of its own (model/oscal.rs) declares them itself.

// model/: the data of the program as plain Rust types and the rules of
// their validity; no I/O (no database, no network, no file).
mod model {
    pub mod ai;
    pub mod api_key;
    pub mod chats;
    pub mod evaluation_support;
    pub mod installation;
    pub mod oscal;
    pub mod standards;
    pub mod text;
    pub mod time;
}

// security/: the building blocks of protection: random numbers from the
// system, and the TPM 2.0, the optional further factor. The encryption of
// the databases is in db/encryption/.
mod security {
    pub mod random;
    // security/tpm/: the TPM 2.0 (docs/architecture.md, section 5.1).
    pub mod tpm {
        pub mod chip;
        pub mod device_secret;
        pub mod emulator;
        #[cfg(test)]
        pub mod test_support;
    }
}

// db/: every operation on a database, as CRUD, one folder per area of data
// (CLAUDE.md, "Database code"). The schema.rs of an area says in which
// database its data lives and how it is protected.
mod db {
    pub mod error;
    // db/encryption/: all code of encryption of the databases.
    pub mod encryption {
        pub mod database_key;
        pub mod generate_key;
        pub mod keys;
        pub mod sqlcipher;
    }
    // db/system/: the system database of the installation (not encrypted).
    pub mod system {
        pub mod create;
        pub mod database;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
    // db/user/: the database of one evaluator (encrypted), its settings
    // and secrets.
    pub mod user {
        pub mod database;
        pub mod delete;
        pub mod migrations;
        pub mod read;
        pub mod schema;
        pub mod update;
    }
    // db/ai/: the data of the AI (the database of the evaluator).
    pub mod ai {
        // db/ai/chats/: the chats with the AI.
        pub mod chats {
            pub mod create;
            pub mod delete;
            pub mod read;
            pub mod rows;
            pub mod schema;
        }
        // db/ai/connection/: the AI connections and their API keys.
        pub mod connection {
            pub mod create;
            pub mod delete;
            pub mod read;
            pub mod rows;
            pub mod schema;
            pub mod update;
        }
        // db/ai/instructions/: the instructions of the tasks of the AI.
        pub mod instructions {
            pub mod delete;
            pub mod read;
            pub mod schema;
            pub mod update;
        }
        // db/ai/journal/: the journal of the AI, never changed.
        pub mod journal {
            pub mod create;
            pub mod read;
            pub mod rows;
            pub mod schema;
        }
        // db/ai/roles/: the connection each role of the AI uses.
        pub mod roles {
            pub mod read;
            pub mod schema;
            pub mod update;
        }
    }
    // db/standards/: the documents of standards, their PDFs and the text of
    // their pages (the database of the evaluator).
    pub mod standards {
        pub mod create;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
    // db/oscal/: the documents of NIST OSCAL (the database of the evaluator).
    pub mod oscal {
        pub mod create;
        pub mod delete;
        pub mod error;
        pub mod ids;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
    // db/evaluation_support/: what the program adds to OSCAL, linked by
    // UUIDs and ids (the database of the evaluator).
    pub mod evaluation_support {
        pub mod create;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
    }
    // db/scan_settings/: the settings of the scanner of serial ports, as two
    // texts in the settings of the evaluator (its file has the names of the
    // settings, so it declares its own modules).
    pub mod scan_settings;
}

// ai/: the whole communication with AI services (CLAUDE.md, "AI layer").
mod ai {
    // ai/api/: the only way to the AI, in the form of REST: resources and
    // operations; every question is written into the journal of the AI.
    pub mod api {
        pub mod checks;
        pub mod completions;
        pub mod connection;
        pub mod connections;
        pub mod roles;
        pub mod tasks;
        #[cfg(test)]
        pub mod test_support;
        // ai/api/provider/: the communication with the provider (HTTP, TLS,
        // request, answer, stream). Only ai/api/ reaches it; the rest of the
        // program asks through the files above.
        pub(in crate::ai::api) mod provider {
            pub mod ai_error;
            pub mod client;
            pub mod json_path;
            pub mod request;
            pub mod response;
            pub mod sse;
        }
    }
    // ai/tasks/: for each task of the AI, the question sent to the model and
    // the reading of its answer, on plain data of model/.
    pub mod tasks {
        pub mod catalog_outline;
        pub mod catalog_section;
        pub mod chat;
        mod json;
    }
    // ai/presets/: the ready settings of well-known AI services (its file has
    // the presets themselves, so it declares its own module).
    pub mod presets;
}

// hardware_tests/: everything that talks to hardware. No command reaches a
// device without an action of the evaluator for it (P2 of
// docs/architecture.md). permissions/permission.rs is empty until step 6 of
// docs/todo.md and not compiled.
mod hardware_tests {
    // hardware_tests/devices/: the scan of the serial ports, and the
    // settings of the scanner.
    pub mod devices {
        pub mod port_property;
        pub mod scan_settings;
        pub mod serial_ports;
    }
}

// pdf/: reading PDFs with the tools of Poppler (pdfinfo, pdftotext): the only
// place where the program starts another program on a document of the
// evaluator, kept in one folder for the audit. It uses nothing of the program.
mod pdf {
    pub mod poppler;
}

// installation/: the installation of this Linux account, its one user and
// the session of that user: the core of the protection of the program.
// uninstall.rs is not compiled ("DO NOT USE: TO BE REWORKED", step 12 of
// docs/todo.md).
mod installation {
    pub mod accounts;
    pub mod lifecycle;
    pub mod session;
    pub mod verify;
}

// app/: what every view uses: the state of the window, and the errors of
// the commands in kinds the page tells apart.
mod app {
    pub mod error;
    pub mod state;
    #[cfg(test)]
    pub mod test_support;
    // app/serializer/: how an error of a command reaches the page (serde).
    pub mod serializer {
        pub mod error_view;
    }
}

// components/: the elements of the window used more than once.
mod components;

// views/: the views of the window, one folder per view. The entry of a view
// is the file of its name in its folder: every command the page calls for
// it (step 46 of docs/todo.md); the folders of its elements around it hold
// their data and logic. The entry has the name of its folder, so its
// module has the name of its parent: clippy's `module_inception` is
// allowed there, on purpose.
mod views {
    // views/devices/: the view Devices: the serial ports of the computer and
    // the settings of the scanner.
    pub mod devices {
        #[allow(clippy::module_inception)]
        pub mod devices;

        pub mod column_view;
        pub mod stored_settings;
        pub mod ports_table {
            pub mod view;
        }
        pub mod scanner_settings {
            pub mod view;
        }
    }

    // views/start/: the view Start: the setup at the first start and the
    // login, before anybody is logged in.
    pub mod start {
        #[allow(clippy::module_inception)]
        pub mod start;
    }

    // views/shell/: the view Shell: the frame of the window around every page,
    // and the state of the program.
    pub mod shell {
        #[allow(clippy::module_inception)]
        pub mod shell;

        pub mod program {
            pub mod view;
        }
    }

    // views/chat/: the view Chat: the chats of the evaluator with the model of
    // the role consultation.
    pub mod chat {
        #[allow(clippy::module_inception)]
        pub mod chat;

        pub mod chat_list {
            pub mod view;
        }
        pub mod connection_line {
            pub mod view;
        }
        pub mod conversation {
            pub mod stored;
            pub mod turn;
            pub mod view;
        }
    }

    // views/settings/: the view Settings: the password of the evaluator, the
    // connections to AI services and their keys, what the installation is.
    pub mod settings {
        #[allow(clippy::module_inception)]
        pub mod settings;

        pub mod api_key {
            pub mod view;
        }
        pub mod connection_check {
            pub mod view;
        }
        pub mod connection_form {
            pub mod view;
        }
        pub mod connection_list {
            pub mod view;
        }
    }

    // views/evaluation_plan/: the view Evaluation plan: the documents of the
    // standards in NIST OSCAL and the documents of standards (PDF) they quote.
    pub mod evaluation_plan {
        #[allow(clippy::module_inception)]
        pub mod evaluation_plan;

        pub mod document_search {
            pub mod search;
            pub mod view;
        }
        pub mod new_catalog {
            pub mod made;
            pub mod view;
        }
        pub mod oscal_documents {
            pub mod dialog;
            pub mod file;
            pub mod view;
        }
        pub mod standard_documents {
            pub mod dialog;
            pub mod load;
            pub mod stored;
            pub mod view;
        }

        // The editor of a catalog, a screen of the view; it declares its
        // modules in its mod.rs until step 46.6.
        pub mod catalog_editor;
    }

    // Help for the tests of the views: a window of the mock runtime of Tauri.
    #[cfg(test)]
    pub mod test_support;
}

// Help for the tests: temporary folders, a database with a small standard.
#[cfg(test)]
mod test_support;

// The routes of the window: the list of the commands the page may call,
// above the views it names.
mod routes;

// The direction of the dependencies between the folders, and the four
// lists of the commands, checked on the source.
#[cfg(test)]
mod architecture_tests;
#[cfg(test)]
mod command_lists_tests;

use std::path::PathBuf;
use std::process::ExitCode;

use app::state::AppState;
use installation::lifecycle::Opened;
use installation::verify::{InstallationState, verify_installation};

// The two entries of the installation in the keyring of the Linux
// account (docs/architecture.md, section 5.6). These constants are
// the *names* under which the entries are stored, not their
// values: the location and the secret are read from the keyring
// at every start and never written into the program.
//
// TODO (step 13 of docs/todo.md): move both to
// installation/verify.rs, where the keyring will be read; unused
// until then.

/// Name of the keyring entry with the location of the installation
/// (the folder that holds its databases).
const KEYRING_DATA_DIR_ENTRY: &str = "io-master.data-dir";

/// Name of the keyring entry with the secret of the application
/// (32 random bytes, also the recovery code).
const KEYRING_APP_SECRET_ENTRY: &str = "io-master.app-secret";

// ok kk
fn main() -> ExitCode {
    // The program takes no parameters. One given anyway (an old
    // `--port`, a typing mistake) is refused rather than passed over,
    // so that nobody believes it had an effect.
    if let Some(parameter) = std::env::args().nth(1) {
        eprintln!("Error: IO Master takes no parameters (given: '{parameter}').");
        eprintln!("Everything is set in its window.");

        // 2 is the usual exit code for a wrong command line.
        return ExitCode::from(2);
    }

    // The state of the installation decides how the window starts
    // (installation/verify.rs). A damaged installation is a
    // critical error: the program does not start
    // (docs/architecture.md, section 3.2).
    let (data_dir, opened) = match verify_installation() {
        InstallationState::NotInstalled { data_dir } => (data_dir, Opened::FirstRun),
        InstallationState::Installed { data_dir, running } => (data_dir, Opened::Installed(running)),
        InstallationState::Damaged(damage) => {
            eprintln!("Error: {damage}.");
            eprintln!("IO Master does not start on a damaged installation.");
            return ExitCode::FAILURE;
        }
    };

    // Everything else is the start of the interface: the desktop
    // window (tauri_startup below).
    tauri_startup(data_dir, opened)
}

/// Opens the window for the installation in `data_dir`, in the state
/// `opened`, and runs until the window is closed. Returns the exit
/// code of the program; every failure is told in the terminal.
///
/// Nothing of the program runs here yet: the builder only registers
/// the parts (a plugin, the shared state, the table of commands), and
/// `run` opens the window and loops over its events. A command runs
/// later, when the page calls it by name.
///
/// The page is built into the program (`ui/dist`, made with
/// `npm run build`); it is never loaded from the network, and no
/// network port is opened. Its security: the Content-Security-Policy
/// of `tauri.conf.json` (only the own scripts and styles), and the
/// list of commands it may call.
fn tauri_startup(data_dir: PathBuf, opened: Opened) -> ExitCode {
    let result = tauri::Builder::default()
        // The dialogs of the system, used only from Rust (commands.rs).
        .plugin(tauri_plugin_dialog::init())
        // The state shared by all commands: the installation, the
        // session, the work in progress (app/state.rs).
        .manage(AppState::new(data_dir, opened))
        // The commands the page may call (routes.rs).
        .invoke_handler(routes::handler())
        .run(tauri::generate_context!());

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: the window could not be opened: {error}");
            ExitCode::FAILURE
        }
    }
}
