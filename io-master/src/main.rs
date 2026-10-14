//! Entry point of IO Master.
//!
//! `main` only reads the command line and starts the front end
//! that was asked for. Parameters are added step by step; the
//! ones that exist today:
//!
//! ```text
//! io-master --cli     start the console user interface
//! ```
//!
//! The web user interface, which is meant to drive the workflow
//! in the future, will get its own parameter.

mod devices {
    // The file is devices/devices.rs by choice of the project
    // layout; clippy would prefer a different name for it.
    #[allow(clippy::module_inception)]
    pub mod devices;
}

mod keyring {
    // Same layout as devices/devices.rs.
    #[allow(clippy::module_inception)]
    pub mod keyring;

    pub mod database_key;
}

mod db {
    pub mod generate_key;
}

mod cli_ui {
    pub mod console;
    pub mod port_table;
}

use std::process::ExitCode;

/// What the command line asks the program to do.
#[derive(Debug, PartialEq, Eq)]
enum Mode {
    /// `--cli`: the console user interface.
    Console,
}

/// Why the command line could not be understood.
#[derive(Debug, PartialEq, Eq)]
enum ArgumentError {
    /// No parameter was given.
    Missing,
    /// A parameter the program does not know, as typed.
    Unknown(String),
    /// More parameters than the program accepts; holds the first
    /// one that is too many.
    Unexpected(String),
}

/// Turns the parameters (without the program name) into a mode.
///
/// Exactly one parameter is accepted for now.
fn parse_arguments(arguments: &[String]) -> Result<Mode, ArgumentError> {
    let mode = match arguments.first().map(String::as_str) {
        None => return Err(ArgumentError::Missing),
        Some("--cli") => Mode::Console,
        Some(other) => return Err(ArgumentError::Unknown(other.to_owned())),
    };

    if let Some(extra) = arguments.get(1) {
        return Err(ArgumentError::Unexpected(extra.clone()));
    }

    Ok(mode)
}

/// Prints how the program is started.
fn print_usage(program: &str) {
    eprintln!("Usage:");
    eprintln!("  {program} --cli    Start the console user interface");
}

fn main() -> ExitCode {
    let mut arguments = std::env::args();

    // The name the program was started with, for the usage text.
    let program = arguments.next().unwrap_or_else(|| "io-master".to_owned());
    let arguments: Vec<String> = arguments.collect();

    match parse_arguments(&arguments) {
        Ok(Mode::Console) => {
            cli_ui::console::console_ui();
            ExitCode::SUCCESS
        }
        Err(error) => {
            match error {
                ArgumentError::Missing => eprintln!("Missing parameter"),
                ArgumentError::Unknown(parameter) => {
                    eprintln!("Unknown parameter: '{parameter}'");
                }
                ArgumentError::Unexpected(parameter) => {
                    eprintln!("Unexpected parameter: '{parameter}'");
                }
            }

            print_usage(&program);

            // 2 is the usual exit code for a wrong command line.
            ExitCode::from(2)
        }
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
