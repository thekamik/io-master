//! Unit tests of `console.rs`.

use super::*;

/// Runs the console on the given typed text and returns what
/// it printed.
fn session(typed: &str) -> String {
    let mut output = Vec::new();

    run(&mut typed.as_bytes(), &mut output).expect("memory never fails");

    String::from_utf8(output).expect("the console prints text")
}

#[test]
fn commands_are_recognized() {
    assert_eq!(parse_command("help\n"), Command::Help);
    assert_eq!(parse_command("  scan  "), Command::Scan);
    assert_eq!(parse_command("uart"), Command::Uart);
    assert_eq!(parse_command("ai"), Command::Ai);
    assert_eq!(parse_command("exit"), Command::Exit);
    assert_eq!(parse_command(" \t\n"), Command::Empty);
    assert_eq!(parse_command("HELP"), Command::Unknown("HELP".to_owned()));
}

#[test]
fn help_lists_every_command() {
    let printed = session("help\nexit\n");

    for (name, _) in COMMANDS {
        assert!(printed.contains(name), "help does not mention {name}");
    }
}

#[test]
fn unfinished_commands_say_so() {
    let printed = session("scan\nuart\nai\nexit\n");

    assert_eq!(printed.matches(NOT_IMPLEMENTED).count(), 3);
}

#[test]
fn exit_ends_the_loop() {
    // The line after exit is never read.
    let printed = session("exit\nhelp\n");

    assert!(!printed.contains("Commands:"));
}

#[test]
fn end_of_input_ends_the_loop() {
    let printed = session("");

    assert!(printed.ends_with(&format!("{PROMPT}\n")));
}

#[test]
fn unknown_commands_are_explained() {
    assert!(session("bogus\n").contains("Unknown command: 'bogus'"));
}
