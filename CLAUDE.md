# IO Master: working rules

## Project state

- The new version is the Rust project in `io-master/`. It is built from scratch, not ported line by line.
- Version 0.2 (C) is the reference for *what* the tool can do: `v0.2_functionality.md` lists its functions with IDs (CLI-1, UART-4, ...). Its code is in git history (commit `6cc4091`). Functions are re-created one at a time, redesigned for the new version.
- The long-term direction is AI-assisted Common Criteria evaluation (`v0.2_functionality.md`, section 3.7).
- **The agreed concept of v0.3 is in `docs/architecture.md`**: one program with two roles fixed at installation (workstation, master), web UI only, all secrets protected by TPM 2.0, optional password reset through the master, collaboration through the laboratory's git. Read it before designing a function.
- TODO before release: development uses the TPM 2.0 emulator `swtpm`; it must be replaced by a real TPM 2.0.
- **The work follows `docs/todo.md`, one step at a time.** Every step: concept approved by the user, implementation with tests, check, approval, commit. When the list is done, further functionality is planned anew.

## Main design rule: the web UI is the only interface

The whole workflow is controlled from a **web UI** served over HTTPS (TLS 1.3 or newer). There is no console interface; `cli_ui/` is to be removed. Every function is designed so that the web UI can use it without changes to the function:

- **Core logic is separate from presentation.** Functions in the core never print, never read from the terminal, and never exit the process. They take parameters and return results; the web UI presents them.
- **Results and errors are data.** Internal functions return plain Rust types (structs, enums, `Vec`, `Result`) and are used only inside Rust. Errors are enums that the UI can tell apart, not only text.
- **Serialization belongs to the UI.** Conversion to JSON or any other external format is done only at the boundary to the web UI, in `io-master/src/web_ui/serializer/`. The core types carry no serialization code and no serde attributes.
- **Long operations report progress as events** (for example a channel or a stream), not by printing. The browser receives them live over a WebSocket or server-sent events.
- **Operations can be cancelled by the caller** (a cancellation token).
- **Questions to the user go through the web UI.** When an operation needs an answer (a PIN, a confirmation), it asks through an interface the UI implements, never by reading stdin itself.
- **State lives in the core, not in a page.** Several browser tabs may look at the same state.

## Rules carried over from v0.2

- No command reaches a device without an action of a person for that command. The program proposes; a person executes.
- Secrets (API keys, the key of the database, git credentials) only in protected storage: today the Secret Service keyring, in the target design the TPM 2.0 (`docs/architecture.md`, section 5). Never in plain files.
- Every network connection that carries a secret or evaluation data uses TLS 1.3 or newer with a verified certificate (web UI, AI providers, master, git).
- Code comments and documentation in English, thorough. Conversation with the user in Polish.

## Way of working

- Propose the concept first and get approval, then write code.
- Small steps, each tested on its own (unit tests in Rust, end-to-end tests where a front end is involved) before the next step starts.
- Readability first: the user has to keep an overview of the code. Clear structure, one responsibility per file.
- Unit tests live in their own file next to the module, named `<module>_tests.rs` (`devices.rs` keeps `device_tests.rs`), and are attached with `#[cfg(test)] #[path = "<file>"] mod tests;`.
- Prefer few, well-known dependencies; explain each one when it is added.
