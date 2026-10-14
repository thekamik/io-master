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
//! run()             the loop, on any input and output
//! console_ui()      run() on the terminal
//! ```

use std::io::{self, BufRead, Write};

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
    /// `scan`: look for devices (not implemented yet).
    Scan,
    /// `uart`: work with a serial port (not implemented yet).
    Uart,
    /// `ai`: talk to an AI model (not implemented yet).
    Ai,
    /// `exit`: leave the console and close the program.
    Exit,
    /// Anything else, as typed.
    Unknown(String),
}

/// The commands and their descriptions, in the order `help`
/// lists them.
const COMMANDS: [(&str, &str); 5] = [
    ("help", "Show this list of commands"),
    ("scan", "Scan for devices"),
    ("uart", "Work with a serial port (UART)"),
    ("ai", "Talk to an AI model"),
    ("exit", "Leave the console and close the program"),
];

/// Recognizes the command on one typed line.
///
/// Spaces around the command are ignored. Names are matched
/// exactly, including letter case.
pub fn parse_command(line: &str) -> Command {
    match line.trim() {
        "" => Command::Empty,
        "help" => Command::Help,
        "scan" => Command::Scan,
        "uart" => Command::Uart,
        "ai" => Command::Ai,
        "exit" => Command::Exit,
        other => Command::Unknown(other.to_owned()),
    }
}

/// Writes the list of commands.
fn write_help(output: &mut impl Write) -> io::Result<()> {
    writeln!(output, "Commands:")?;

    for (name, description) in COMMANDS {
        writeln!(output, "  {name:<6} {description}")?;
    }

    Ok(())
}

/// The console loop on any input and output.
///
/// Ends on `exit` or at the end of the input. Fails only when
/// the input cannot be read or the output cannot be written.
pub fn run(input: &mut impl BufRead, output: &mut impl Write) -> io::Result<()> {
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
            Command::Scan | Command::Uart | Command::Ai => {
                writeln!(output, "{NOT_IMPLEMENTED}")?;
            }
            Command::Exit => return Ok(()),
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
    let _ = run(&mut stdin.lock(), &mut stdout.lock());
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "console_tests.rs"]
mod tests;
