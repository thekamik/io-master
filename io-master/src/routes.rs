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
        crate::components::catalog_tree::catalog_tree::catalog_tree_get,
        crate::components::pdf_viewer::pdf_viewer::pdf_viewer_documents,
        crate::components::pdf_viewer::pdf_viewer::pdf_viewer_get,
        // The view Users (views/users/): the account, the session and the
        // encryption of its database.
        crate::views::users::users::setup_form_install,
        crate::views::users::users::login_form_log_in,
        crate::views::users::users::login_form_recover,
        crate::views::users::users::password_form_change,
        crate::views::users::users::recovery_code_form_replace,
        crate::views::users::users::user_menu_log_out,
        crate::views::users::users::user_menu_close,
        // The frame of the window (views/shell/).
        crate::views::shell::shell::program_get,
        // The view Settings (views/settings/): the settings of the program.
        crate::views::settings::settings::connection_list_get,
        crate::views::settings::settings::connection_form_new,
        crate::views::settings::settings::connection_form_get,
        crate::views::settings::settings::connection_form_save,
        crate::views::settings::settings::connection_form_delete,
        crate::views::settings::settings::connection_form_activate,
        crate::views::settings::settings::connection_form_field_help,
        crate::views::settings::settings::api_key_get,
        crate::views::settings::settings::api_key_store,
        crate::views::settings::settings::api_key_delete,
        crate::views::settings::settings::connection_check_run,
        // The view Chat (views/chat/).
        crate::views::chat::chat::connection_line_get,
        crate::views::chat::chat::chat_list_get,
        crate::views::chat::chat::conversation_open,
        crate::views::chat::chat::conversation_send,
        crate::views::chat::chat::conversation_stop,
        crate::views::chat::chat::conversation_delete,
        // The view Evaluation plan (views/evaluation_plan/).
        crate::views::evaluation_plan::evaluation_plan::project_list_get,
        crate::views::evaluation_plan::evaluation_plan::project_form_get,
        crate::views::evaluation_plan::evaluation_plan::task_list_get,
        crate::views::evaluation_plan::evaluation_plan::new_project_catalogs,
        crate::views::evaluation_plan::evaluation_plan::new_project_create,
        crate::views::evaluation_plan::evaluation_plan::project_list_delete,
        crate::views::evaluation_plan::evaluation_plan::project_form_save,
        crate::views::evaluation_plan::evaluation_plan::task_list_add,
        crate::views::evaluation_plan::evaluation_plan::task_list_remove,
        crate::views::evaluation_plan::evaluation_plan::task_list_move,
        crate::views::evaluation_plan::evaluation_plan::standard_browser_get,
        crate::views::evaluation_plan::evaluation_plan::standard_browser_search,
        crate::views::evaluation_plan::evaluation_plan::own_requirement_add,
        crate::views::evaluation_plan::evaluation_plan::own_requirement_save,
        crate::views::evaluation_plan::evaluation_plan::own_requirement_delete,
        crate::views::evaluation_plan::evaluation_plan::standard_outputs_get,
        crate::views::evaluation_plan::evaluation_plan::standard_outputs_add,
        crate::views::evaluation_plan::evaluation_plan::standard_outputs_save,
        crate::views::evaluation_plan::evaluation_plan::standard_outputs_delete,
        crate::views::evaluation_plan::evaluation_plan::task_proposals_get,
        crate::views::evaluation_plan::evaluation_plan::task_proposals_propose,
        crate::views::evaluation_plan::evaluation_plan::task_proposals_accept,
        crate::views::evaluation_plan::evaluation_plan::task_proposals_reject,
        crate::views::evaluation_plan::evaluation_plan::task_proposals_accept_all,
        crate::views::evaluation_plan::evaluation_plan::task_proposals_reject_all,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_get,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_add_block,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_save_block,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_move_block,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_remove_block,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_connect,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_disconnect,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_propose,
        crate::views::evaluation_plan::evaluation_plan::ai_work_stop,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_accept,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_reject,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_accept_all,
        crate::views::evaluation_plan::evaluation_plan::task_scheme_reject_all,
        crate::views::evaluation_plan::evaluation_plan::ai_work_instructions_get,
        crate::views::evaluation_plan::evaluation_plan::ai_work_instructions_save,
        crate::views::evaluation_plan::evaluation_plan::oscal_documents_get,
        crate::views::evaluation_plan::evaluation_plan::oscal_documents_import,
        crate::views::evaluation_plan::evaluation_plan::oscal_documents_export,
        crate::views::evaluation_plan::evaluation_plan::oscal_documents_delete,
        // The view Converter (views/converter/).
        crate::views::converter::converter::catalog_list_get,
        crate::views::converter::converter::new_catalog_sources,
        crate::views::converter::converter::new_catalog_create,
        crate::views::converter::converter::standard_documents_get,
        crate::views::converter::converter::standard_documents_load,
        crate::views::converter::converter::standard_documents_stop,
        crate::views::converter::converter::standard_documents_update,
        crate::views::converter::converter::standard_documents_delete,
        crate::views::converter::converter::document_search_run,
        // The editor of a catalog, a part of the view Converter
        // (views/converter/catalog_editor/).
        crate::views::converter::catalog_editor::catalog_editor::conversion_wizard_get,
        crate::views::converter::catalog_editor::catalog_editor::conversion_wizard_find_contents,
        crate::views::converter::catalog_editor::catalog_editor::conversion_wizard_contents,
        crate::views::converter::catalog_editor::catalog_editor::conversion_wizard_stage,
        crate::views::converter::catalog_editor::catalog_editor::conversion_wizard_group_pages,
        crate::views::converter::catalog_editor::catalog_editor::conversion_wizard_group_state,
        crate::views::converter::catalog_editor::catalog_editor::object_form_get,
        crate::views::converter::catalog_editor::catalog_editor::object_form_save_group,
        crate::views::converter::catalog_editor::catalog_editor::object_form_save_control,
        crate::views::converter::catalog_editor::catalog_editor::object_form_delete,
        crate::views::converter::catalog_editor::catalog_editor::new_object_add_group,
        crate::views::converter::catalog_editor::catalog_editor::new_object_add_control,
        crate::views::converter::catalog_editor::catalog_editor::object_quotes_get,
        crate::views::converter::catalog_editor::catalog_editor::object_quotes_add,
        crate::views::converter::catalog_editor::catalog_editor::object_quotes_delete,
        crate::views::converter::catalog_editor::catalog_editor::proposals_get,
        crate::views::converter::catalog_editor::catalog_editor::proposals_accept,
        crate::views::converter::catalog_editor::catalog_editor::proposals_reject,
        crate::views::converter::catalog_editor::catalog_editor::ai_builder_sources,
        crate::views::converter::catalog_editor::catalog_editor::ai_builder_groups,
        crate::views::converter::catalog_editor::catalog_editor::ai_builder_controls,
        crate::views::converter::catalog_editor::catalog_editor::ai_builder_stop,
        crate::views::converter::catalog_editor::catalog_editor::ai_instructions_get,
        crate::views::converter::catalog_editor::catalog_editor::ai_instructions_save,
    ]
}
