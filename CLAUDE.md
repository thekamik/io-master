# IO Master: working rules

## Project state

- The new version is the Rust project in `io-master/`. It is built from scratch, not ported line by line.
- Version 0.2 (C) is the reference for *what* the tool can do: `v0.2_functionality.md` lists its functions with IDs (CLI-1, UART-4, ...). Its code is in git history (commit `6cc4091`). Functions are re-created one at a time, redesigned for the new version.
- The long-term direction is AI-assisted Common Criteria evaluation (`v0.2_functionality.md`, section 3.7).

## Main design rule: the web UI drives the workflow

In the future the whole workflow is controlled from a **web UI**. The console is a second front end, not the main one. Every function is designed so that a web UI can use it without changes to the function:

- **Core logic is separate from presentation.** Functions in the core never print, never read from the terminal, and never exit the process. They take parameters and return results; the console and the web UI present them.
- **Results and errors are data.** Internal functions return plain Rust types (structs, enums, `Vec`, `Result`) and are used only inside Rust. Errors are enums that a front end can tell apart, not only text.
- **Serialization belongs to the UI.** Conversion to JSON or any other external format is done only at the boundary to a front end, in `io-master/src/ui/serializer/`. The core types carry no serialization code and no serde attributes.
- **Long operations report progress as events** (for example a channel or a stream), not by printing. A front end shows them live: a terminal redraws, a browser receives them over a WebSocket or server-sent events.
- **Operations can be cancelled by the caller** (a cancellation token), not only by Ctrl+C. Ctrl+C is the console's way of cancelling.
- **Questions to the user go through the front end.** When an operation needs an answer (a PIN, a confirmation), it asks through an interface the front end implements, never by reading stdin itself.
- **State lives in the core, not in a shell.** Several front ends, or several browser tabs, may look at the same state.

## Rules carried over from v0.2

- No command reaches a device without an action of a person for that command. The program proposes; a person executes.
- API keys only in the system keyring; requests that carry a key only over HTTPS with TLS 1.3 or newer and a verified certificate.
- Code comments and documentation in English, thorough. Conversation with the user in Polish.

## Way of working

- Propose the concept first and get approval, then write code.
- Small steps, each tested on its own (unit tests in Rust, end-to-end tests where a front end is involved) before the next step starts.
- Readability first: the user has to keep an overview of the code. Clear structure, one responsibility per file.
- Unit tests live in their own file next to the module, named `<module>_tests.rs` (`devices.rs` keeps `device_tests.rs`), and are attached with `#[cfg(test)] #[path = "<file>"] mod tests;`.
- Prefer few, well-known dependencies; explain each one when it is added.
