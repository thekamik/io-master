//! The console user interface.
//!
//! A loop that shows a prompt, reads one line and carries out
//! the command on it, until `exit` or the end of the input
//! (Ctrl+D).
//!
//! The work is split in two, so that each part can be tested on
//! its own and so that the same commands can later be offered by
//! the web user interface:
//!
//! ```text
//! parse_command()   text of a line -> Command    (no input/output)
//! run()             the loop, on any input and output, with the
//!                   functions that do the work passed in
//! console_ui()      run() on the terminal, with the real functions
//! ```
//!
//! Tables and other longer output are built in files of their
//! own, for example `port_table.rs`.

use std::io::{self, BufRead, Write};

use crate::devices::devices::{self, PortType, ScanError, SerialPortInfo};
use crate::ui::port_table::format_port_table;

/// The prompt shown before every line.
const PROMPT: &str = "io-master> ";

/// Shown by the commands that exist but do nothing yet.
const NOT_IMPLEMENTED: &str = "This function is being implemented.";

/// A command typed in the console.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// A line without words; nothing to do.
    Empty,
    /// `help`: list the commands.
    Help,
    /// `scan [TYPE]`: list the serial ports, all of them or only
    /// those of one type.
    Scan(Option<PortType>),
    /// `uart`: work with a serial port (not implemented yet).
    Uart,
    /// `ai`: talk to an AI model (not implemented yet).
    Ai,
    /// `exit`: leave the console and close the program.
    Exit,
    /// A known command used the wrong way; holds the
    /// explanation.
    Invalid(String),
    /// Anything else, as typed.
    Unknown(String),
}

/// The function that scans for serial ports.
///
/// The console receives it as a parameter instead of calling
/// [`devices::scan`] itself, so that the tests can give it a scan
/// with known results and need no hardware.
pub type Scanner = fn(Option<&PortType>) -> Result<Vec<SerialPortInfo>, ScanError>;

/// The commands and their descriptions, in the order `help`
/// lists them.
const COMMANDS: [(&str, &str); 5] = [
    ("help", "Show this list of commands"),
    ("scan [TYPE]", "List the serial ports; TYPE shows one kind only: ttyACM, ttyUSB, ttyS, ..."),
    ("uart", "Work with a serial port (UART)"),
    ("ai", "Talk to an AI model"),
    ("exit", "Leave the console and close the program"),
];

/// Recognizes the command on one typed line.
///
/// Words are separated by spaces. Names are matched exactly,
/// including letter case, as port types are in Linux.
pub fn parse_command(line: &str) -> Command {
    let words: Vec<&str> = line.split_whitespace().collect();

    match words.as_slice() {
        [] => Command::Empty,
        ["help"] => Command::Help,
        ["scan"] => Command::Scan(None),
        ["scan", port_type] => Command::Scan(Some(PortType::from_base_name(port_type))),
        ["scan", ..] => Command::Invalid("'scan' takes at most one port type".to_owned()),
        ["uart"] => Command::Uart,
        ["ai"] => Command::Ai,
        ["exit"] => Command::Exit,
        [name, ..] if ["help", "uart", "ai", "exit"].contains(name) => {
            Command::Invalid(format!("'{name}' takes no arguments"))
        }
        _ => Command::Unknown(line.trim().to_owned()),
    }
}

/// Writes the list of commands.
fn write_help(output: &mut impl Write) -> io::Result<()> {
    writeln!(output, "Commands:")?;

    for (name, description) in COMMANDS {
        writeln!(output, "  {name:<12} {description}")?;
    }

    Ok(())
}

/// Command `scan`: shows the table of serial ports.
fn show_ports(output: &mut impl Write, scanner: Scanner, filter: Option<&PortType>) -> io::Result<()> {
    // A type the program does not know may still exist (the
    // scan reports such ports as well), but more often it is
    // a typing mistake. Say so, then scan anyway.
    if let Some(PortType::Other(name)) = filter {
        let known: Vec<String> = PortType::known().map(|port_type| port_type.to_string()).collect();

        writeln!(output, "Note: '{name}' is not a known port type.")?;
        writeln!(output, "Known types: {}", known.join(", "))?;
    }

    let ports = match scanner(filter) {
        Ok(ports) => ports,
        Err(error) => return writeln!(output, "Cannot scan for serial ports: {error}"),
    };

    if ports.is_empty() {
        return match filter {
            Some(port_type) => writeln!(output, "No serial ports of type {port_type} found."),
            None => writeln!(output, "No serial ports found."),
        };
    }

    write!(output, "{}", format_port_table(&ports))
}

/// The console loop on any input and output.
///
/// `scanner` is the function that the `scan` command uses.
///
/// Ends on `exit` or at the end of the input. Fails only when
/// the input cannot be read or the output cannot be written.
pub fn run(input: &mut impl BufRead, output: &mut impl Write, scanner: Scanner) -> io::Result<()> {
    writeln!(output, "Type 'help' to list commands")?;

    let mut line = String::new();

    loop {
        // The prompt has no line end, so it is flushed by hand
        // to appear before the program waits for the line.
        write!(output, "{PROMPT}")?;
        output.flush()?;

        line.clear();

        // 0 bytes: the input has ended (Ctrl+D). Start a new
        // line, so the shell of the system does not continue on
        // the line of the prompt.
        if input.read_line(&mut line)? == 0 {
            writeln!(output)?;
            return Ok(());
        }

        match parse_command(&line) {
            Command::Empty => {}
            Command::Help => write_help(output)?,
            Command::Scan(filter) => show_ports(output, scanner, filter.as_ref())?,
            Command::Uart | Command::Ai => writeln!(output, "{NOT_IMPLEMENTED}")?,
            Command::Exit => return Ok(()),
            Command::Invalid(explanation) => writeln!(output, "{explanation}")?,
            Command::Unknown(name) => {
                writeln!(output, "Unknown command: '{name}' (type 'help' to list commands)")?;
            }
        }
    }
}

/// Starts the console user interface on the terminal.
pub fn console_ui() {
    let stdin = io::stdin();
    let stdout = io::stdout();

    // An error here means that the terminal itself is gone, for
    // example a closed pipe. There is nobody left to tell, so
    // the console simply ends.
    let _ = run(&mut stdin.lock(), &mut stdout.lock(), devices::scan);
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "console_tests.rs"]
mod tests;
