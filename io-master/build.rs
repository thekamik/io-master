//! Build script of IO Master: prepares the desktop interface (Tauri).
//!
//! It reads `tauri.conf.json` and `capabilities/`, checks them, and
//! makes the permission `allow-<command>` for every command below. The
//! window may call a command only when its capability
//! (`capabilities/default.json`) lists that permission.
//!
//! The list names the same commands as `tauri::generate_handler!` in
//! `src/main.rs` and the capability; each is defined in its file of
//! `src/panels/views/`.

fn main() {
    let commands = tauri_build::AppManifest::new().commands(&[
        "installation_create",
        "login_get",
        "login_log_in",
        "login_log_out",
        "standard_list",
        "standard_import",
        "plan_list",
        "plan_create",
        "plan_get",
        "plan_save",
        "plan_add_document",
        "work_unit_list",
        "work_unit_controls",
        "work_unit_add",
        "work_unit_add_all",
        "work_unit_remove",
        "work_unit_get",
        "work_unit_save",
        "work_unit_add_proof",
        "plan_fill_preview",
        "plan_fill_parameters",
        "plan_fill_dependencies",
        "plan_fill_stop",
        "evaluation_list",
        "evaluation_create",
        "evaluation_get",
        "evaluation_set_agent",
        "evaluation_document_give",
        "evaluation_proof_give",
        "evaluation_assess_preview",
        "evaluation_assess",
        "evaluation_assess_stop",
        "evaluation_unit_list",
        "evaluation_unit_get",
        "evaluation_unit_save",
        "integration_list",
        "integration_create",
        "integration_save",
        "integration_activate",
        "integration_set_key",
        "integration_check",
        "agent_list",
        "agent_create",
        "agent_get",
        "agent_save",
        "agent_knowledge_test",
        "knowledge_base_list",
        "knowledge_base_create",
        "knowledge_base_save",
        "knowledge_source_list",
        "knowledge_source_add",
        "knowledge_source_replace",
        "knowledge_source_rename",
        "knowledge_search",
        "agent_journal_list",
        "agent_journal_get",
        "chat_list",
        "chat_create",
        "chat_get",
        "chat_rename",
        "chat_ask",
        "chat_stop",
    ]);

    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(commands))
        .expect("the desktop interface is prepared");
}
