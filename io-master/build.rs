//! Build script of IO Master: prepares the desktop interface (Tauri).
//!
//! It reads `tauri.conf.json` and `capabilities/`, checks them, and
//! makes the permission `allow-<command>` for every command below.
//! The window may call a command only when its capability
//! (`capabilities/default.json`) lists that permission, so this
//! list, the handler list in `src/main.rs` (`tauri_startup`) and the
//! capability must name the same commands (`src/tauri_ui/commands.rs`).

fn main() {
    let commands = tauri_build::AppManifest::new().commands(&[
        "setup_form_install",
        "login_form_log_in",
        "program_get",
        "user_menu_log_out",
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
        "stop_work",
        "open_catalog",
        "add_catalog_group",
        "update_catalog_group",
        "add_catalog_control",
        "update_catalog_control",
        "delete_catalog_object",
        "add_catalog_quote",
        "delete_catalog_quote",
        "accept_catalog_proposal",
        "reject_catalog_proposal",
        "catalog_instructions",
        "save_catalog_instructions",
        "propose_catalog_groups",
        "propose_catalog_controls",
    ]);

    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(commands))
        .expect("the desktop interface is prepared");
}
