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
        "app_state",
        "set_up",
        "log_in",
        "log_out",
        "change_password",
        "ai_config",
        "save_ai_connection",
        "delete_ai_connection",
        "activate_ai_connection",
        "store_ai_key",
        "delete_ai_key",
        "ai_field_instruction",
        "check_ai_connection",
        "scan_ports",
        "save_scan_settings",
        "list_chats",
        "open_chat",
        "delete_chat",
        "send_message",
        "stop_reply",
        "list_documents",
        "load_documents",
        "stop_work",
        "update_document",
        "delete_document",
        "document_page",
        "search_documents",
        "document_pdf",
    ]);

    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(commands))
        .expect("the desktop interface is prepared");
}
