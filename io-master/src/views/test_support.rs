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

use crate::app::state::AppState;
use crate::app::test_support::{TestState, first_start, installed, logged_in};
use crate::installation::session::Session;
use crate::test_support::TempDir;

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
        self.app.state::<AppState>().session().expect("somebody is logged in")
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

/// A window over the state of `test`, with every command of the views
/// (`routes.rs`, the list of the program itself).
fn window(test: TestState) -> TestWindow {
    let TestState { directory, state } = test;

    let app = mock_builder()
        .manage(state)
        .invoke_handler(crate::routes::handler())
        .build(mock_context(noop_assets()))
        .expect("the mock app is built");

    let window = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default()).build().expect("the window opens");

    TestWindow { window, app, directory }
}
