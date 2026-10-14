//! Entry point of IO Master.
//!
//! `main` reads the command line and the configuration of the
//! installation, then starts the web user interface and serves it
//! until Ctrl+C is pressed.
//!
//! ```text
//! io-master                     http://127.0.0.1:8080, this computer only
//! io-master --port 9000         http://127.0.0.1:9000
//! io-master --address IP        HTTPS with TLS 1.3 on IP (see below)
//! io-master --config PATH       the configuration of the installation
//!                               from PATH instead of the usual place
//! ```
//!
//! `--address` always means the secure server: HTTPS with TLS 1.3,
//! also when the address is 127.0.0.1. Its settings (certificate,
//! private key) are entered beforehand in the Settings of the
//! local interface and kept in the system database.
//!
//! TODO (step 5a of docs/todo.md): the TLS server. Until then
//! `--address` is recognized and refused with an explanation.

mod config {
    // The file is config/config.rs by choice of the project
    // layout; clippy would prefer a different name for it.
    #[allow(clippy::module_inception)]
    pub mod config;
}

mod devices {
    // Same layout as config/config.rs.
    #[allow(clippy::module_inception)]
    pub mod devices;
}

mod keyring {
    // Same layout as config/config.rs.
    #[allow(clippy::module_inception)]
    pub mod keyring;

    pub mod database_key;
}

mod db {
    pub mod generate_key;
}

mod random {
    // Same layout as config/config.rs.
    #[allow(clippy::module_inception)]
    pub mod random;
}

mod web_ui {
    pub mod pages;
    pub mod security;
    pub mod session;
    // Same layout as config/config.rs.
    #[allow(clippy::module_inception)]
    pub mod web_ui;
}

use std::net::IpAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use config::config::ConfigError;

/// The port of the web interface when `--port` is not given.
const DEFAULT_PORT: u16 = 8080;

/// An example configuration, shown when none is found.
const EXAMPLE_CONFIG: &str = "role = \"workstation\"   # or \"master\"\n\
                              single_host = false     # optional";

/// How the program was asked to start.
#[derive(Debug, PartialEq, Eq)]
struct Arguments {
    /// `--config PATH`: the configuration file to read instead of
    /// the usual one.
    config: Option<PathBuf>,
    /// `--port N`: the port to listen on.
    port: u16,
    /// `--address IP`: start the secure (TLS) server on this
    /// address.
    address: Option<IpAddr>,
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
    let mut config = None;
    let mut port = None;
    let mut address = None;

    let mut rest = arguments.iter();

    while let Some(flag) = rest.next() {
        // The known flags, each with its value.
        let name: &'static str = match flag.as_str() {
            "--config" => "--config",
            "--port" => "--port",
            "--address" => "--address",
            _ => return Err(ArgumentError::Unknown(flag.clone())),
        };

        let value = rest.next().ok_or(ArgumentError::MissingValue(name))?;

        let repeated = match name {
            "--config" => config.replace(PathBuf::from(value)).is_some(),
            "--port" => port.replace(parse_port(value)?).is_some(),
            _ => address.replace(parse_address(value)?).is_some(),
        };

        if repeated {
            return Err(ArgumentError::Repeated(name));
        }
    }

    Ok(Arguments {
        config,
        port: port.unwrap_or(DEFAULT_PORT),
        address,
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
            eprintln!("Usage: io-master [--port N] [--address IP] [--config PATH]");

            // 2 is the usual exit code for a wrong command line.
            return ExitCode::from(2);
        }
    };

    let path = match arguments.config {
        Some(path) => Ok(path),
        None => config::config::default_path(),
    };

    let config = match path.and_then(|path| config::config::load(&path)) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Error: {error}");

            if matches!(error, ConfigError::NotFound(_)) {
                eprintln!("\nThe installation is not finished. Create the file with, for example:\n");
                eprintln!("{EXAMPLE_CONFIG}");
            }

            return ExitCode::FAILURE;
        }
    };

    // TODO (step 5a of docs/todo.md): start the TLS server with
    // the settings from the system database.
    if let Some(address) = arguments.address {
        eprintln!("Error: the secure server on {address} is not configured.");
        eprintln!("--address starts HTTPS with TLS 1.3, whose certificate and key are set");
        eprintln!("in Settings of the local interface (io-master without --address).");
        eprintln!("That setting does not exist yet (step 5a of docs/todo.md).");
        return ExitCode::FAILURE;
    }

    let listener = match web_ui::web_ui::bind(arguments.port).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Error: cannot listen on 127.0.0.1:{}: {error}", arguments.port);
            return ExitCode::FAILURE;
        }
    };

    if config.single_host {
        eprintln!("WARNING: single-host mode. Master and workstation run on one computer;");
        eprintln!("         whoever takes over this computer gets every secret.");
    }

    println!(
        "IO Master ({}) is running at http://127.0.0.1:{}/ (press Ctrl+C to stop)",
        config.role.name(),
        arguments.port
    );

    match web_ui::web_ui::run(listener, config, ctrl_c()).await {
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
