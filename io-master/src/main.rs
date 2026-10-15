//! Entry point of IO Master.
//!
//! `main` has three tasks, and does each by calling the module that
//! owns it:
//!
//! 1. reads the command line ([`parse_arguments`], in this file);
//! 2. checks the installation of this Linux account
//!    (`installation/verify.rs`); a damaged installation is
//!    a critical error and the program does not start;
//! 3. starts the web interface (`web_ui/startup.rs`), which serves
//!    until Ctrl+C is pressed.
//!
//! The command line chooses only how the server is reached; all
//! other configuration is made in the web interface:
//!
//! ```text
//! io-master                     http://127.0.0.1:8080, this computer only
//! io-master --port 9000         http://127.0.0.1:9000
//! io-master --address IP        HTTPS with TLS 1.3 on IP
//! ```
//!
//! `--address` always means the secure server, also when the
//! address is 127.0.0.1. Its certificate and key are set beforehand
//! in Settings of the local interface. TODO (step 4 of
//! docs/todo.md): the TLS server; until then `--address` is refused
//! with an explanation (web_ui/startup.rs).
//!
//! At the first start nothing is installed: the program prints a
//! one-time setup code in the terminal, and the web interface asks
//! for it together with the user and the role of the installation.
//!
//! TODO (step 13 of docs/todo.md): the location of the installation
//! and the secret of the application come from the keyring of the
//! Linux account; one Linux account holds one installation with one
//! user (docs/architecture.md, sections 3 and 5.6).

mod ai {
    pub mod ai_error;
    pub mod ai_settings;
    pub mod anthropic;
    pub mod api_key;
    pub mod chats;
    pub mod model;
    pub mod reply;
    pub mod sse;
    #[cfg(test)]
    pub mod test_support;
}

mod accounts {
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod accounts;
}

mod installation {
    // The file is installation/installation.rs by choice of the
    // project layout; clippy would prefer a different name for it.
    #[allow(clippy::module_inception)]
    pub mod installation;
    pub mod lifecycle;
    pub mod verify;
}

mod devices {
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod devices;
    pub mod port_property;
    pub mod scan_settings;
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
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod db;
    pub mod generate_key;
    pub mod keys;
    pub mod system_db;
    pub mod user_db;
}

mod random {
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod random;
}

mod web_ui {
    pub mod chat_page;
    pub mod pages;
    pub mod scan_page;
    pub mod security;
    pub mod session;
    pub mod settings;
    pub mod setup;
    pub mod startup;
    #[cfg(test)]
    pub mod test_support;
    // Same layout as installation/installation.rs.
    #[allow(clippy::module_inception)]
    pub mod web_ui;
}

use std::net::IpAddr;
use std::process::ExitCode;

use installation::lifecycle::Opened;
use installation::verify::{InstallationState, verify_installation};

/// The port of the web interface when `--port` is not given.
const DEFAULT_PORT: u16 = 8080;

// The two entries of the installation in the keyring of the Linux
// account (docs/architecture.md, section 5.6). These constants are
// the *names* under which the entries are stored, not their
// values: the location and the secret are read from the keyring
// at every start and never written into the program.
//
// TODO (step 13 of docs/todo.md): move both to
// installation/verify.rs, where the keyring will be read; unused
// until then.

/// Name of the keyring entry with the location of the installation
/// (the folder that holds its databases).
const KEYRING_DATA_DIR_ENTRY: &str = "io-master.data-dir";

/// Name of the keyring entry with the secret of the application
/// (32 random bytes, also the recovery code).
const KEYRING_APP_SECRET_ENTRY: &str = "io-master.app-secret";

/// How the program was asked to start.
#[derive(Debug, PartialEq, Eq)]
struct Arguments {
    /// `--port N`: the port to listen on; [`DEFAULT_PORT`] when the
    /// flag is not given (no flag and `--port 8080` mean the same).
    port: u16,
    /// `--address IP`: on which address the secure server runs.
    ///
    /// `Option` holds one of two values:
    ///
    /// - `None`: the flag was not given; the program serves plain
    ///   HTTP on 127.0.0.1 only, for this computer.
    /// - `Some(ip)`: the flag was given with a valid IPv4 or IPv6
    ///   address; the program starts the secure server (HTTPS,
    ///   TLS 1.3) on `ip`, also when `ip` is 127.0.0.1.
    ///
    /// A default address would not do: "no flag" and
    /// `--address 127.0.0.1` mean different servers, and only
    /// `None` tells them apart. The text is checked when the
    /// command line is read, so an `IpAddr` here is always a valid
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
// ok kk
fn parse_arguments(arguments: &[String]) -> Result<Arguments, ArgumentError> {
    let mut port = None;
    let mut address = None;

    // Goes through the parameters typed by the user, from left to
    // right. `rest` is one shared position in the list: each round
    // of the loop takes the next flag from it, and a flag with a
    // value takes its value from it too (`rest.next()` below), so
    // that the next round starts after that value. The loop ends
    // when the list is used up (`rest.next()` gives `None`).
    let mut rest = arguments.iter();

    while let Some(flag) = rest.next() {

        // The known flags, each with its value.
        let name: &'static str = match flag.as_str() {
            "--port" => "--port",
            "--address" => "--address",
            _ => return Err(ArgumentError::Unknown(flag.clone())),
        };

        // The value that must follow the flag. Two cases mean that
        // the value was left out, and both give `MissingValue`:
        //
        // - the list ends after the flag (`--port`);
        // - the next parameter is a flag itself, recognized by its
        //   leading "-" (`--address --port 8080`). No valid value
        //   starts with "-": a port is a positive number, and an
        //   IPv4 or IPv6 address starts with a digit, a letter a-f
        //   or ":".
        //
        // Only this is checked here; whether the value is a valid
        // port or address is checked by parse_port and
        // parse_address below.
        let value = rest
            .next()
            .filter(|value| !value.starts_with('-'))
            .ok_or(ArgumentError::MissingValue(name))?;

        let repeated = match name {
            "--port" => port.replace(parse_port(value)?).is_some(),
            _ => address.replace(parse_address(value)?).is_some(),
        };

        if repeated {
            return Err(ArgumentError::Repeated(name));
        }
    }

    Ok(Arguments {
        port: port.unwrap_or(DEFAULT_PORT),
        address,
    })
}

/// Reads the value of `--port`: a number from 1 to 65535.
// ok kk
fn parse_port(text: &str) -> Result<u16, ArgumentError> {
    text.parse::<u16>()
        .ok()
        .filter(|&port| port != 0)
        .ok_or_else(|| ArgumentError::InvalidValue(format!("'{text}' is not a port (1 to 65535)")))
}

/// Reads the value of `--address`: an IPv4 or IPv6 address.
// ok kk
fn parse_address(text: &str) -> Result<IpAddr, ArgumentError> {
    text.parse()
        .map_err(|_| ArgumentError::InvalidValue(format!("'{text}' is not an IP address")))
}

//ok kk
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
            eprintln!("Usage: io-master [--port N] [--address IP]");

            // 2 is the usual exit code for a wrong command line.
            return ExitCode::from(2);
        }
    };

    // The state of the installation decides how the server starts
    // (installation/verify.rs). A damaged installation is a
    // critical error: the program does not start
    // (docs/architecture.md, section 3.2).
    let (data_dir, opened) = match verify_installation() {
        InstallationState::NotInstalled { data_dir } => (data_dir, Opened::FirstRun),
        InstallationState::Installed { data_dir, running } => (data_dir, Opened::Installed(running)),
        InstallationState::Damaged(damage) => {
            eprintln!("Error: {damage}.");
            eprintln!("IO Master does not start on a damaged installation.");
            return ExitCode::FAILURE;
        }
    };

    // Everything else is the start of the web interface
    // (web_ui/startup.rs).
    web_ui::startup::web_startup(arguments.port, arguments.address, data_dir, opened).await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
