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
mod business;
mod db;
mod hardware_tests;
mod installation;
mod model;
mod security;
mod tauri_ui;
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
        // The commands the page may call: those of the views, each in
        // the folder of its element (views/<view>/<element>/commands.rs),
        // and those not moved yet, thin functions in
        // tauri_ui/commands.rs that hand over to the file named in
        // their group. A command must also be listed in build.rs and
        // capabilities/default.json, or the page is refused when it
        // calls it.
        .invoke_handler(tauri::generate_handler![
            // The view Start (views/start/) and the frame of the window
            // (views/shell/).
            views::start::setup_form::commands::setup_form_install,
            views::start::login_form::commands::login_form_log_in,
            views::shell::program::commands::program_get,
            views::shell::user_menu::commands::user_menu_log_out,
            // The password (commands/account.rs).
            commands::change_password,
            // Settings, tab AI: the connections to AI services
            // (commands/ai.rs).
            commands::ai_config,
            commands::save_ai_connection,
            commands::delete_ai_connection,
            commands::activate_ai_connection,
            commands::store_ai_key,
            commands::delete_ai_key,
            commands::check_ai_connection,
            commands::ai_field_instruction,
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
            // The evaluation plan: the documents of the standards
            // (commands/documents.rs).
            commands::list_documents,
            commands::load_documents,
            commands::stop_work,
            commands::update_document,
            commands::delete_document,
            commands::document_page,
            commands::search_documents,
            commands::document_pdf,
            // The evaluation plan: the documents of OSCAL
            // (commands/oscal.rs).
            commands::list_oscal_documents,
            commands::import_oscal_document,
            commands::export_oscal_document,
            commands::delete_oscal_document,
            // The evaluation plan: the catalogs of OSCAL
            // (commands/catalogs.rs).
            commands::create_catalog,
            commands::open_catalog,
            commands::add_catalog_group,
            commands::update_catalog_group,
            commands::add_catalog_control,
            commands::update_catalog_control,
            commands::delete_catalog_object,
            commands::add_catalog_quote,
            commands::delete_catalog_quote,
            commands::accept_catalog_proposal,
            commands::reject_catalog_proposal,
            commands::catalog_instructions,
            commands::save_catalog_instructions,
            commands::propose_catalog_groups,
            commands::propose_catalog_controls,
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
