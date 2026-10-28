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
    ]);

    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(commands))
        .expect("the desktop interface is prepared");
}
