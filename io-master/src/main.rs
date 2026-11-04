//! Entry point of IO Master.
//!
//! IO Master is a desktop program: it is started without parameters
//! and opens a window. All configuration is made in the window.
//!
//! It is built on the models of `objects/`, each with its store, its rules
//! and its AI (`docs/architecture.md`, `docs/code_structure.md`).

// The check `ensure!` (rule 5 of CLAUDE.md), declared before the
// folders so that every one of them knows it by its name alone.
#[macro_use]
mod checks;

// The three groups of the program, the dependencies one way
// (docs/code_structure.md): panels/ ──► objects/ ──► infrastructure/. Each
// folder declares its modules in the file of the same name beside it
// (objects.rs for objects/); there is no mod.rs.
mod infrastructure;
mod objects;
mod panels;

// Help for the tests: temporary folders, a new database with its data.
#[cfg(test)]
mod test_support;

use std::process::ExitCode;

use infrastructure::data_folder::default_data_folder;
use panels::session::Session;
use tauri::{Manager, RunEvent};

/// Starts the program: refuses parameters, then opens the window.
fn main() -> ExitCode {
    // The program takes no parameters. One given anyway (a typing
    // mistake) is refused rather than passed over, so that nobody believes
    // it had an effect.
    if let Some(parameter) = std::env::args().nth(1) {
        eprintln!("Error: IO Master takes no parameters (given: '{parameter}').");
        eprintln!("Everything is set in its window.");

        // 2 is the usual exit code for a wrong command line.
        return ExitCode::from(2);
    }

    tauri_startup()
}

/// Opens the window and runs until it is closed. Returns the exit code of
/// the program; every failure is told in the terminal.
///
/// The page is built into the program (`ui/dist`, made with
/// `npm run build`); it is never loaded from the network, and no network
/// port is opened. It may call only the commands listed in
/// `tauri::generate_handler!` below, in `build.rs` and in
/// `capabilities/default.json`; the three lists name the same commands.
///
/// Called by: [`main`].
/// Calls: [`run_window`].
fn tauri_startup() -> ExitCode {
    let Some(data_folder) = default_data_folder() else {
        eprintln!("Error: the data folder of IO Master cannot be found: neither XDG_DATA_HOME nor HOME is set.");
        return ExitCode::FAILURE;
    };

    // The decrypted data is only in RAM: the system must never write the
    // memory of the process into a file (infrastructure/security/process.rs).
    if let Err(error) = infrastructure::security::process::forbid_memory_dumps() {
        eprintln!("Error: the memory dumps of the program could not be forbidden: {error}");
        return ExitCode::FAILURE;
    }

    let built = tauri::Builder::default()
        // The dialogs of the system, used only from Rust.
        .plugin(tauri_plugin_dialog::init())
        // What every command shares: the data folder, the user logged in.
        .manage(Session::new(data_folder))
        // The commands the page may call (panels/views/).
        .invoke_handler(tauri::generate_handler![
            panels::views::program_start::installation::installation_create,
            panels::views::program_start::login::login_get,
            panels::views::program_start::login::login_log_in,
            panels::views::program_start::login::login_recover,
            panels::views::program_start::login::login_log_out,
            panels::views::account::credentials::account_change_password,
            panels::views::account::credentials::account_new_recovery_code,
            panels::views::evaluation_plan::standards::standard_list,
            panels::views::evaluation_plan::standards::standard_import,
            panels::views::evaluation_plan::plans::plan_list,
            panels::views::evaluation_plan::plans::plan_create,
            panels::views::evaluation_plan::plans::plan_get,
            panels::views::evaluation_plan::plans::plan_save,
            panels::views::evaluation_plan::plans::plan_add_document,
            panels::views::evaluation_plan::work_units::work_unit_list,
            panels::views::evaluation_plan::work_units::work_unit_controls,
            panels::views::evaluation_plan::work_units::work_unit_add,
            panels::views::evaluation_plan::work_units::work_unit_add_all,
            panels::views::evaluation_plan::work_units::work_unit_remove,
            panels::views::evaluation_plan::work_units::work_unit_get,
            panels::views::evaluation_plan::work_units::work_unit_save,
            panels::views::evaluation_plan::work_units::work_unit_add_proof,
            panels::views::evaluation_plan::fill::plan_fill_preview,
            panels::views::evaluation_plan::fill::plan_fill_parameters,
            panels::views::evaluation_plan::fill::plan_fill_dependencies,
            panels::views::evaluation_plan::fill::plan_fill_progress,
            panels::views::evaluation_plan::fill::plan_fill_stop,
            panels::views::evaluation::evaluations::evaluation_list,
            panels::views::evaluation::evaluations::evaluation_create,
            panels::views::evaluation::evaluations::evaluation_get,
            panels::views::evaluation::evaluations::evaluation_set_agent,
            panels::views::evaluation::inputs::evaluation_document_give,
            panels::views::evaluation::inputs::evaluation_proof_give,
            panels::views::evaluation::run::evaluation_assess_preview,
            panels::views::evaluation::run::evaluation_assess,
            panels::views::evaluation::run::evaluation_assess_progress,
            panels::views::evaluation::run::evaluation_assess_stop,
            panels::views::evaluation::work_units::evaluation_unit_list,
            panels::views::evaluation::work_units::evaluation_unit_get,
            panels::views::evaluation::work_units::evaluation_unit_save,
            panels::views::settings::integrations::integration_list,
            panels::views::settings::integrations::integration_form,
            panels::views::settings::integrations::integration_create,
            panels::views::settings::integrations::integration_save,
            panels::views::settings::integrations::integration_activate,
            panels::views::settings::integrations::integration_set_key,
            panels::views::settings::integrations::integration_check,
            panels::views::agents::agent::agent_list,
            panels::views::agents::agent::agent_create,
            panels::views::agents::agent::agent_get,
            panels::views::agents::agent::agent_save,
            panels::views::agents::agent::agent_knowledge_test,
            panels::views::agents::knowledge::knowledge_base_list,
            panels::views::agents::knowledge::knowledge_base_create,
            panels::views::agents::knowledge::knowledge_base_save,
            panels::views::agents::knowledge::knowledge_source_list,
            panels::views::agents::knowledge::knowledge_source_add,
            panels::views::agents::knowledge::knowledge_source_replace,
            panels::views::agents::knowledge::knowledge_source_rename,
            panels::views::agents::knowledge::knowledge_search,
            panels::views::agents::journal::agent_journal_list,
            panels::views::agents::journal::agent_journal_get,
            panels::views::chat::chats::chat_list,
            panels::views::chat::chats::chat_create,
            panels::views::chat::chats::chat_get,
            panels::views::chat::chats::chat_rename,
            panels::views::chat::chats::chat_ask,
            panels::views::chat::chats::chat_stop,
            panels::views::reports::report::report_list,
            panels::views::reports::report::report_preview,
            panels::views::reports::report::report_export,
            panels::views::reports::templates::report_template_list,
            panels::views::reports::templates::report_template_create,
            panels::views::reports::templates::report_template_get,
            panels::views::reports::templates::report_template_save,
            panels::views::reports::templates::report_template_import,
            panels::views::reports::templates::report_template_export,
            panels::views::reports::templates::report_markers,
        ])
        .build(tauri::generate_context!());

    let app = match built {
        Ok(app) => app,
        Err(error) => {
            eprintln!("Error: the window could not be opened: {error}");
            return ExitCode::FAILURE;
        }
    };

    run_window(app)
}

/// Runs the window `app` until it is closed and gives the exit code of the
/// program.
///
/// Called by: [`tauri_startup`].
fn run_window(app: tauri::App) -> ExitCode {
    // However the program ends (the cross of the window, the end of the
    // session of the desktop), the user is logged out: their database is
    // closed. The closure is the one Tauri requires for its events (a
    // library place, rule 9 of CLAUDE.md).
    let code = app.run_return(|handle, event| {
        if let RunEvent::Exit = event
            && handle.state::<Session>().log_out().is_err()
        {
            eprintln!("Error: the session could not be ended cleanly.");
        }
    });

    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
