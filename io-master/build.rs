//! Build script of IO Master: prepares the desktop interface (Tauri).
//!
//! It reads `tauri.conf.json` and `capabilities/`, checks them, and
//! makes the permission `allow-<command>` for every command below.
//! The window may call a command only when its capability
//! (`capabilities/default.json`) lists that permission, so this
//! list, the handler list in `src/routes.rs` (`generate_handler!`) and the
//! capability must name the same commands (each is defined in the
//! `commands.rs` of its element, `src/views/<view>/<element>/`).

fn main() {
    let commands = tauri_build::AppManifest::new().commands(&[
        "setup_form_install",
        "login_form_log_in",
        "program_get",
        "user_menu_log_out",
        "user_menu_close",
        "password_form_change",
        "connection_list_get",
        "connection_form_new",
        "connection_form_get",
        "connection_form_save",
        "connection_form_delete",
        "connection_form_activate",
        "connection_form_field_help",
        "api_key_get",
        "api_key_store",
        "api_key_delete",
        "connection_check_run",
        "ports_table_scan",
        "scanner_settings_get",
        "scanner_settings_save",
        "connection_line_get",
        "chat_list_get",
        "conversation_open",
        "conversation_send",
        "conversation_stop",
        "conversation_delete",
        "oscal_documents_get",
        "oscal_documents_import",
        "oscal_documents_export",
        "oscal_documents_delete",
        "new_catalog_sources",
        "new_catalog_create",
        "standard_documents_get",
        "standard_documents_load",
        "standard_documents_stop",
        "standard_documents_update",
        "standard_documents_delete",
        "document_search_run",
        "pdf_viewer_documents",
        "pdf_viewer_get",
        "catalog_tree_get",
        "object_form_get",
        "object_form_save_group",
        "object_form_save_control",
        "object_form_delete",
        "new_object_add_group",
        "new_object_add_control",
        "object_quotes_get",
        "object_quotes_add",
        "object_quotes_delete",
        "proposals_get",
        "proposals_accept",
        "proposals_reject",
        "ai_builder_sources",
        "ai_builder_groups",
        "ai_builder_controls",
        "ai_builder_stop",
        "ai_instructions_get",
        "ai_instructions_save",
    ]);

    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(commands))
        .expect("the desktop interface is prepared");
}
