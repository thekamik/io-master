//! Entry point of IO Master.
//!
//! IO Master is a desktop program: it is started without parameters
//! and opens a window. All configuration is made in the window.
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
//! one user of the installation. One Linux account holds one
//! installation with one user (docs/architecture.md, section 3).

// The check `ensure!` (rule 5 of CLAUDE.md), declared before the
// folders so that every one of them knows it by its name alone.
#[macro_use]
mod checks;

// ── The map of the program ──────────────────────────────────────────
// Every folder of the program, in the order of its layers from the bottom
// up: a folder uses only the folders above it in this list
// (docs/code_structure.md, section 1; checked by architecture_tests.rs).
// The comment of a group says which part of the project it is; what each
// file holds is said at its functions. Since steps 45 and 46 of
// docs/history.md every module is written here, except those of a file with code
// that has modules of its own (model_old/oscal.rs), which declares them itself.

// model/: the new models (2026-10-28): the data of the program as it is kept
// in the databases, built model by model; the code is moved to them from
// model_old/ step by step, so that the program works all the time.
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
    }
}

// model_old/: the earlier models: the data of the program as plain Rust
// types and the rules of their validity; no I/O (no database, no network,
// no file). Replaced by model/ step by step.
mod model_old {
    pub mod ai;
    pub mod api_key;
    pub mod chats;
    pub mod conversion;
    pub mod evaluation;
    pub mod evaluation_project;
    pub mod evaluation_support;
    pub mod installation;
    pub mod knowledge;
    pub mod oscal;
    pub mod own_requirement;
    pub mod recovery_code;
    pub mod standards;
    pub mod task_scheme;
    pub mod text;
    pub mod time;
    pub mod work_package;
}

// security/: the building blocks of protection: random numbers from the
// system, and the protection of the memory of the process. The encryption
// of the databases is in db/encryption/.
mod security {
    pub mod process;
    pub mod random;
}

// db/: the new operations on the databases (2026-10-28): create, read and
// update of the models of model/, one file per file of model/, no delete.
mod db {
    pub mod database;
    pub mod error;
    pub mod lists;
    // db/encryption/: all code of encryption of the databases, moved from
    // db_old/ unchanged (2026-10-28); serves db/ and db_old/.
    pub mod encryption {
        pub mod database_key;
        pub mod generate_key;
        pub mod keys;
        pub mod recovery;
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

// db_old/: the earlier operations on the databases, replaced by db/ step by
// step (2026-10-28). Every operation on a database, as CRUD, one folder per area of data
// (CLAUDE.md, "Database code"). The schema.rs of an area says in which
// database its data lives and how it is protected.
mod db_old {
    pub mod error;
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
    // db/conversion/: the conversions of the documents of standards into
    // catalogs of OSCAL, step by step (the database of the evaluator).
    pub mod conversion {
        pub mod create;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
    // db/evaluation/: the evaluations of the Evaluation lab, what the
    // evaluator adds to them (the database of the evaluator).
    pub mod evaluation {
        pub mod create;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
    // db/knowledge/: the knowledge bases of the AI, their sources and the
    // index of their passages (the database of the evaluator).
    pub mod knowledge {
        pub mod create;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
    // db/evaluation_project/: the master projects of the evaluations and their
    // tasks (the database of the evaluator).
    pub mod evaluation_project {
        pub mod create;
        pub mod delete;
        pub mod read;
        pub mod rows;
        pub mod schema;
        pub mod update;
    }
    // db/own_requirements/: which catalog is the shared catalog of the
    // requirements of the laboratory (a setting of the evaluator; no table).
    pub mod own_requirements {
        pub mod delete;
        pub mod read;
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
        pub mod assessment;
        pub mod candidates;
        pub mod catalog_contents;
        pub mod catalog_outline;
        pub mod catalog_section;
        pub mod chat;
        pub mod evidence_fragments;
        pub mod evidence_text;
        pub mod input_chat;
        mod json;
        pub mod knowledge_answer;
        pub mod knowledge_context;
        pub mod output_draft;
        pub mod related_elements;
        pub mod requirement_choice;
        pub mod requirement_context;
        pub mod task_scheme;
    }
    // ai/presets/: the ready settings of well-known AI services (its file has
    // the presets themselves, so it declares its own module).
    pub mod presets;
}

// pdf/: reading PDFs with the tools of Poppler (pdfinfo, pdftotext): the only
// place where the program starts another program on a document of the
// evaluator, kept in one folder for the audit. It uses nothing of the program.
mod pdf {
    pub mod poppler;
}

// installation/: the installation of this Linux account, its one user and
// the session of that user: the core of the protection of the program.
mod installation {
    pub mod accounts;
    pub mod lifecycle;
    pub mod permissions;
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

// components/: the elements of the window used more than once, several times
// in one view or in several views (docs/architecture.md, section 10.2). A
// component is built as an element of a view; one with commands has an entry,
// the file of its name. The views use the components; a component never uses
// a view (checked by architecture_tests.rs).
mod components {
    // components/ai_work/: one question of a work of the AI with its events
    // (the progress, a failure, the end); the works of the AI of the views
    // Evaluation plan and Evaluation lab. No command of its own.
    pub mod ai_work {
        pub mod ask;
        pub mod view;
    }
    // components/knowledge_agent/: the passages an agent with knowledge gives
    // the model with a question (step 57.3); the proving ground of the view
    // Knowledge (RAG), later the works of the AI of the other views. No
    // command of its own.
    pub mod knowledge_agent {
        #[allow(clippy::module_inception)]
        pub mod knowledge_agent;

        pub mod find;
        pub mod view;
    }
    // components/part_rows/: the parts of a group or a control as rows with
    // their level; the object chosen and the proposals of the editor of a
    // catalog. No command: the elements give its rows with their data.
    // components/catalog_tree/: the tree of a catalog, its groups and
    // controls as rows with their level; the editor of a catalog (Converter)
    // and the browser of the standards (Evaluation plan).
    pub mod catalog_tree {
        #[allow(clippy::module_inception)]
        pub mod catalog_tree;

        pub mod view;
    }
    pub mod part_rows {
        pub mod view;
    }
    // components/recovery_code/: the recovery code, shown once after the
    // setup, a change or a recovery of the password, and a new code (step 49
    // of docs/history.md). No command: the commands of the view Users give it.
    pub mod recovery_code {
        pub mod view;
    }
    // components/pdf_viewer/: the viewer of a standard, a quote highlighted on
    // its page; the evaluation plan and the editor of a catalog.
    pub mod pdf_viewer {
        #[allow(clippy::module_inception)]
        pub mod pdf_viewer;

        pub mod view;
    }
}

// views/: the views of the window, one folder per view. The entry of a view
// is the file of its name in its folder: every command the page calls for
// it (step 46 of docs/history.md); the folders of its elements around it hold
// their data and logic. The entry has the name of its folder, so its
// module has the name of its parent: clippy's `module_inception` is
// allowed there, on purpose.
mod views {
    // views/users/: the view Users: the account of the evaluator, its session
    // and the encryption of its database in one place: the setup, the login,
    // the password, the logout.
    pub mod users {
        #[allow(clippy::module_inception)]
        pub mod users;
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

        pub mod ai_journal {
            pub mod view;
        }
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

    // views/evaluation_lab/: the view Evaluation lab: a work package of the
    // Evaluation plan imported and carried out for one product (step 52).
    pub mod evaluation_lab {
        #[allow(clippy::module_inception)]
        pub mod evaluation_lab;

        #[cfg(test)]
        pub mod test_support;

        pub mod assessment {
            pub mod context;
            pub mod propose;
        }
        pub mod evaluation_list {
            pub mod view;
        }
        pub mod evidence {
            pub mod add;
            pub mod dialog;
        }
        pub mod fragments {
            pub mod propose;
            pub mod view;
        }
        pub mod input_chat {
            pub mod chat;
        }
        pub mod lab_task {
            pub mod read;
            pub mod view;
        }
        pub mod lab_tasks {
            pub mod view;
        }
        pub mod outputs {
            pub mod draft;
        }
        pub mod package_import {
            pub mod dialog;
            pub mod read;
            pub mod store;
            pub mod view;
        }
        pub mod report {
            pub mod dialog;
            pub mod export;
        }
    }

    // views/knowledge/: the view Knowledge (RAG): the knowledge bases of the
    // AI, their sources and the search of their passages (step 57).
    pub mod knowledge {
        #[allow(clippy::module_inception)]
        pub mod knowledge;

        pub mod agent_form {
            pub mod dialog;
            pub mod export;
            pub mod view;
        }
        pub mod agent_list {
            pub mod dialog;
            pub mod import;
            pub mod view;
        }
        pub mod knowledge_base {
            pub mod add;
            pub mod dialog;
            pub mod view;
        }
        pub mod knowledge_list {
            pub mod view;
        }
        pub mod knowledge_search {
            pub mod search;
            pub mod view;
        }
        pub mod proving_ground {
            pub mod ask;
            pub mod view;
        }
    }

    // views/evaluation_plan/: the view Evaluation plan: it works only on the
    // documents of OSCAL (the list, the import, the export, the deletion;
    // later the document shown graphically and turned into the flow of data
    // of the evaluation).
    pub mod evaluation_plan {
        #[allow(clippy::module_inception)]
        pub mod evaluation_plan;

        pub mod new_project {
            pub mod catalogs;
            pub mod view;
        }
        pub mod oscal_documents {
            pub mod dialog;
            pub mod file;
            pub mod view;
        }
        pub mod own_requirement {
            pub mod catalog;
            pub mod view;
        }
        pub mod project_form {
            pub mod view;
        }
        pub mod project_list {
            pub mod view;
        }
        pub mod standard_browser {
            pub mod requirement;
            pub mod search;
            pub mod view;
        }
        pub mod standard_outputs {
            pub mod view;
        }
        pub mod task_links {
            pub mod propose;
            pub mod view;
        }
        pub mod task_list {
            pub mod view;
        }
        pub mod task_proposals {
            pub mod propose;
            pub mod view;
        }
        pub mod task_scheme {
            pub mod propose;
            pub mod scheme;
            pub mod view;
        }
        pub mod work_packages {
            pub mod export;
            pub mod view;
        }
    }

    // views/converter/: the view Converter: the documents of a standard (PDF)
    // converted into a catalog of OSCAL with the help of the AI.
    pub mod converter {
        #[allow(clippy::module_inception)]
        pub mod converter;

        pub mod catalog_list {
            pub mod view;
        }
        pub mod document_search {
            pub mod search;
            pub mod view;
        }
        pub mod new_catalog {
            pub mod made;
            pub mod view;
        }
        pub mod standard_documents {
            pub mod dialog;
            pub mod load;
            pub mod stored;
            pub mod view;
        }

        // views/converter/catalog_editor/: the editor of a catalog, a
        // screen of the view, with an entry of its own.
        pub mod catalog_editor {
            #[allow(clippy::module_inception)]
            pub mod catalog_editor;

            #[cfg(test)]
            pub mod test_support;
            pub mod ai_builder {
                pub mod controls;
                pub mod groups;
                pub mod parts;
                pub mod steps;
                pub mod view;
            }
            pub mod conversion_wizard {
                pub mod contents;
                pub mod locate;
                pub mod progress;
                pub mod view;
            }
            pub mod new_object {
                pub mod view;
            }
            pub mod object_form {
                pub mod oscal_object;
                pub mod view;
            }
            pub mod object_quotes {
                pub mod checked_quote;
                pub mod view;
            }
            pub mod proposals {
                pub mod accept;
                pub mod view;
            }
        }
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
use tauri::{Manager, RunEvent};

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
    // The decrypted data is only in RAM: the system must never write the
    // memory of the process into a file (security/process.rs).
    if let Err(error) = security::process::forbid_memory_dumps() {
        eprintln!("Error: the memory dumps of the program could not be forbidden: {error}");
        return ExitCode::FAILURE;
    }

    let built = tauri::Builder::default()
        // The dialogs of the system, used only from Rust (commands.rs).
        .plugin(tauri_plugin_dialog::init())
        // The state shared by all commands: the installation, the
        // session, the work in progress (app/state.rs).
        .manage(AppState::new(data_dir, opened))
        // The commands the page may call (routes.rs).
        .invoke_handler(routes::handler())
        .build(tauri::generate_context!());

    let app = match built {
        Ok(app) => app,
        Err(error) => {
            eprintln!("Error: the window could not be opened: {error}");
            return ExitCode::FAILURE;
        }
    };

    // However the program ends (the cross of the window, "Log out and
    // close", the end of the session of the desktop), the session ends the
    // way "Log out" ends it (AppState::log_out): the work stopped, the
    // database closed, its key gone from memory. The closure is the one
    // Tauri requires for its events (a library place, rule 9 of CLAUDE.md).
    let code = app.run_return(|handle, event| {
        if let RunEvent::Exit = event {
            handle.state::<AppState>().log_out();
        }
    });

    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
