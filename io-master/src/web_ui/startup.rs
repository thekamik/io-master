//! The start of the web interface: everything between the checked
//! installation and the running HTTP server.
//!
//! `main` reads the command line, checks the installation
//! (`installation/verify.rs`), and then hands over to
//! [`web_startup`], which decides which server starts:
//!
//! ```text
//! --address not given   plain HTTP on 127.0.0.1   start_http
//! --address IP          HTTPS, TLS 1.3, on IP     start_tls
//! ```
//!
//! `--address` always means the secure server, also for 127.0.0.1
//! (docs/architecture.md, section 4). The plain HTTP server:
//!
//! 1. opens the listening socket on 127.0.0.1;
//! 2. at the first start, makes the one-time setup code;
//! 3. tells in the terminal where the interface is (and the code);
//! 4. prepares the connection to the AI service;
//! 5. serves the interface until Ctrl+C is pressed ([`ctrl_c`]).
//!
//! The messages in the terminal belong here: they are part of how
//! this interface is started, the only thing the person who
//! started the program sees before the browser is open.

use std::net::IpAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::ai::anthropic::Anthropic;
use crate::installation::lifecycle::Opened;

use super::setup::SetupCode;
use super::web_ui::{self, AppState};

/// Waits until Ctrl+C is pressed.
async fn ctrl_c() {
    // If the signal cannot be watched, the server simply runs
    // until the process is stopped in another way.
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}

/// Starts the web interface for the installation in `data_dir`, in
/// the state `opened`, and serves it until Ctrl+C is pressed.
/// `address` chooses the server: `None` (no `--address`) the plain
/// HTTP server on 127.0.0.1, `Some(ip)` the secure server on `ip`.
/// Returns the exit code of the program; every failure is told in
/// the terminal first.
pub async fn web_startup(port: u16, address: Option<IpAddr>, data_dir: PathBuf, opened: Opened) -> ExitCode {
    match address {
        None => start_http(port, data_dir, opened).await,
        Some(address) => start_tls(address, port, &opened),
    }
}

/// The secure server: HTTPS with TLS 1.3 only, on `address`.
///
/// TODO (step 4 of docs/todo.md): the TLS server, with the
/// certificate and key set in Settings of the local interface and
/// kept in the system database. Until then it refuses to start,
/// with an explanation; it never falls back to plain HTTP, which is
/// not what `--address` asks for.
fn start_tls(address: IpAddr, port: u16, opened: &Opened) -> ExitCode {
    // The first start is made only in the local interface: the TLS
    // server needs settings that do not exist before it.
    if matches!(opened, Opened::FirstRun) {
        eprintln!("Error: IO Master is not installed yet; the first start is made in the local");
        eprintln!("interface: start it without --address.");
        return ExitCode::FAILURE;
    }

    eprintln!("Error: the secure server on {address}:{port} is not configured.");
    eprintln!("--address starts HTTPS with TLS 1.3, whose certificate and key are set");
    eprintln!("in Settings of the local interface (io-master without --address).");
    eprintln!("That setting does not exist yet (step 4 of docs/todo.md).");

    ExitCode::FAILURE
}

/// The plain HTTP server on 127.0.0.1 and `port`, for this computer
/// only.
async fn start_http(port: u16, data_dir: PathBuf, opened: Opened) -> ExitCode {
    let listener = match web_ui::bind(port).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Error: cannot listen on 127.0.0.1:{port}: {error}");
            return ExitCode::FAILURE;
        }
    };

    // The first start: a one-time code that only the person who
    // started the program sees.
    let setup_code = match opened {
        Opened::FirstRun => match SetupCode::generate() {
            Ok(code) => Some(code),
            Err(error) => {
                eprintln!("Error: no setup code could be created: {error}");
                return ExitCode::FAILURE;
            }
        },
        Opened::Installed(_) => None,
    };

    match (&opened, &setup_code) {
        (Opened::Installed(running), _) => {
            println!(
                "IO Master ({}) is running at http://127.0.0.1:{port}/ (press Ctrl+C to stop)",
                running.installation.role.name(),
            );
        }
        (Opened::FirstRun, Some(code)) => {
            println!("IO Master is not installed yet. Open http://127.0.0.1:{port}/");
            println!("and enter this one-time setup code: {}", code.display());
        }
        (Opened::FirstRun, None) => unreachable!("a first start always has a setup code"),
    }

    // The connection to Anthropic; it opens nothing yet, it only
    // prepares TLS with the trusted certificates of the system.
    let ai = match Anthropic::new() {
        Ok(ai) => ai,
        Err(error) => {
            eprintln!("Error: the connection for the AI cannot be prepared: {error}");
            return ExitCode::FAILURE;
        }
    };

    let state = AppState::new(data_dir, opened, setup_code, port, ai);

    match web_ui::run(listener, state, ctrl_c()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}
