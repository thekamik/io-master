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
        "integration_list",
        "integration_create",
        "integration_save",
        "integration_activate",
        "integration_set_key",
        "integration_check",
        "instructions_get",
        "instructions_save",
        "journal_list",
        "journal_get",
    ]);

    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(commands))
        .expect("the desktop interface is prepared");
}
