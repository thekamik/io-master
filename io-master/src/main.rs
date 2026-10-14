//! Entry point of IO Master.
//!
//! `main` reads the command line, starts the TPM, finds out the
//! state of the installation, then serves the web user interface
//! until Ctrl+C is pressed.
//!
//! ```text
//! io-master                     http://127.0.0.1:8080, this computer only
//! io-master --port 9000         http://127.0.0.1:9000
//! io-master --address IP        HTTPS with TLS 1.3 on IP (see below)
//! io-master --data-dir DIR      the data of the installation in DIR
//!                               instead of ~/.local/share/io-master
//! io-master --uninstall         remove the installation (asks first)
//! ```
//!
//! At the first start nothing is installed: the program prints
//! a one-time setup code, and the web interface asks for it
//! together with the first user and the role of the installation.
//! A damaged installation does not start; `--uninstall` removes it.
//!
//! `--address` always means the secure server: HTTPS with TLS 1.3,
//! also when the address is 127.0.0.1. Its settings (certificate,
//! private key) are entered beforehand in the Settings of the
//! local interface and kept in the system database.
//!
//! TODO (step 4 of docs/todo.md): the TLS server. Until then
//! `--address` is recognized and refused with an explanation.
//!
//! TODO (step 8 of docs/todo.md): until the beta version the TPM is
//! always the emulator swtpm (tpm/emulator.rs), never the real TPM.

mod installation {
    // The file is installation/installation.rs by choice of the
    // project layout; clippy would prefer a different name for it.
    #[allow(clippy::module_inception)]
    pub mod installation;
    pub mod lifecycle;
}

mod devices {
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod devices;
}

mod tpm {
    pub mod device_secret;
    pub mod emulator;
    #[cfg(test)]
    pub mod test_support;
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod tpm;
}

mod db {
    pub mod database_key;
    pub mod generate_key;
}

mod random {
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod random;
}

mod web_ui {
    pub mod pages;
    pub mod security;
    pub mod session;
    pub mod setup;
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod web_ui;
}

use std::io::BufRead;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use installation::installation::State;
use installation::lifecycle;
use tpm::emulator::{EMULATOR_DIRECTORY, Emulator};
use tpm::tpm::TpmAccess;
use web_ui::setup::SetupCode;
use web_ui::web_ui::AppState;

/// The port of the web interface when `--port` is not given.
const DEFAULT_PORT: u16 = 8080;

/// What has to be typed to confirm `--uninstall`.
const UNINSTALL_CONFIRMATION: &str = "uninstall";

/// How the program was asked to start.
#[derive(Debug, PartialEq, Eq)]
struct Arguments {
    /// `--data-dir DIR`: where the data of the installation is
    /// kept, instead of the usual place; for development and
    /// tests.
    data_dir: Option<PathBuf>,
    /// `--port N`: the port to listen on.
    port: u16,
    /// `--address IP`: start the secure (TLS) server on this
    /// address.
    address: Option<IpAddr>,
    /// `--uninstall`: remove the installation instead of starting.
    uninstall: bool,
}

/// Why the command line could not be understood.
#[derive(Debug, PartialEq, Eq)]
enum ArgumentError {
    /// A flag without the value that must follow it.
    MissingValue(&'static str),
    /// A flag given twice.
    Repeated(&'static str),
    /// A value that the flag does not accept; holds the
    /// explanation.
    InvalidValue(String),
    /// A parameter the program does not know, as typed.
    Unknown(String),
}

/// Reads the parameters (without the program name). Each flag may
/// be given once, in any order.
fn parse_arguments(arguments: &[String]) -> Result<Arguments, ArgumentError> {
    let mut data_dir = None;
    let mut port = None;
    let mut address = None;
    let mut uninstall = false;

    let mut rest = arguments.iter();

    while let Some(flag) = rest.next() {
        // The only flag without a value.
        if flag == "--uninstall" {
            if uninstall {
                return Err(ArgumentError::Repeated("--uninstall"));
            }

            uninstall = true;
            continue;
        }

        // The known flags, each with its value.
        let name: &'static str = match flag.as_str() {
            "--data-dir" => "--data-dir",
            "--port" => "--port",
            "--address" => "--address",
            _ => return Err(ArgumentError::Unknown(flag.clone())),
        };

        let value = rest.next().ok_or(ArgumentError::MissingValue(name))?;

        let repeated = match name {
            "--data-dir" => data_dir.replace(PathBuf::from(value)).is_some(),
            "--port" => port.replace(parse_port(value)?).is_some(),
            _ => address.replace(parse_address(value)?).is_some(),
        };

        if repeated {
            return Err(ArgumentError::Repeated(name));
        }
    }

    Ok(Arguments {
        data_dir,
        port: port.unwrap_or(DEFAULT_PORT),
        address,
        uninstall,
    })
}

/// Reads the value of `--port`: a number from 1 to 65535.
fn parse_port(text: &str) -> Result<u16, ArgumentError> {
    text.parse::<u16>()
        .ok()
        .filter(|&port| port != 0)
        .ok_or_else(|| ArgumentError::InvalidValue(format!("'{text}' is not a port (1 to 65535)")))
}

/// Reads the value of `--address`: an IPv4 or IPv6 address.
fn parse_address(text: &str) -> Result<IpAddr, ArgumentError> {
    text.parse()
        .map_err(|_| ArgumentError::InvalidValue(format!("'{text}' is not an IP address")))
}

/// Waits until Ctrl+C is pressed.
async fn ctrl_c() {
    // If the signal cannot be watched, the server simply runs
    // until the process is stopped in another way.
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}

/// `--uninstall`: removes the installation after the user typed
/// the confirmation. Removes the marker from the TPM, the sealed
/// secret, the record, and the state of the TPM emulator.
fn uninstall(data_dir: &Path) -> ExitCode {
    println!("This removes the installation of IO Master in {}", data_dir.display());
    println!("and its marker in the TPM. Its data cannot be opened afterwards.");
    println!("Type '{UNINSTALL_CONFIRMATION}' to continue:");

    let mut answer = String::new();

    if std::io::stdin().lock().read_line(&mut answer).is_err() || answer.trim() != UNINSTALL_CONFIRMATION {
        println!("Nothing was removed.");
        return ExitCode::FAILURE;
    }

    let emulator = match Emulator::start(data_dir) {
        Ok(emulator) => emulator,
        Err(error) => {
            eprintln!("Error: {error}");
            return ExitCode::FAILURE;
        }
    };

    let removed = TpmAccess { port: emulator.port() }
        .connect()
        .map_err(lifecycle::LifecycleError::Tpm)
        .and_then(|mut tpm| lifecycle::uninstall(data_dir, &mut tpm));

    // The emulator is stopped before its state is removed.
    drop(emulator);

    if let Err(error) = removed {
        eprintln!("Error: {error}");
        return ExitCode::FAILURE;
    }

    // TODO (step 8 of docs/todo.md): with the real TPM there is no
    // emulator state to remove.
    let emulator_state = data_dir.join(EMULATOR_DIRECTORY);

    if let Err(error) = std::fs::remove_dir_all(&emulator_state)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        eprintln!("Error: {}: {error}", emulator_state.display());
        return ExitCode::FAILURE;
    }

    println!("The installation was removed.");

    ExitCode::SUCCESS
}

#[tokio::main]
async fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    let arguments = match parse_arguments(&arguments) {
        Ok(arguments) => arguments,
        Err(error) => {
            match error {
                ArgumentError::MissingValue(flag) => eprintln!("'{flag}' needs a value"),
                ArgumentError::Repeated(flag) => eprintln!("'{flag}' is given more than once"),
                ArgumentError::InvalidValue(explanation) => eprintln!("{explanation}"),
                ArgumentError::Unknown(text) => eprintln!("Unknown parameter: '{text}'"),
            }
            eprintln!("Usage: io-master [--port N] [--address IP] [--data-dir DIR] [--uninstall]");

            // 2 is the usual exit code for a wrong command line.
            return ExitCode::from(2);
        }
    };

    let data_dir = match arguments.data_dir {
        Some(directory) => directory,
        None => match installation::installation::default_data_dir() {
            Ok(directory) => directory,
            Err(error) => {
                eprintln!("Error: {error}");
                return ExitCode::FAILURE;
            }
        },
    };

    if arguments.uninstall {
        return uninstall(&data_dir);
    }

    // The first start is made only in the local interface: the
    // TLS server needs settings that do not exist before it.
    // TODO (step 4 of docs/todo.md): start the TLS server with the
    // settings from the system database.
    if let Some(address) = arguments.address {
        eprintln!("Error: the secure server on {address} is not configured.");
        eprintln!("--address starts HTTPS with TLS 1.3, whose certificate and key are set");
        eprintln!("in Settings of the local interface (io-master without --address).");
        eprintln!("That setting does not exist yet (step 4 of docs/todo.md).");
        return ExitCode::FAILURE;
    }

    // The TPM, until the beta version always the emulator. It runs
    // as long as this variable lives: until the server ends.
    let emulator = match Emulator::start(&data_dir) {
        Ok(emulator) => emulator,
        Err(error) => {
            eprintln!("Error: {error}");
            return ExitCode::FAILURE;
        }
    };

    let tpm = TpmAccess { port: emulator.port() };

    let state = match tpm
        .connect()
        .map_err(lifecycle::LifecycleError::Tpm)
        .and_then(|mut connection| lifecycle::open(&data_dir, &mut connection))
    {
        Ok(state) => state,
        Err(error) => {
            eprintln!("Error: {error}");
            eprintln!("IO Master does not start on a damaged installation.");
            eprintln!("To remove it and start again: io-master --uninstall");
            return ExitCode::FAILURE;
        }
    };

    let listener = match web_ui::web_ui::bind(arguments.port).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Error: cannot listen on 127.0.0.1:{}: {error}", arguments.port);
            return ExitCode::FAILURE;
        }
    };

    // The first start: a one-time code that only the person who
    // started the program sees.
    let setup_code = match state {
        State::FirstRun => match SetupCode::generate() {
            Ok(code) => Some(code),
            Err(error) => {
                eprintln!("Error: no setup code could be created: {error}");
                return ExitCode::FAILURE;
            }
        },
        State::Installed(_) => None,
    };

    match (&state, &setup_code) {
        (State::Installed(installation), _) => {
            println!(
                "IO Master ({}) is running at http://127.0.0.1:{}/ (press Ctrl+C to stop)",
                installation.role.name(),
                arguments.port
            );
        }
        (State::FirstRun, Some(code)) => {
            println!("IO Master is not installed yet. Open http://127.0.0.1:{}/", arguments.port);
            println!("and enter this one-time setup code: {}", code.display());
        }
        (State::FirstRun, None) => unreachable!("a first start always has a setup code"),
    }

    eprintln!("Development version: the TPM is the emulator swtpm, not the TPM of this computer.");

    let state = AppState::new(data_dir, state, setup_code, arguments.port, tpm);

    let result = web_ui::web_ui::run(listener, state, ctrl_c()).await;

    // Stop the TPM after the server.
    drop(emulator);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
