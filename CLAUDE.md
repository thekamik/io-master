# IO Master: working rules

**A new session starts by reading `docs/handover.md`** (the state of the work, what comes next, how to build and test).

## Project state

- The new version is the Rust project in `io-master/`. It is built from scratch, not ported line by line.
- Version 0.2 (C) is the reference for *what* the tool can do: `v0.2_functionality.md` lists its functions with IDs (CLI-1, UART-4, ...). Its code is in git history (commit `6cc4091`). Functions are re-created one at a time, redesigned for the new version.
- The long-term direction is AI-assisted Common Criteria evaluation (`v0.2_functionality.md`, section 3.7).
- **The agreed concept of v0.3 is in `docs/architecture.md`**: one program with two roles chosen once at the first start in the window, one Linux account = one installation = one user (workstation and master alike), a desktop window (Tauri 2 + Svelte) as the only interface, the password plus the secret of the application in the keyring of the Linux account as the default factors (a desktop session with GNOME Keyring is required) with the TPM 2.0 as an optional further factor, optional password reset through the master, collaboration through the laboratory's git. Read it before designing a function.
- The program must run without a TPM (virtual machines). TODO before release: the optional TPM factor uses a real TPM 2.0, not the emulator `swtpm`.
- **The work follows `docs/todo.md`, one step at a time, in the order of its table "order of work"; step numbers are names and never change.** Every step: concept approved by the user, implementation with tests, check, approval, commit. When the list is done, further functionality is planned anew.
- **The order of work is a guide, not an absolute rule** (user, 2026-10-17). Architecture, readability of the code and security come first, so that the program stays maintainable and auditable: when the user finds that a part of the code escapes their overview (as the database code did), it is a critical error and is fixed at once, before further functions.

## Main design rule: the desktop window is the only interface

Since v0.4 the whole workflow is controlled from a **desktop window** (Tauri 2 + Svelte, `src/tauri_ui/` and `ui/`; `docs/architecture.md`, section 4). The program is started without parameters and opens no network port for its interface; network functions (the reset service of a master and its client) are started from the window. There is no console interface. The web interface of v0.3 was removed (branch `v0.3` in git). Every function is designed so that the interface can use it without changes to the function:

- **Core logic is separate from presentation.** Functions in the core never print, never read from the terminal, and never exit the process. They take parameters and return results; the interface presents them.
- **Results and errors are data.** Internal functions return plain Rust types (structs, enums, `Vec`, `Result`) and are used only inside Rust. Errors are enums that the UI can tell apart, not only text.
- **Serialization belongs to the UI.** Conversion to JSON or any other external format is done only at the boundary to the interface, in `io-master/src/tauri_ui/serializer/` (desktop interface, Tauri 2 + Svelte, since v0.4). The core types carry no serialization code and no serde attributes.
- **Long operations report progress as events** (for example a channel or a stream), not by printing. The window receives them live through a channel of Tauri.
- **Operations can be cancelled by the caller** (a cancellation token).
- **Questions to the user go through the interface.** When an operation needs an answer (a PIN, a confirmation), it asks through an interface the UI implements, never by reading stdin itself.
- **State lives in the core, not in the page.** The page holds no secret and no state that matters.

## Rules carried over from v0.2

- No command reaches a device without an action of a person for that command. The program proposes; a person executes.
- Secrets (API keys, the key of the database, git credentials) only in the encrypted databases of the evaluators, opened by the password (+ optional TPM factor) (`docs/architecture.md`, section 5). Never in plain files, and never in the system database, which is not encrypted.
- Every network connection that leaves the computer uses TLS 1.3 or newer with a verified certificate (workstation and master, AI providers, git).
- Code comments and documentation in English, thorough. Conversation with the user in Polish; the user is Kamil: address them by name, informally ("ty"), never "Pan".

## Code review (step 12 of docs/todo.md)

- The user reviews the code by hand, file by file: reads, asks questions, edits. Claude answers precisely, points out errors it sees, and changes code only on request. Claude does not review the architecture on its own initiative and does not push for decisions while the user is still reading a file.
- `// ok kk` above a function (or a fragment) means: approved by the user; every line added or removed in it needs the user's approval first. Reading it and suggesting changes is fine. Its tests may be changed. Claude never adds, moves or removes the marker.
- `review.md` (repository root) records the state of the review. When the user says in the chat that a whole file is reviewed, Claude writes it there with the date; every function of such a file counts as marked `// ok kk`.
- Decisions and rules the user gives in the chat that matter beyond the current task are written down at once (this file for working rules, `docs/` for the concept and the plan), so that they survive a reset of the context.

## Database code (step 37 of docs/todo.md)

- **SQL lives only in `io-master/src/db/`.** The modules of a domain (`standards/`, `test_projects/`, `evaluation/`, `ai/`, ...) keep their logic and call the functions of `db/`; they never write SQL.
- `db/` has one subfolder per area of data; each holds `schema.rs` (its tables), `rows.rs` (row ↔ Rust type) and the files of CRUD: `create.rs`, `read.rs`, `update.rs`, `delete.rs` (a file that an area does not need is missing, which is visible in an audit).
- `db/encryption/` holds all code of encryption; the head of every area states in which database its data lives and how it is protected.
- The mechanism of encryption is moved as it is, unchanged; the user audits it and redesigns it later.
- **Whatever happens in the database is in `db/`, in a subfolder of its own** (user, 2026-10-17), so that nothing is mixed: for example `db/ai/connection/`, `db/ai/memory/`, `db/ai/presets/`. Rules of the stored data (a name unique, a key bound to its address) live there with their CRUD.

## AI layer (step 38 of docs/todo.md)

- **`ai/api/` is the only way to the AI**, its functions in the form of REST (resources and operations: `roles`, `tasks`, `completions`, `checks`); not a server. `ai/api/provider/` (HTTP, TLS, request, answer, stream) is visible only inside `ai/api/`, so a request built elsewhere does not compile.
- Every question is asked in a **task** with its **role** (text analysis, summary, consultation, formal analysis) and recorded in the journal of the AI (`completions::record`).
- `ai/chat/` holds the chat and its memory; operations on text are controlled by parameters, and quotes of a standard are never shortened.

## The pipeline of data (user, 2026-10-17)

- The functions and the responsibilities of the parts of the code are separated, and the pipeline is very readable: **at every moment one can tell where the information the program works on is** (in the page, in an object in memory, in the database, on its way to the AI).
- The CRUD of a domain manages the objects in memory (a project open for editing, with its versions for "Undo"); `db/` writes them lastingly, through at once (write-through); `ai/api/` is the only way to the AI.

## The architecture (user, 2026-10-17; step 40 of docs/todo.md)

The user's guidelines; the target layout that follows them is in `docs/architecture_cleanup.md` (approved 2026-10-17, with the decisions in its section 6). **The cleanup is done on the branch `v0.5`; `v0.4` keeps the state before it and is not changed. No approval of a review moves with the code: every file of v0.5 is reviewed from the start.**

1. `tauri_ui/` is a minimal intermediate layer that only joins the back end and the front end.
2. The business logic is in a folder of its own, reached by the interface in the form of a REST API: no server, but the functions are designed as resources and operations.
3. Every operation on a database is in `db/`, ordered as CRUD.
4. All communication with AI is in `ai/`, the intermediary between the external services and the program (one folder for the whole integration, for the audit).
5. Everything that talks to hardware (devices, UART, Bluetooth, permissions of the ports, ...) is in `hardware_tests/`.
6. The documentation is renewed, with a new README that states the aims of the project as they are now.

## Way of working

- Propose the concept first and get approval, then write code.
- Small steps, each tested on its own (unit tests in Rust, end-to-end tests where a front end is involved) before the next step starts.
- Readability first: the user has to keep an overview of the code. Clear structure, one responsibility per file.
- Unit tests live in their own file next to the module, named `<module>_tests.rs` (`devices.rs` keeps `device_tests.rs`), and are attached with `#[cfg(test)] #[path = "<file>"] mod tests;`.
- Prefer few, well-known dependencies; explain each one when it is added.
