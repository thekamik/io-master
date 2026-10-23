//! The routes of the window: the list of every command the page may
//! call, as `urls.py` of Django lists the views (step 43 of
//! docs/todo.md). It is above `views/`, which it names, and below
//! `main.rs`, which gives it to the window; the tests of the commands use
//! the same list (`views/test_support.rs`), so there is one list only.
//!
//! A command is callable only when it is listed in four places: the
//! `commands.rs` of its element (`views/<view>/<element>/`), this file,
//! `build.rs` and `capabilities/default.json`; `command_lists_tests.rs`
//! checks that they agree.

use tauri::Runtime;
use tauri::ipc::Invoke;

/// The table of the commands, for the window of the program (`main.rs`)
/// and for the mock window of the tests. It is the closure that Tauri
/// requires and `generate_handler!` makes (a library place, rule 9 of
/// CLAUDE.md): it finds a command by the name the page calls.
pub fn handler<R: Runtime>() -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        // The components (components/), used by several views.
        crate::components::pdf_viewer::commands::pdf_viewer_documents,
        crate::components::pdf_viewer::commands::pdf_viewer_get,
        // The view Start (views/start/) and the frame of the window
        // (views/shell/).
        crate::views::start::setup_form::commands::setup_form_install,
        crate::views::start::login_form::commands::login_form_log_in,
        crate::views::shell::program::commands::program_get,
        crate::views::shell::user_menu::commands::user_menu_log_out,
        // The view Settings (views/settings/).
        crate::views::settings::password_form::commands::password_form_change,
        crate::views::settings::connection_list::commands::connection_list_get,
        crate::views::settings::connection_form::commands::connection_form_new,
        crate::views::settings::connection_form::commands::connection_form_get,
        crate::views::settings::connection_form::commands::connection_form_save,
        crate::views::settings::connection_form::commands::connection_form_delete,
        crate::views::settings::connection_form::commands::connection_form_activate,
        crate::views::settings::connection_form::commands::connection_form_field_help,
        crate::views::settings::api_key::commands::api_key_get,
        crate::views::settings::api_key::commands::api_key_store,
        crate::views::settings::api_key::commands::api_key_delete,
        crate::views::settings::connection_check::commands::connection_check_run,
        // The view Devices (views/devices/).
        crate::views::devices::ports_table::commands::ports_table_scan,
        crate::views::devices::scanner_settings::commands::scanner_settings_get,
        crate::views::devices::scanner_settings::commands::scanner_settings_save,
        // The view Chat (views/chat/).
        crate::views::chat::connection_line::commands::connection_line_get,
        crate::views::chat::chat_list::commands::chat_list_get,
        crate::views::chat::conversation::commands::conversation_open,
        crate::views::chat::conversation::commands::conversation_send,
        crate::views::chat::conversation::commands::conversation_stop,
        crate::views::chat::conversation::commands::conversation_delete,
        // The view Evaluation plan (views/evaluation_plan/).
        crate::views::evaluation_plan::oscal_documents::commands::oscal_documents_get,
        crate::views::evaluation_plan::oscal_documents::commands::oscal_documents_import,
        crate::views::evaluation_plan::oscal_documents::commands::oscal_documents_export,
        crate::views::evaluation_plan::oscal_documents::commands::oscal_documents_delete,
        crate::views::evaluation_plan::new_catalog::commands::new_catalog_sources,
        crate::views::evaluation_plan::new_catalog::commands::new_catalog_create,
        crate::views::evaluation_plan::standard_documents::commands::standard_documents_get,
        crate::views::evaluation_plan::standard_documents::commands::standard_documents_load,
        crate::views::evaluation_plan::standard_documents::commands::standard_documents_stop,
        crate::views::evaluation_plan::standard_documents::commands::standard_documents_update,
        crate::views::evaluation_plan::standard_documents::commands::standard_documents_delete,
        crate::views::evaluation_plan::document_search::commands::document_search_run,
        // The editor of a catalog, a part of the view Evaluation plan
        // (views/evaluation_plan/catalog_editor/).
        crate::views::evaluation_plan::catalog_editor::catalog_tree::commands::catalog_tree_get,
        crate::views::evaluation_plan::catalog_editor::object_form::commands::object_form_get,
        crate::views::evaluation_plan::catalog_editor::object_form::commands::object_form_save_group,
        crate::views::evaluation_plan::catalog_editor::object_form::commands::object_form_save_control,
        crate::views::evaluation_plan::catalog_editor::object_form::commands::object_form_delete,
        crate::views::evaluation_plan::catalog_editor::new_object::commands::new_object_add_group,
        crate::views::evaluation_plan::catalog_editor::new_object::commands::new_object_add_control,
        crate::views::evaluation_plan::catalog_editor::object_quotes::commands::object_quotes_get,
        crate::views::evaluation_plan::catalog_editor::object_quotes::commands::object_quotes_add,
        crate::views::evaluation_plan::catalog_editor::object_quotes::commands::object_quotes_delete,
        crate::views::evaluation_plan::catalog_editor::proposals::commands::proposals_get,
        crate::views::evaluation_plan::catalog_editor::proposals::commands::proposals_accept,
        crate::views::evaluation_plan::catalog_editor::proposals::commands::proposals_reject,
        crate::views::evaluation_plan::catalog_editor::ai_builder::commands::ai_builder_sources,
        crate::views::evaluation_plan::catalog_editor::ai_builder::commands::ai_builder_groups,
        crate::views::evaluation_plan::catalog_editor::ai_builder::commands::ai_builder_controls,
        crate::views::evaluation_plan::catalog_editor::ai_builder::commands::ai_builder_stop,
        crate::views::evaluation_plan::catalog_editor::ai_instructions::commands::ai_instructions_get,
        crate::views::evaluation_plan::catalog_editor::ai_instructions::commands::ai_instructions_save,
    ]
}
