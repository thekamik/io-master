# IO Master: working rules

**A new session starts by reading `docs/handover.md`**: the state of the work, what comes next, how to build and test.

## The project

- IO Master is a desktop program (Rust 2024, Tauri 2 + Svelte 5, `io-master/`) for security evaluation laboratories: the evaluation of a product against standards kept as NIST OSCAL catalogs, with AI agents that bring their knowledge (RAG) to each work unit. What it is and why: `docs/architecture.md`; the models: `docs/data_model.md`; where everything is in the code: `docs/code_structure.md`; where each model is stored, processed and shown: `docs/architecture_map.md`.
- **It is built on the models of `objects/`.** The earlier program is kept as a reference at the commit `0797e85` (`git show 0797e85:io-master/src/<path>`); its parts come back one by one, rewritten on the models. Nothing is moved from its databases.
- **The data of the tests is temporary until version 1**: the layout of the databases changes as the code needs, with no migration; the user starts the tests anew after a change.
- **The present stage (2026-11-02): cleaning and hardening.** The user has no larger remarks on the code; the work is the clean-up of the code and the documents, and the strengthening of the program (the open points: `docs/todo.md`, "Before the release").

## Working with the user

- **Conversation in Polish**, the user addressed as Kamil, informally ("ty"); code, comments and documents in English.
- **Questions are numbered, one short point each**, each with a recommendation, so that the user answers by number.
- **Concept first, approved by the user, then code.** After each larger stage ask for the approval of the next, unless the user allowed a whole step; the short, mechanical phases of an approved step run to its end without asking, each committed.
- **The models are changed only with the user's consent.** Every change of a model of `objects/` (`<object>_model.rs`: a field added, removed, renamed or retyped, a model added or removed) is proposed first and made only once accepted. The models are the schema of the whole project: their relations are reflected in the database and in the window, so a view processes the models whose relations it shows; when it is unclear which models a view should process, ask.
- **Architecture, readability and security come first**: every data flow can be followed by the user, and every function does one thing. Code that works but cannot be followed is a critical error, fixed before further functions.
- **Decisions that matter beyond the task are written down at once**: working rules here, the concept in `docs/`.

### Branches and commits

- Work is committed to **`test_branch`** as it goes, each piece once checked, and pushed to `origin` without asking.
- **`main` and `V0.2`** (the stable version) are brought to the state of `test_branch` only when the user approves it, never on our own, not even with a question. `V0.1` is a closed snapshot, never committed to. No other branches, no version numbers (the program stays 0.1.0).
- Before every commit: `cargo fmt` (`rustfmt.toml`, 120 characters), `cargo clippy --all-targets` and `npm run check`, all clean; `cargo test`.

### The documents

- **The documents describe the present state of the code**, only what is; the history is in git. A change of the code changes its documents in the same commit, `docs/architecture_map.md` included.
- `docs/todo.md` lists only the open work. `README.md` is written for programmers: the architecture in short (the three groups, the roles of the files of an object, where a new function goes), how to build and test, every dependency with why it is there.
- Descriptions of the architecture are in `docs/`, as Markdown with schemes, never repeated over the code.

### The review of the code

- The user reviews the code by hand, file by file; Claude answers precisely, points out errors, and changes code only on request.
- `// ok kk` above a function means approved by the user: every line added or removed in it needs the user's approval first. Claude never adds, moves or removes the marker.
- `review.md` records the state of the review; a whole file approved in the chat is written there with the date, and counts as marked `// ok kk`.
- **A stale comment is mended at once** (a moved file, a renamed module, a caller that no longer calls), also in an approved file and in `infrastructure/storage/encryption/`; the change is noted in `review.md` when the file was approved. Only the comment changes, never the code of an approved or audited file.

## The code

### Three groups

```text
panels/ ──► objects/ ──► infrastructure/
```

- **`objects/`** holds every model with the functions that work on it, a folder per object (`objects/ai/agent/`), its files named by the object and their role: `agent_model.rs` (the structure, fields only), `agent_store.rs` (its tables; create, read, update, as called), `agent_rules.rs` (the rules of the data), `agent_ai.rs` (the work of the AI on it). An object has only the roles it needs; a big role becomes a folder of the same name (`work_unit_ai/`), whose files carry no prefix. The roles go one way, also between objects: `model ◄── store ◄── rules ◄── ai`. `objects/error.rs` is the error of the rules, `objects/ai_error.rs` the error of every role of the AI, `objects/schema.rs` the list of the tables of each database. A loop over the children of a model belongs to the parent. Helper models never kept in a database (`objects/session/`) are models as much as the stored ones. The connection to an AI service and its protocols are the AI of `Integration` (`integration_ai/`).
- **`infrastructure/`** holds what belongs to no object and no panel, and knows no object: `storage/` (the database files, their encryption, `DbError`; the list of the tables given as a parameter), `security/`, `pdf/`, `clock.rs`, `data_folder.rs`.
- **`panels/`** knows the window: `session.rs` (the user logged in, the open database, the progress), `views/<panel>/` (only the commands the page calls), `logic/<panel>/` (only what a panel shows beyond the objects; it gets the open database as a parameter and never reaches the session).
- **Every function belongs to an object**: it is placed with the model it works on, never as a universal tool shared for the sake of shorter code. Two models that read alike keep each its own function, unless an architectural reason joins them (then ask first).
- The direction of the dependencies is checked by `src/architecture_tests.rs`; `main.rs` is the map of every module, in the order of the groups.

### Names, order, errors

- **Standard names**: modules and items are used by their own names, never renamed on import (`use ... as`); where a name reads ambiguously (the module `password` beside a parameter `password`), a short comment at the call says so.
- **A list gathered from several places follows the folders**: ordered as the folders are, a short comment over each group, the principle of the split stated at the head of the file.
- **Every function easy to find**: the errors of a panel are in its own `error.rs`, each variant naming the error of the backend it holds (`EvaluationError::Ai(AiError)`), the conversion written there once (`From`). No file of the errors of the whole program.

### Comments

The code is written to be read by a human. At every function:

- the first sentence: what it does, in plain words, with an example of the domain ("the dependencies of a work unit");
- `Input:` each parameter, what it is and what it must be; `Output:` what comes back on success, with an example value (`[12, 15]`), and when an error comes back;
- `Called by:` and `Calls:` precise, each function with its full path (`` `objects/evaluation/work_unit/work_unit_store.rs`: `read_lists` ``), the functions of one file after its path, the files parted by `;`; a function of the same file as [`name`]; never "the stores". A command is called by its function of the page (`` `ui/src/lib/api/agents.ts`: `agentList` ``) from its component;
- inside, a comment over every step that is not plain from the code: why, and what it means, never a repetition of the code.

A very short function (one or two lines) has only its first sentence and `Called by:`.

### Tests

- Tests live in the file whose functions they test, as its last item: `#[cfg(test)] mod tests { use super::*; ... }`.
- Help for the tests only in `src/test_support.rs` and `src/test_support/`; the test of the whole program in `src/architecture_tests.rs`.
- Small steps, each tested on its own; window tests only on a private Xvfb display (`docs/handover.md`), never on the user's screen.

### Dependencies

Few, well known, versions pinned; each explained where it is declared and in `README.md` when it is added. They are reached through a narrow, documented boundary.

## The rules of the code: "The Power of 10"

The user's bible; they must not be broken. As G. J. Holzmann (NASA/JPL) wrote them for C:

1. Very simple control flow only: no goto, no setjmp/longjmp, no direct or indirect recursion.
2. Every loop has a fixed upper bound that a checking tool can prove statically.
3. No dynamic memory allocation after initialization.
4. No function longer than one printed page: about 60 lines, one line per statement and per declaration.
5. At least two assertions per function on average, for conditions that should never happen; side-effect free, Boolean; a failing assertion takes an explicit recovery action (an error returned to the caller); an assertion a tool can prove always true or always false breaks the rule.
6. Every data object declared at the smallest possible scope.
7. Every caller checks the return value of every non-void function; every called function checks the validity of all its parameters.
8. The preprocessor only for including headers and simple macros: no token pasting, no variable argument lists, no recursive macros; every macro expands into a complete syntactic unit; conditional compilation kept to a minimum.
9. Pointers restricted: at most one level of dereference, none hidden in macros or typedefs; no function pointers.
10. All warnings enabled at the most pedantic level from the first day, the code compiles without warnings, and it is checked daily with at least one strong static analyzer with zero warnings.

### The rules for Rust

They bind **our code** (`src/`, `ui/`); the dependencies cannot follow them.

1. **Control flow.** No recursion. No `panic!`, `unreachable!`, `todo!`, `unimplemented!` outside the tests. `?` is allowed. A tree is walked with an explicit stack bounded by a constant. In Svelte: no recursive snippets or components.
2. **Bounded loops.** A `loop` or `while` is written as `for _ in 0..MAX_...` with a named constant. A `for` over a collection is bounded by the limit of that collection, checked when the data entered the program. A stream is read up to a constant number of bytes or events.
3. **Bounded memory.** Every input has a maximum size checked at the boundary: a file, a text, a JSON document, an answer of the AI, the number of objects. No collection grows without a limit. `Vec::with_capacity` where the size is known.
4. **Short functions.** Functions as elementary as possible: each does one thing, and the larger work is made of their calls. About 60 lines is the aim, not a hard limit: a longer function that reads well is allowed where splitting it would not make it clearer. One statement per line.
5. **Checks with recovery.** No `assert!` or `debug_assert!` outside the tests. A condition that should never happen is checked with `ensure!(condition, error)` (`checks.rs`), which returns the error; never a silent default, a skipped element or a substituted value. On average two per function, for what the types do not guarantee; a check that only fills the average is not written.
6. **Smallest scope.** No `static mut`, no global mutable state besides the session (`panels/session.rs`).
7. **Every result handled, every parameter checked.** No `unwrap`, `expect`, `let _ =` on a `Result`, or `.ok()` that drops an error. What comes from the page is checked once at its entry.
8. **Macros.** Own `macro_rules!` only simple (like `ensure!`). The macros of std are allowed. Procedural macros (serde derives, `#[tauri::command]`) only at the boundary to the page. `#[cfg]` only as `#[cfg(test)]`: the tests run with the parameters of the program (Argon2id at its full cost, `argon2` built optimized in the dev profile).
9. **References.** `#![forbid(unsafe_code)]`; no raw pointers, no `dyn Trait`, no `Rc<RefCell<...>>`. No function pointers and no closures as parameters of our own functions; closures only where a library requires them (`map_err`, `ok_or_else`, `split`), short; prefer a `for` loop and a named function.
10. **Zero warnings**, `dead_code` included: clippy at its `pedantic` level (`[lints]` in `Cargo.toml`, each lint switched off there with its reason) clean before every commit; in the page TypeScript strict and `svelte-check` clean. Code no panel calls is removed, not kept for later.

## The window

- **The desktop window is the only interface.** The program is started without parameters, opens no network port and is only a client of the AI services.
- **The page and the Rust program are two separate projects** (the page: `io-master/ui/`). The page holds no secret and no state that matters; only presentation stays in it (a tab, a selection, an open form). The one secret that reaches it is the recovery code, shown once.
- **A command** is listed in three places that must agree: `tauri::generate_handler!` in `main.rs`, `build.rs` and `capabilities/default.json`. Its function on the page is in `ui/src/lib/api/<panel>.ts`, one function per command, named as the command (`agentGet` calls `agent_get`).
- **Core logic is apart from presentation**: results and errors are plain Rust types; serialization only in `panels/views/` (the models carry no serde); long work reports its progress and can be stopped.

## The database

- **Create, read and update only: no delete**, with one exception: `delete_work_unit`, only for a plan in planning.
- **The database files and the models in them are apart**: `infrastructure/storage/` holds the database files (where they are, made, opened, their tables made from the list its caller gives, their encryption, the lists of ids, `DbError`) and knows no model; the store of each object holds the model in them.
- **One store per model** (`<object>_store.rs`), with its tables and those of its functions `create_<model>`, `read_<model>`, `update_<model>` that are called; the tables of each database are listed in `objects/schema.rs`, the help for the lists of ids is `infrastructure/storage/lists.rs`.
- **SQL lives only in `infrastructure/storage/` and the stores** (checked by `architecture_tests.rs`). Columns read by name, statements with several parameters name them, one transaction per operation, a value the program never writes is `DbError::Corrupt`.
- `infrastructure/storage/encryption/` holds the encryption of the databases, moved unchanged from the earlier program; the user audits it later.

## The AI

- **The AI of the integration is the only way to an AI service**: `prepare` (under the lock of the session), `send` (without it), `record` (the journal), in `objects/ai/integration/integration_ai/question.rs`, `integration_ai/sending.rs` and `objects/ai/journal/journal_ai.rs`; the answer is read only after `record`.
- **Every question of an agent is kept in the journal of that agent**; a question without an agent (the check of an integration) is not kept.
- An agent is a layer of knowledge over the model of an integration; integrations are only in the AI settings, agents and knowledge bases in the panel Agents.

## Security

- Secrets (API keys, the key of the database) only in the encrypted database of the user, never shown back, never printed, never in the system database.
- Every connection leaving the computer uses TLS 1.3 with a verified certificate; plain HTTP only to this computer.
- The password is the only factor (Argon2id); a forgotten password is recovered with the recovery code.
