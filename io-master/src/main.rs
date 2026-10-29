//! Entry point of IO Master.
//!
//! IO Master is a desktop program: it is started without parameters
//! and opens a window. All configuration is made in the window.
//!
//! **The program is being rebuilt (user, 2026-10-28)** on the new models
//! (`model/`) and their database (`db/`). The earlier program, with its
//! views, its AI layer and its login, is kept in the branch `main` as a
//! reference; its parts come back here one by one, rewritten on the new
//! models (`docs/handover.md`). Until then the window opens empty.

// The check `ensure!` (rule 5 of CLAUDE.md), declared before the
// folders so that every one of them knows it by its name alone.
#[macro_use]
mod checks;

// ── The map of the program ──────────────────────────────────────────
// Every folder of the program, in the order of its layers from the bottom
// up: a folder uses only the folders above it in this list
// (docs/code_structure.md, section 1; checked by architecture_tests.rs).
// Every module is written here; there is no mod.rs.

// model/: the data of the program as it is kept in the databases, one
// structure per kind of data, fields only (docs/data_model.md).
mod model {
    // model/ai/: the integration with an external AI service and its key,
    // the agents, the knowledge for the AI (RAG), the instructions of the
    // tasks, the chats, the journal of the AI.
    pub mod ai {
        pub mod agent;
        pub mod api_key;
        pub mod chat;
        pub mod instructions;
        pub mod integration;
        pub mod journal;
        pub mod rag;
    }
    // model/evaluation/: the plan of an evaluation, its work units, the
    // standards, the documents of the product and the proofs of the
    // laboratory.
    pub mod evaluation {
        pub mod evaluation_plan;
        pub mod lab_proof;
        pub mod oscal_standard_document;
        pub mod product_document;
        pub mod work_unit;
    }
    // model/user/: the user and the settings of the program.
    pub mod user {
        pub mod program_data;
        pub mod user_data;
        pub mod user_keys;
    }
}

// security/: the building blocks of protection: random numbers from the
// system, and the protection of the memory of the process. The encryption
// of the databases is in db/encryption/.
#[allow(dead_code, reason = "used again when the login is rebuilt (2026-10-28)")]
mod security {
    pub mod process;
    pub mod random;
}

// db/: create, read and update of the models of model/, one file per file
// of model/, no delete (user, 2026-10-28); the two databases (system.v2.db,
// users/<id>.v2.db) and all code of encryption.
#[allow(dead_code, reason = "used by the program again when its views are rebuilt (2026-10-28)")]
mod db {
    pub mod database;
    pub mod error;
    pub mod lists;
    pub mod system_database;
    // db/encryption/: the encryption of the database files: their key and
    // SQLCipher; moved unchanged from the earlier program, audited later.
    pub mod encryption {
        pub mod database_key;
        pub mod sqlcipher;
    }
    // db/ai/: the integrations and their keys, the knowledge (RAG) and its
    // index, the agents, the instructions of the tasks, the chats, the
    // journal of the AI.
    pub mod ai {
        pub mod agent;
        pub mod api_key;
        pub mod chat;
        pub mod instructions;
        pub mod integration;
        pub mod journal;
        pub mod rag;
    }
    // db/user/: the user and the installation (the system database), the
    // settings of the user.
    pub mod user {
        pub mod program_data;
        pub mod user_data;
        pub mod user_keys;
    }
    // db/evaluation/: the plans, their work units, the standards, the
    // documents of the product, the proofs of the laboratory.
    pub mod evaluation {
        pub mod evaluation_plan;
        pub mod lab_proof;
        pub mod oscal_standard_document;
        pub mod product_document;
        pub mod work_unit;
    }
}

// session/: what every command of the window shares: the data folder, and
// the user logged in with their open database (the only global state).
mod session {
    pub mod data_folder;
    pub mod state;
}

// panels/: the panels of the window (user, 2026-10-28), each in two layers
// with the same folders: views/ (only the commands the page calls, and
// their error) and logic/ (the work behind them, not called by the page; it
// gets the open database as a parameter and never reaches the session).
mod panels {
    pub mod logic {
        // panels/logic/evaluation_plan/: the making of a plan: the standards
        // checked against the schema of NIST and read, the plans, their work
        // units, the order of the work and the cross references.
        #[allow(dead_code, reason = "called by the views of the plan, which come next (2026-10-29)")]
        pub mod evaluation_plan {
            pub mod oscal;
        }
        // panels/logic/program_start/: the first start, the login and the
        // recovery of a forgotten password; the password and the recovery
        // code that wrap the key of the database (moved unchanged from the
        // earlier program, audited later).
        #[allow(dead_code, reason = "the recovery and the change of the password come later (2026-10-28)")]
        pub mod program_start {
            pub mod access;
            pub mod generate_key;
            pub mod keys;
            pub mod recovery;
            pub mod recovery_code;
        }
    }
    pub mod views {
        pub mod error;
        // panels/views/program_start/: the first start and the login.
        pub mod program_start {
            pub mod installation;
            pub mod login;
        }
    }
}

// pdf/: reading PDFs with the tools of Poppler (pdfinfo, pdftotext): the only
// place where the program starts another program on a document of the
// evaluator, kept in one folder for the audit. It uses nothing of the program.
#[allow(dead_code, reason = "used again when the documents of the product are read (2026-10-28)")]
mod pdf {
    pub mod poppler;
}

// Help for the tests: temporary folders, a new database with its data.
#[cfg(test)]
mod test_support;

// The direction of the dependencies between the folders, checked on the
// source.
#[cfg(test)]
mod architecture_tests;

use std::process::ExitCode;

use session::data_folder::default_data_folder;
use session::state::Session;
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
fn tauri_startup() -> ExitCode {
    let Some(data_folder) = default_data_folder() else {
        eprintln!("Error: the data folder of IO Master cannot be found: neither XDG_DATA_HOME nor HOME is set.");
        return ExitCode::FAILURE;
    };

    // The decrypted data is only in RAM: the system must never write the
    // memory of the process into a file (security/process.rs).
    if let Err(error) = security::process::forbid_memory_dumps() {
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
            panels::views::program_start::login::login_log_out,
        ])
        .build(tauri::generate_context!());

    let app = match built {
        Ok(app) => app,
        Err(error) => {
            eprintln!("Error: the window could not be opened: {error}");
            return ExitCode::FAILURE;
        }
    };

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
