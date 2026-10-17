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

mod ai {
    pub mod api {
        pub mod api_key;
        pub mod checks;
        pub mod completions;
        pub mod connection;
        // The communication with the provider: only `ai/api/` reaches
        // it; the rest of the program asks through the files above.
        pub(in crate::ai::api) mod provider {
            pub mod ai_error;
            pub mod client;
            pub mod json_path;
            pub mod request;
            pub mod response;
            pub mod sse;
        }
        pub mod roles;
        pub mod tasks;
        #[cfg(test)]
        pub mod test_support;
    }
    pub mod chat {
        pub mod chats;
        pub mod conversation;
        pub mod memory;
    }
}

mod evaluation {
    pub mod analysis;
    pub mod checks;
    pub mod contents;
    pub mod documents;
    pub mod exchange;
    pub mod pdf_text;
    pub mod projects;
    pub mod quote;
    pub mod scheme;
    pub mod schemes;
    #[cfg(test)]
    pub mod test_support;
}

mod product_evaluation {
    pub mod import;
    pub mod journal;
    pub mod model;
    pub mod projects;
    pub mod results;
    #[cfg(test)]
    pub mod test_support;
}

mod accounts {
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod accounts;
}

mod installation {
    // The file is installation/installation.rs by choice of the
    // project layout; clippy would prefer a different name for it.
    #[allow(clippy::module_inception)]
    pub mod installation;
    pub mod lifecycle;
    pub mod verify;
}

mod devices {
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod devices;
    pub mod port_property;
    pub mod scan_settings;
}

mod tpm {
    pub mod device_secret;
    pub mod emulator;
    #[cfg(test)]
    pub mod test_support;
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod tpm;
}

mod db {
    pub mod encryption {
        pub mod database_key;
        pub mod generate_key;
        pub mod keys;
        pub mod sqlcipher;
    }
    pub mod error;
    pub mod system {
        pub mod create;
        pub mod database;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
    pub mod user {
        pub mod database;
        pub mod delete;
        pub mod read;
        pub mod schema;
        pub mod update;
    }
    pub mod ai {
        pub mod connection {
            pub mod create;
            pub mod delete;
            pub mod read;
            pub mod rows;
            pub mod schema;
            pub mod update;
        }
        pub mod instructions {
            pub mod delete;
            pub mod read;
            pub mod schema;
            pub mod update;
        }
        pub mod journal {
            pub mod create;
            pub mod read;
            pub mod rows;
            pub mod schema;
        }
        pub mod memory {
            pub mod create;
            pub mod delete;
            pub mod read;
            pub mod rows;
            pub mod schema;
        }
        pub mod presets {
            pub mod field_help;
            pub mod read;
        }
        pub mod roles {
            pub mod read;
            pub mod schema;
            pub mod update;
        }
    }
    pub mod standards {
        pub mod create;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
    pub mod test_projects {
        pub mod create;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
    pub mod evaluations {
        pub mod create;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
}

mod random {
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod random;
}

mod tauri_ui {
    pub mod account_actions;
    pub mod ai_config_actions;
    pub mod blocking;
    pub mod chat_actions;
    pub mod commands;
    pub mod error;
    pub mod project_actions;
    pub mod scan_actions;
    pub mod scheme_actions;
    pub mod serializer {
        pub mod ai_config_view;
        pub mod app_view;
        pub mod chat_view;
        pub mod error_view;
        pub mod project_view;
        pub mod scan_view;
        pub mod scheme_view;
        pub mod standard_view;
    }
    pub mod standard_actions;
    pub mod state;
    #[cfg(test)]
    pub mod test_support;
    pub mod user_data;
}

use std::path::PathBuf;
use std::process::ExitCode;

use installation::lifecycle::Opened;
use installation::verify::{InstallationState, verify_installation};
use tauri_ui::commands;
use tauri_ui::state::UiState;

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
        // session, the work in progress (tauri_ui/state.rs).
        .manage(UiState::new(data_dir, opened))
        // The commands the page may call. Each is a thin function in
        // tauri_ui/commands.rs that hands over to the file named in
        // its group, where the work is done. A command must also be
        // listed in build.rs and capabilities/default.json, or the
        // page is refused when it calls it.
        .invoke_handler(tauri::generate_handler![
            // The program and the account (account_actions.rs).
            commands::app_state,
            commands::set_up,
            commands::log_in,
            commands::log_out,
            commands::change_password,
            // Settings, tab AI: the connections to AI services
            // (ai_config_actions.rs).
            commands::ai_config,
            commands::save_ai_connection,
            commands::delete_ai_connection,
            commands::activate_ai_connection,
            commands::store_ai_key,
            commands::delete_ai_key,
            commands::check_ai_connection,
            commands::ai_field_instruction,
            // The scan of the serial ports (scan_actions.rs).
            commands::scan_ports,
            commands::save_scan_settings,
            // The chat with the AI (chat_actions.rs).
            commands::list_chats,
            commands::open_chat,
            commands::delete_chat,
            commands::send_message,
            commands::stop_reply,
            // Create evaluation scheme: the documents of the standards
            // (standard_actions.rs).
            commands::list_documents,
            commands::load_documents,
            commands::stop_work,
            commands::update_document,
            commands::delete_document,
            commands::document_page,
            commands::search_documents,
            commands::document_pdf,
            // Create evaluation scheme: the test projects, the list of
            // their schemes, the table of contents read by the AI, the
            // exchange as a file (project_actions.rs).
            commands::list_projects,
            commands::create_project,
            commands::open_project,
            commands::update_project,
            commands::set_project_ready,
            commands::delete_project,
            commands::create_scheme,
            commands::update_scheme,
            commands::accept_schemes,
            commands::delete_schemes,
            commands::propose_schemes,
            commands::export_project,
            commands::import_project,
            // Create evaluation scheme: the drawing of one scheme, its
            // tiles and links, and the analysis of chapters by the AI
            // (scheme_actions.rs).
            commands::open_scheme,
            commands::add_tile,
            commands::update_tile,
            commands::move_tile,
            commands::move_tiles,
            commands::delete_tiles,
            commands::accept_tiles,
            commands::add_link,
            commands::delete_link,
            commands::undo_scheme,
            commands::analysis_instructions,
            commands::save_analysis_instructions,
            commands::analyse_scheme,
            commands::analyse_schemes,
        ])
        .run(tauri::generate_context!());

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: the window could not be opened: {error}");
            ExitCode::FAILURE
        }
    }
}

