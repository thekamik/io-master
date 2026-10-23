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

// The folders of the program. Each declares its own modules in the
// file of the same name next to it (`db.rs` next to `db/`), so that
// a new file never changes this one; the whole tree is described in
// docs/code_structure.md.
mod ai;
mod app;
mod db;
mod hardware_tests;
mod installation;
mod model;
mod pdf;
mod security;
#[cfg(test)]
mod test_support;
mod views;

// The direction of the dependencies between the folders, and the four
// lists of the commands, checked on the source.
#[cfg(test)]
mod architecture_tests;
#[cfg(test)]
mod command_lists_tests;

use std::path::PathBuf;
use std::process::ExitCode;

use app::state::UiState;
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
        .manage(UiState::new(data_dir, opened))
        // The commands the page may call: those of the views, each in
        // the folder of its element (views/<view>/<element>/commands.rs).
        // A command must also be listed in build.rs and
        // capabilities/default.json, or the page is refused when it
        // calls it.
        .invoke_handler(tauri::generate_handler![
            // The view Start (views/start/) and the frame of the window
            // (views/shell/).
            views::start::setup_form::commands::setup_form_install,
            views::start::login_form::commands::login_form_log_in,
            views::shell::program::commands::program_get,
            views::shell::user_menu::commands::user_menu_log_out,
            // The view Settings (views/settings/).
            views::settings::password_form::commands::password_form_change,
            views::settings::connection_list::commands::connection_list_get,
            views::settings::connection_form::commands::connection_form_new,
            views::settings::connection_form::commands::connection_form_get,
            views::settings::connection_form::commands::connection_form_save,
            views::settings::connection_form::commands::connection_form_delete,
            views::settings::connection_form::commands::connection_form_activate,
            views::settings::connection_form::commands::connection_form_field_help,
            views::settings::api_key::commands::api_key_get,
            views::settings::api_key::commands::api_key_store,
            views::settings::api_key::commands::api_key_delete,
            views::settings::connection_check::commands::connection_check_run,
            // The view Scan (views/scan/).
            views::scan::ports_table::commands::ports_table_scan,
            views::scan::scanner_settings::commands::scanner_settings_get,
            views::scan::scanner_settings::commands::scanner_settings_save,
            // The view Chat (views/chat/).
            views::chat::connection_line::commands::connection_line_get,
            views::chat::chat_list::commands::chat_list_get,
            views::chat::conversation::commands::conversation_open,
            views::chat::conversation::commands::conversation_send,
            views::chat::conversation::commands::conversation_stop,
            views::chat::conversation::commands::conversation_delete,
            // The view Evaluation plan (views/plans/).
            views::plans::oscal_documents::commands::oscal_documents_get,
            views::plans::oscal_documents::commands::oscal_documents_import,
            views::plans::oscal_documents::commands::oscal_documents_export,
            views::plans::oscal_documents::commands::oscal_documents_delete,
            views::plans::new_catalog::commands::new_catalog_sources,
            views::plans::new_catalog::commands::new_catalog_create,
            views::plans::standard_documents::commands::standard_documents_get,
            views::plans::standard_documents::commands::standard_documents_load,
            views::plans::standard_documents::commands::standard_documents_stop,
            views::plans::standard_documents::commands::standard_documents_update,
            views::plans::standard_documents::commands::standard_documents_delete,
            views::plans::document_search::commands::document_search_run,
            views::plans::pdf_viewer::commands::pdf_viewer_documents,
            views::plans::pdf_viewer::commands::pdf_viewer_get,
            // The view editor of a catalog (views/catalog_editor/).
            views::catalog_editor::catalog_tree::commands::catalog_tree_get,
            views::catalog_editor::object_form::commands::object_form_get,
            views::catalog_editor::object_form::commands::object_form_save_group,
            views::catalog_editor::object_form::commands::object_form_save_control,
            views::catalog_editor::object_form::commands::object_form_delete,
            views::catalog_editor::new_object::commands::new_object_add_group,
            views::catalog_editor::new_object::commands::new_object_add_control,
            views::catalog_editor::object_quotes::commands::object_quotes_get,
            views::catalog_editor::object_quotes::commands::object_quotes_add,
            views::catalog_editor::object_quotes::commands::object_quotes_delete,
            views::catalog_editor::proposals::commands::proposals_get,
            views::catalog_editor::proposals::commands::proposals_accept,
            views::catalog_editor::proposals::commands::proposals_reject,
            views::catalog_editor::ai_builder::commands::ai_builder_sources,
            views::catalog_editor::ai_builder::commands::ai_builder_groups,
            views::catalog_editor::ai_builder::commands::ai_builder_controls,
            views::catalog_editor::ai_builder::commands::ai_builder_stop,
            views::catalog_editor::ai_instructions::commands::ai_instructions_get,
            views::catalog_editor::ai_instructions::commands::ai_instructions_save,
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
