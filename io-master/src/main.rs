//! Entry point of IO Master.
//!
//! IO Master is a desktop program: it is started without parameters
//! and opens a window. All configuration is made in the window.
//!
//! **The program is being rebuilt (user, 2026-10-28)** on the new models
//! (`objects/`, each with its store, its rules and its AI). The earlier program, with its
//! views, its AI layer and its login, is kept at the commit `0797e85` as a
//! reference; its parts come back here one by one, rewritten on the new
//! models (`docs/handover.md`).

// The check `ensure!` (rule 5 of CLAUDE.md), declared before the
// folders so that every one of them knows it by its name alone.
#[macro_use]
mod checks;

// ── The map of the program ──────────────────────────────────────────
// Every folder of the program, in the order of its layers from the bottom
// up: a folder uses only the folders above it in this list
// (docs/code_structure.md, section 1; checked by architecture_tests.rs).
// Every module is written here; there is no mod.rs.

// infrastructure/: what belongs to no object and no panel, and knows no
// object (user, 2026-11-01): the database files and their encryption, random
// bytes and the protection of the process, PDFs read with Poppler, the clock,
// the data folder.
mod infrastructure {
    // The time of the computer, as every time written into the databases
    // is read.
    pub mod clock;
    // Where the data of the program is: $XDG_DATA_HOME/io-master.
    pub mod data_folder;
    // infrastructure/pdf/: reading PDFs with the tools of Poppler (pdfinfo,
    // pdftotext): the only place where the program starts another program on
    // a document of the evaluator, kept in one folder for the audit.
    pub mod pdf {
        pub mod poppler;
    }
    // infrastructure/security/: random numbers from the system, and the
    // protection of the memory of the process.
    pub mod security {
        pub mod process;
        pub mod random;
    }
    // infrastructure/storage/: the two database files (system.v2.db,
    // users/<id>.v2.db): where they are, made, opened, their tables made from
    // the list given by their caller; the help for the lists of a model; their
    // encryption; DbError, the error of every function on a database.
    pub mod storage {
        pub mod connection;
        pub mod error;
        pub mod lists;
        pub mod system_database;
        pub mod user_database;
        // infrastructure/storage/encryption/: the key of the database files
        // and SQLCipher; moved unchanged from the earlier program, audited
        // later.
        pub mod encryption {
            pub mod database_key;
            pub mod generate_key;
            pub mod sqlcipher;
        }
    }
}

// objects/: every model with the functions that work on it, a folder per
// object, its files named by the object and their role (user, 2026-11-01):
// <object>_model.rs (the structure), <object>_store.rs (create, read,
// update), <object>_rules.rs (the rules of the data), <object>_ai.rs (the
// work of the AI); a big role becomes a folder of the same name.
mod objects {
    // AiError, the error of every work of the AI.
    pub mod ai_error;
    // DomainError, the error of the rules of the data.
    pub mod error;
    // The one rule of a text the user gives: trimmed, within its limit.
    pub mod text;
    // The tables of each database, given to infrastructure/storage/ when a
    // database is opened.
    pub mod schema;
    // objects/ai/: the integrations with the AI services and their keys, the
    // agents and their knowledge, the chats, the journal of every question of
    // an agent.
    pub mod ai {
        // The agent: a layer of knowledge over the model of an integration;
        // the knowledge it gives with a question, and the question it asks
        // with it.
        pub mod agent {
            pub mod agent_ai;
            pub mod agent_model;
            pub mod agent_rules;
            pub mod agent_store;
        }
        // The API key of an integration: kept apart, masked in every text
        // taken from an answer.
        pub mod api_key {
            pub mod api_key_ai;
            pub mod api_key_model;
            pub mod api_key_rules;
            pub mod api_key_store;
        }
        // The chat of the user with an agent: its rules, its question
        // shortened from its oldest messages until it fits, its answer.
        pub mod chat {
            pub mod chat_ai;
            pub mod chat_model;
            pub mod chat_rules;
            pub mod chat_store;
        }
        // The integration, the service that answers: the question prepared
        // and sent, the address checked, the connection over TLS 1.3, the
        // body and the stream of each protocol, chosen in one place
        // (protocol.rs).
        pub mod integration {
            pub mod integration_model;
            pub mod integration_store;
            // What an integration must be: the address and the CA
            // certificate the program sends to, the integrations as the
            // settings of the AI change them.
            pub mod integration_rules {
                pub mod address;
                pub mod settings;
            }
            pub mod integration_ai {
                pub mod anthropic;
                pub mod check;
                pub mod connection;
                pub mod openai;
                pub mod protocol;
                pub mod question;
                pub mod reading;
                pub mod sending;
                pub mod sse;
                pub mod turns;
            }
        }
        // The journal of the AI: the answer written into the journal of its
        // agent before it is given out (record).
        pub mod journal {
            pub mod journal_ai;
            pub mod journal_model;
            pub mod journal_store;
        }
        // The knowledge (RAG): the bases and their sources, cut into
        // passages and indexed (FTS5), searched before a question.
        pub mod rag {
            pub mod rag_model;
            pub mod rag_rules;
            pub mod rag_store;
        }
    }
    // objects/evaluation/: the plan of an evaluation and its work units, the
    // standards, the documents of the product and the proofs of the
    // laboratory.
    pub mod evaluation {
        // The plan of an evaluation: its life and its data, the plan in
        // evaluation (a copy for one product), the order of its work, its work
        // units as its children; the loops of the AI over them (the assessment
        // of an evaluation, "Fill with AI").
        pub mod evaluation_plan {
            pub mod evaluation_plan_ai;
            pub mod evaluation_plan_model;
            pub mod evaluation_plan_store;
            pub mod evaluation_plan_rules {
                pub mod evaluation;
                pub mod order;
                pub mod planning;
                pub mod work_units;
            }
        }
        // A proof of the laboratory: the record of a test, given once.
        pub mod lab_proof {
            pub mod lab_proof_model;
            pub mod lab_proof_rules;
            pub mod lab_proof_store;
        }
        // A standard as an OSCAL catalog: imported, checked against the schema
        // of NIST, its controls read.
        pub mod oscal_standard_document {
            pub mod oscal_standard_document_model;
            pub mod oscal_standard_document_rules;
            pub mod oscal_standard_document_store;
        }
        // A document of the product: its file given once, its text cut into
        // passages and indexed.
        pub mod product_document {
            pub mod product_document_model;
            pub mod product_document_rules;
            pub mod product_document_store;
        }
        // A work unit: the parameters the planner gives it, the results of the
        // evaluator and the check of a quote; the work of the AI on it (its
        // assessment, its filling).
        pub mod work_unit {
            pub mod work_unit_model;
            pub mod work_unit_store;
            pub mod work_unit_rules {
                pub mod parameters;
                pub mod results;
            }
            pub mod work_unit_ai {
                pub mod assessment;
                pub mod assessment_answer;
                pub mod assessment_question;
                pub mod filling;
                pub mod proposal;
            }
        }
    }
    // objects/session/: helper models, never kept in a database (user,
    // 2026-11-01).
    pub mod session {
        // The progress of the long work of the AI: a step written into it; the
        // locks of the session as a long work takes them.
        pub mod progress {
            pub mod progress_model;
            pub mod progress_ai {
                pub mod lock;
                pub mod step;
            }
        }
    }
    // objects/user/: the user, the installation and the settings, the keys of
    // the user; the first start and the login.
    pub mod user {
        // The installation (the system database) and the settings of the user.
        pub mod program_data {
            pub mod program_data_model;
            pub mod program_data_store;
        }
        // The user; their account made at the first start, the login.
        pub mod user_data {
            pub mod user_data_model;
            pub mod user_data_rules;
            pub mod user_data_store;
        }
        // The two copies of the key of the database of the user, wrapped with
        // the password and with the recovery code (moved unchanged from the
        // earlier program, audited later).
        pub mod user_keys {
            pub mod user_keys_model;
            pub mod user_keys_store;
            pub mod user_keys_rules {
                pub mod password;
                pub mod recovery;
                pub mod recovery_code;
            }
        }
    }
}

// panels/: the panels of the window (user, 2026-10-28), each in two layers
// with the same folders: views/ (only the commands the page calls, and
// their error) and logic/ (the work behind them, not called by the page; it
// gets the open database as a parameter and never reaches the session).
mod panels {
    // What every command of the window shares: the data folder, and the user
    // logged in with their open database (the only global state).
    pub mod session;
    pub mod logic {
        // panels/logic/evaluation/: the evaluations: a plan copied for one
        // product with its agent, the documents and proofs given, the
        // results of the evaluator (the assessment by the AI is in
        // objects/evaluation/).
        pub mod evaluation {
            pub mod error;
            pub mod inputs;
        }
    }
    pub mod views {
        pub mod error;
        pub mod opened;
        pub mod progress;
        // panels/views/agents/: the agents, the knowledge bases and their
        // sources, the journal of an agent.
        pub mod agents {
            pub mod agent;
            pub mod journal;
            pub mod knowledge;
        }
        // panels/views/chat/: the chats with an agent, the answer streamed
        // into the page, its stop.
        pub mod chat {
            pub mod chats;
        }
        // panels/views/evaluation/: the evaluations, their agent, the
        // documents and proofs given, the assessment by the AI, the work
        // units and the results of the evaluator.
        pub mod evaluation {
            pub mod evaluations;
            pub mod inputs;
            pub mod run;
            pub mod work_units;
        }
        // panels/views/evaluation_plan/: the standards, the plans in
        // planning, their work units.
        pub mod evaluation_plan {
            pub mod fill;
            pub mod plans;
            pub mod standards;
            pub mod work_units;
        }
        // panels/views/program_start/: the first start and the login.
        pub mod program_start {
            pub mod installation;
            pub mod login;
        }
        // panels/views/settings/: the integrations, their keys and the check
        // of the connection.
        pub mod settings {
            pub mod integrations;
        }
    }
}

// Help for the tests: temporary folders, a new database with its data.
#[cfg(test)]
mod test_support;

// The direction of the dependencies between the folders, checked on the
// source.
#[cfg(test)]
mod architecture_tests;

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
            panels::views::program_start::login::login_log_out,
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
