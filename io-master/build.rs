//! Build script of IO Master: prepares the desktop interface (Tauri).
//!
//! It reads `tauri.conf.json` and `capabilities/`, checks them, and
//! makes the permission `allow-<command>` for every command below. The
//! window may call a command only when its capability
//! (`capabilities/default.json`) lists that permission.
//!
//! **No command yet** (2026-10-28): the program is being rebuilt on the
//! new models, and the commands come back with the views.

fn main() {
    let commands = tauri_build::AppManifest::new().commands(&[]);

    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(commands))
        .expect("the desktop interface is prepared");
}
