# IO Master: working rules

**A new session starts by reading `docs/handover.md`** (the state of the work, what comes next, how to build and test).

## Project state

- The new version is the Rust project in `io-master/`. It is built from scratch, not ported line by line.
- Version 0.2 (C) is the reference for *what* the tool can do: `v0.2_functionality.md` lists its functions with IDs (CLI-1, UART-4, ...). Its code is in git history (commit `6cc4091`). Functions are re-created one at a time, redesigned for the new version.
- The long-term direction is AI-assisted Common Criteria evaluation (`v0.2_functionality.md`, section 3.7).
- **The agreed concept is in `docs/architecture.md`**: one program with two roles chosen once at the first start in the window, one Linux account = one installation = one user (workstation and master alike), a desktop window (Tauri 2 + Svelte) as the only interface, the password plus the secret of the application in the keyring of the Linux account as the default factors (a desktop session with GNOME Keyring is required) with the TPM 2.0 as an optional further factor, optional password reset through the master, collaboration through the laboratory's git. Read it before designing a function.
- **The structure of the code is in `docs/code_structure.md`** (the layers, the direction of the dependencies and its test, the conventions, the whole tree). Read it before writing code.
- **Commits (user, 2026-10-21):** Claude commits its changes to `test_branch` as it goes and pushes them to `origin`, so that no work stays on one machine (the user works on two). **Never a commit, merge or push to `main` without the user's permission**, asked each time.
- **The documentation reflects the current state of the code** (user, 2026-10-21): a change of the code changes its documentation in the same commit; a document that describes a plan says so.
- **Branches (user, 2026-10-20):** `main` holds the state that is tested and approved; the work is done on `test_branch` and goes to `main` once it is approved. No other branches and **no version numbers** while the project is at an early stage (the program stays at version 0.1.0 of `Cargo.toml`); the earlier branches `v0.2` to `v0.5` were removed, their history is in `main` (the commits named in `docs/history.md`, section "Branches until 2026-10-20").
- The program must run without a TPM (virtual machines). TODO before release: the optional TPM factor uses a real TPM 2.0, not the emulator `swtpm`.
- **`docs/todo.md` is a heap of reminders, not a plan in order** (user, 2026-10-21): the user chooses what is done next; its table "order of work" is not followed. Step numbers are names and never change. Every step: concept approved by the user, implementation with tests, check, approval, commit.
- **Architecture, readability of the code and security come first** (user, 2026-10-17), so that the program stays maintainable and auditable: when the user finds that a part of the code escapes their overview (as the database code did), it is a critical error and is fixed at once, before further functions. **Every data flow must be traceable by the user, and every function does one thing** (user, 2026-10-21): code that works but cannot be followed cannot be corrected or audited, which is unacceptable.

## The rules of the code: "The Power of 10" (user, 2026-10-21)

The user's bible; they must not be broken. The rules as the user gave them (G. J. Holzmann, NASA/JPL, written for C):

1. Very simple control flow only: no goto, no setjmp/longjmp, no direct or indirect recursion.
2. Every loop has a fixed upper bound that a checking tool can prove statically.
3. No dynamic memory allocation after initialization.
4. No function longer than one printed page: about 60 lines, one line per statement and per declaration.
5. At least two assertions per function on average, for conditions that should never happen; side-effect free, Boolean; a failing assertion takes an explicit recovery action (an error returned to the caller); an assertion a tool can prove always true or always false breaks the rule.
6. Every data object declared at the smallest possible scope.
7. Every caller checks the return value of every non-void function; every called function checks the validity of all its parameters.
8. The preprocessor only for including headers and simple macros: no token pasting, no variable argument lists, no recursive macros; every macro expands into a complete syntactic unit; conditional compilation kept to a minimum.
9. Pointers restricted: at most one level of dereference, none hidden in macros or typedefs; no function pointers.
10. All warnings enabled at the most pedantic level from the first day, the code compiles without warnings, and it is checked daily with at least one (preferably more) strong static analyzer with zero warnings.

### The rules for Rust (adapted by Claude with the user's leave, 2026-10-21; the user may correct each)

They bind **our code** (`src/`, `ui/`). The dependencies (Tauri, rusqlite, serde, rustls, ...) cannot follow them; they are kept few and well known, checked by `cargo-audit` and `cargo-deny`, and reached through a narrow, documented boundary.

1. **Control flow.** No recursion, direct or indirect. No `panic!`, `unreachable!`, `todo!`, `unimplemented!` outside the tests. `?` is a plain return and allowed. A tree (the groups and controls of OSCAL) is walked with an explicit stack bounded by a constant depth; a deeper document is refused with an error. In Svelte: no recursive snippets or components (a tree reaches the page flattened, row and level).
2. **Bounded loops.** A `loop` or `while` is written as `for _ in 0..MAX_...` with a named constant. A `for` over a collection is bounded by the limit of that collection, checked when the data entered the program. A stream (the answer of an AI, a file) is read up to a constant number of bytes or events.
3. **Bounded memory** (instead of "no allocation", which Rust and its libraries cannot do; leaks, use-after-free and double free are excluded by ownership). Every input that enters the program has a maximum size checked at the boundary: a file, its pages, a text, a JSON document, an answer of the AI, the number of objects. No collection grows without a limit in a long work. `Vec::with_capacity` where the size is known.
4. **Short functions.** At most 60 lines per function (`clippy::too_many_lines` with the threshold 60), one statement per line.
5. **Checks with recovery.** No `assert!` or `debug_assert!` outside the tests: they stop the process, which is no recovery. A condition that should never happen is checked with `ensure!(condition, error)`, a simple macro that returns the error to the caller. On average at least two per function, for what the types do not guarantee; a check the compiler or the type already proves is not written (the rule itself forbids it).
6. **Smallest scope.** Every binding at the smallest scope; no `static mut`, no global mutable state besides the state of the window (`app/state.rs`), which says why it is there.
7. **Every result handled, every parameter checked.** No `unwrap`, `expect`, `let _ =` on a `Result`, or `.ok()` that drops an error. What comes from the page is checked once at its entry and turned into a type of `model/` that carries its validity; a function checks what its parameter types do not guarantee.
8. **Macros.** Own `macro_rules!` only simple (like `ensure!`): no recursion, no repetitions `$(...)*`, each expanding into a complete expression or statement. The macros of std (`format!`, `vec!`, `write!`) are allowed. Procedural macros (serde derives, `#[tauri::command]`) only at the boundary to the page. `#[cfg]` only as `#[cfg(test)]`.
9. **References.** `#![forbid(unsafe_code)]`; no raw pointers, no `&&T`, no `dyn Trait` or `Box<dyn ...>`, no `Rc<RefCell<...>>` in our code. No function pointers and no closures as parameters of our own functions. Closures only where a library requires them (`spawn_blocking`, a sort key), short, and each such place names the reason; prefer a `for` loop to an iterator chain with closures.
10. **Zero warnings, from now on.** Warnings of rustc denied; clippy `pedantic` and the restriction lints that carry these rules (`unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `as_conversions`, ...) denied in `Cargo.toml` (`[lints]`); `cargo clippy`, `cargo-audit`, `cargo-deny` clean before every commit. In the page: TypeScript strict, `svelte-check` and eslint clean.

**The code does not follow them yet** (measured 2026-10-21: ~16 recursive functions, 44 `while`/`loop`, 10 functions over 60 lines, no checks of rule 5, 24 closures as parameters, 22 warnings of rustc and 125 of clippy pedantic). Every file that is rebuilt is brought to them; a new file follows them from its first line.

## Terms (user, 2026-10-21)

- **"UI"** in the words of the user was the Rust folder `io-master/src/tauri_ui/`, renamed `io-master/src/app/` in step 43 (2026-10-23); **"business"** was the Rust folder `io-master/src/business/`, replaced by the views (`io-master/src/views/`) in step 42. The page (`io-master/ui/`, Svelte and TypeScript) is called **the page** or the front end.
- **The page and the Rust program are two separate projects that communicate** (through the commands of Tauri). Their files are never mixed in one folder.
- **One vocabulary (user, 2026-10-23): `docs/architecture.md`, section 10.** The user prompts in Polish and the code is in English, so the names of the views (`start`, `shell`, `chat`, `evaluation_plan`, `evaluation_lab`, `devices`, `settings`), of the components and of the folders are used the same way in Rust, in the page, in the navigation and in the documents. A Polish word of the user is read through that table. **A plan (step 43 of `docs/todo.md`):** the code still has some old names (`plans`, `catalog_editor`, `scan`).
- **Views and components (user, 2026-10-23).** A **view** is a whole page of the window, the high-level display. A **component** is a small element used more than once, several times in one view or in several views (example: the window of a *requirement*, which describes a requirement of a standard, appears many times in a view and in other views, and has functions of the AI such as the analysis of the requirement and of the evidence linked to it). Components live in `components/` and are imported by the views; a component never imports a view. A part of a view used only once stays in the folder of its view. No mess: what is a view and what is a component is always clear.

## Main design rule: the desktop window is the only interface

Since step 14 (2026-10-16) the whole workflow is controlled from a **desktop window** (Tauri 2 + Svelte, `src/views/` and `ui/`, a thin layer over the folders below; `src/app/` holds what the whole window shares; `docs/architecture.md`, section 4). The program is started without parameters and opens no network port for its interface; network functions (the reset service of a master and its client) are started from the window. There is no console interface. The earlier web interface was removed (its last state: commit `acedc8b`). Every function is designed so that the interface can use it without changes to the function:

- **Core logic is separate from presentation.** Functions in the core never print, never read from the terminal, and never exit the process. They take parameters and return results; the interface presents them.
- **Results and errors are data.** Internal functions return plain Rust types (structs, enums, `Vec`, `Result`) and are used only inside Rust. Errors are enums that the UI can tell apart, not only text.
- **Serialization belongs to the UI.** Conversion to JSON or any other external format is done only at the boundary to the interface, in the `view.rs` of an element of `io-master/src/views/` and in `io-master/src/app/serializer/` (the error of a command). The core types carry no serialization code and no serde attributes.
- **Long operations report progress as events** (for example a channel or a stream), not by printing. The window receives them live through a channel of Tauri.
- **Operations can be cancelled by the caller** (a cancellation token).
- **Questions to the user go through the interface.** When an operation needs an answer (a PIN, a confirmation), it asks through an interface the UI implements, never by reading stdin itself.
- **State lives in the core, not in the page.** The page holds no secret and no state that matters.
- **What goes to Rust and what stays in the page (user, 2026-10-21).** An element of the window calls the back end only for what matters: running the AI, processing or storing data, anything that changes the state of the evaluation. What is only presentation (hover, an open menu, a selection, a tab, the position during a drag) stays in the page. Only the final result is sent, never the steps in between: for a diagram, the last coordinates (to restore it), not every position of a drag. **The views (user, 2026-10-21; step 42 of `docs/todo.md`, layout in `docs/code_structure.md`, section 2.9):** one folder per view and one per interactive element in `src/views/`, whose commands take the parameters from the page, do the work with the folders below and give the data back; they replace `business/` and the commands of `tauri_ui/`. Every view is moved (Scan the pilot; Start, Shell, Chat, Settings, Evaluation plan, the editor of a catalog); `business/` and the commands of `tauri_ui/` are removed.

## Rules carried over from version 0.2 (the program in C)

- No command reaches a device without an action of a person for that command. The program proposes; a person executes.
- Secrets (API keys, the key of the database, git credentials) only in the encrypted databases of the evaluators, opened by the password (+ optional TPM factor) (`docs/architecture.md`, section 5). Never in plain files, and never in the system database, which is not encrypted.
- Every network connection that leaves the computer uses TLS 1.3 or newer with a verified certificate (workstation and master, AI providers, git).
- Code comments and documentation in English, thorough. Conversation with the user in Polish; the user is Kamil: address them by name, informally ("ty"), never "Pan".

## Code review (step 12 of docs/todo.md)

- The user reviews the code by hand, file by file: reads, asks questions, edits. Claude answers precisely, points out errors it sees, and changes code only on request. Claude does not review the architecture on its own initiative and does not push for decisions while the user is still reading a file.
- `// ok kk` above a function (or a fragment) means: approved by the user; every line added or removed in it needs the user's approval first. Reading it and suggesting changes is fine. Its tests may be changed. Claude never adds, moves or removes the marker.
- `review.md` (repository root) records the state of the review. When the user says in the chat that a whole file is reviewed, Claude writes it there with the date; every function of such a file counts as marked `// ok kk`.
- Decisions and rules the user gives in the chat that matter beyond the current task are written down at once (this file for working rules, `docs/` for the concept and the plan), so that they survive a reset of the context.

## Database code (steps 37 and 40 of docs/todo.md)

- **SQL lives only in `io-master/src/db/`.** The views (`views/`) and `ai/` keep their logic and call the functions of `db/`; they never write SQL.
- `db/` has one subfolder per area of data; each holds `schema.rs` (its tables), `rows.rs` (row ↔ type of `model/`) and the files of CRUD: `create.rs`, `read.rs`, `update.rs`, `delete.rs` (a file that an area does not need is missing, which is visible in an audit).
- Free functions only; a caller imports an area as `db_<area>` and calls `db_<area>::<operation>::<object>`. Columns read by name, statements with several parameters name them, one transaction per operation, a value the program never writes is `DbError::Corrupt`, changes of the layout are migrations by version (`db/user/migrations.rs`).
- `db/encryption/` holds all code of encryption; the head of every area states in which database its data lives and how it is protected.
- The mechanism of encryption is moved as it is, unchanged; the user audits it and redesigns it later.
- **Whatever happens in the database is in `db/`, in a subfolder of its own** (user, 2026-10-17), so that nothing is mixed: for example `db/ai/connection/`, `db/ai/chats/`, `db/ai/journal/`. Rules of the stored data (a name unique, a key bound to its address) live there with their CRUD.

## AI layer (steps 38 and 40 of docs/todo.md)

- **`ai/api/` is the only way to the AI**, its functions in the form of REST (resources and operations: `connections`, `roles`, `tasks`, `completions`, `checks`); not a server. `ai/api/provider/` (HTTP, TLS, request, answer, stream) is visible only inside `ai/api/`, so a request built elsewhere does not compile.
- Every question is asked in a **task** with its **role** (text analysis, summary, consultation, formal analysis) and written into the journal of the AI by `ai/api/` itself (`completions::ask`); no caller can skip it.
- **The text of every question and the reading of every answer are in `ai/tasks/`** (one file per task: `chapter`, `contents`, `chat`), on plain data of `model/`. The memory of the chat (`ai/tasks/chat.rs`) is controlled by parameters, and quotes of a standard are never shortened.

## The pipeline of data (user, 2026-10-17)

- The functions and the responsibilities of the parts of the code are separated, and the pipeline is very readable: **at every moment one can tell where the information the program works on is** (in the page, in an object in memory, in the database, on its way to the AI).
- The session of the evaluator holds the objects in memory (`installation/session.rs`: the open database, a project open for editing with its versions for "Undo", an evaluation open for work); `db/` writes them lastingly, through at once (write-through); `ai/api/` is the only way to the AI.

## The architecture (user, 2026-10-17; step 40 of docs/todo.md)

The user's guidelines, built in step 40 (the layout and its rules: `docs/code_structure.md`). **No approval of a review moved with the code: every file is reviewed from the start of step 40 on** (`review.md`). Guideline 2 was replaced in step 42 by the views ("Main design rule" above): the commands are named `<element>_<action>` and live in the folder of their element.

1. `tauri_ui/` is a minimal intermediate layer that only joins the back end and the front end. (Renamed `app/` in step 43, 2026-10-23: what every view uses, as the session and the error handling of a web framework.)
2. The business logic is in a folder of its own, reached by the interface in the form of a REST API: no server, but the functions are designed as resources and operations.
3. Every operation on a database is in `db/`, ordered as CRUD.
4. All communication with AI is in `ai/`, the intermediary between the external services and the program (one folder for the whole integration, for the audit).
5. Everything that talks to hardware (devices, UART, Bluetooth, permissions of the ports, ...) is in `hardware_tests/`.
6. The documentation is renewed, with a new README that states the aims of the project as they are now.

## Standards and NIST OSCAL (user, 2026-10-18; step 41 of docs/todo.md)

- The work with standards is two functions: the **evaluation plan** (was "test project": the OSCAL scheme of a standard, made or loaded, shown as a graph, handed over) and the **evaluation** (evidence and checks with the AI, the report); besides them the program integrates the tools of the functional tests.
- **The data of the standards is stored in NIST OSCAL** (catalog, mapping, profile, assessment plan, assessment results; SSP and component definition later). What the program adds (AI instructions, prompts, quotes and their checks, proposals, the layout of the graph) is the **evaluation support**, our own model, kept apart and linked to OSCAL objects only by their UUIDs and ids; nothing of ours is written into an OSCAL document.
- Schemes of our own are needed (Common Criteria, ETSI EN 303 645); the tests also run on files of NIST. Concept and open questions: `docs/oscal.md`.

## Way of working

- Propose the concept first and get approval, then write code.
- Small steps, each tested on its own (unit tests in Rust, end-to-end tests where a front end is involved) before the next step starts.
- Readability first: the user has to keep an overview of the code. Clear structure, one responsibility per file.
- Unit tests live in their own file next to the module, named `<file>_tests.rs`, and are attached with `#[cfg(test)] #[path = "<file>_tests.rs"] mod tests;`. `cargo fmt` (`rustfmt.toml`, 120 characters) before every commit.
- A command of the window is listed in four places (the `commands.rs` of its element in `views/`, `routes.rs`, `build.rs`, `capabilities/default.json`); the tests check that they agree, and that no folder uses a folder above it (`src/architecture_tests.rs`).
- Prefer few, well-known dependencies; explain each one when it is added.
