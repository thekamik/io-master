//! Help for the tests of the views: a window of the mock runtime of
//! Tauri (`tauri::test`), with the commands of the views and the state
//! of a real installation, so that a test calls a command as the page
//! does: by its name, with its arguments as JSON, through the IPC of
//! Tauri.

use std::sync::Arc;

use serde_json::Value;
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponseBody};
use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::{App, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::installation::session::Session;
use crate::tauri_ui::state::UiState;
use crate::tauri_ui::test_support::{TestState, first_start, installed, logged_in};
use crate::test_support::TempDir;

use super::{chat, plans, scan, settings, shell, start};

/// A window, and what must live as long as it: the app and the folder
/// of the installation.
pub struct TestWindow {
    window: WebviewWindow<MockRuntime>,
    app: App<MockRuntime>,
    #[allow(dead_code)]
    directory: TempDir,
}

impl TestWindow {
    /// The session of the logged-in evaluator, to prepare data the way
    /// another view would.
    pub fn session(&self) -> Arc<Session> {
        self.app.state::<UiState>().session().expect("somebody is logged in")
    }

    /// Calls `command` with `arguments` (an object, the names as the page
    /// writes them: `allTypes`), as the page does; the answer of the
    /// command, or the error the page would get.
    pub fn call(&self, command: &str, arguments: Value) -> Result<Value, Value> {
        get_ipc_response(&self.window, request(command, arguments))
            .map(|body| body.deserialize::<Value>().expect("JSON"))
    }

    /// Calls `command` as [`TestWindow::call`] does, for a command that
    /// answers with raw bytes (a PDF).
    pub fn call_bytes(&self, command: &str, arguments: Value) -> Result<Vec<u8>, Value> {
        get_ipc_response(&self.window, request(command, arguments)).map(|body| match body {
            InvokeResponseBody::Raw(bytes) => bytes,
            InvokeResponseBody::Json(text) => text.into_bytes(),
        })
    }
}

/// The request the page sends for `command` with `arguments`.
fn request(command: &str, arguments: Value) -> InvokeRequest {
    InvokeRequest {
        cmd: command.to_owned(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "tauri://localhost".parse().expect("a URL"),
        body: InvokeBody::Json(arguments),
        headers: tauri::http::HeaderMap::default(),
        invoke_key: INVOKE_KEY.to_owned(),
    }
}

/// A window with "anna" logged in.
pub fn logged_in_window() -> TestWindow {
    window(tauri::async_runtime::block_on(logged_in()))
}

/// A window at the first start: nothing is installed.
pub fn first_start_window() -> TestWindow {
    window(first_start())
}

/// A window of an installation where nobody is logged in.
pub fn logged_out_window() -> TestWindow {
    window(installed())
}

/// A window over the state of `test`, with every command of the views.
fn window(test: TestState) -> TestWindow {
    let TestState { directory, state } = test;

    let app = mock_builder()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            start::setup_form::commands::setup_form_install,
            start::login_form::commands::login_form_log_in,
            shell::program::commands::program_get,
            shell::user_menu::commands::user_menu_log_out,
            chat::connection_line::commands::connection_line_get,
            chat::chat_list::commands::chat_list_get,
            chat::conversation::commands::conversation_open,
            chat::conversation::commands::conversation_send,
            chat::conversation::commands::conversation_stop,
            chat::conversation::commands::conversation_delete,
            settings::password_form::commands::password_form_change,
            settings::connection_list::commands::connection_list_get,
            settings::connection_form::commands::connection_form_new,
            settings::connection_form::commands::connection_form_get,
            settings::connection_form::commands::connection_form_save,
            settings::connection_form::commands::connection_form_delete,
            settings::connection_form::commands::connection_form_activate,
            settings::connection_form::commands::connection_form_field_help,
            settings::api_key::commands::api_key_get,
            settings::api_key::commands::api_key_store,
            settings::api_key::commands::api_key_delete,
            settings::connection_check::commands::connection_check_run,
            plans::oscal_documents::commands::oscal_documents_get,
            plans::oscal_documents::commands::oscal_documents_import,
            plans::oscal_documents::commands::oscal_documents_export,
            plans::oscal_documents::commands::oscal_documents_delete,
            plans::new_catalog::commands::new_catalog_sources,
            plans::new_catalog::commands::new_catalog_create,
            plans::standard_documents::commands::standard_documents_get,
            plans::standard_documents::commands::standard_documents_load,
            plans::standard_documents::commands::standard_documents_stop,
            plans::standard_documents::commands::standard_documents_update,
            plans::standard_documents::commands::standard_documents_delete,
            plans::document_search::commands::document_search_run,
            plans::pdf_viewer::commands::pdf_viewer_documents,
            plans::pdf_viewer::commands::pdf_viewer_get,
            scan::ports_table::commands::ports_table_scan,
            scan::scanner_settings::commands::scanner_settings_get,
            scan::scanner_settings::commands::scanner_settings_save,
        ])
        .build(mock_context(noop_assets()))
        .expect("the mock app is built");

    let window = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default()).build().expect("the window opens");

    TestWindow { window, app, directory }
}
