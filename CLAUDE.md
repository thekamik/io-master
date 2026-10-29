# IO Master: working rules

**A new session starts by reading `docs/handover.md`** (the state of the work, what comes next, how to build and test).

## The models: only with the user's consent (user, 2026-10-29)

**The models of `io-master/src/model/` are changed only with the user's consent and permission.** Every change of a model (a field added, removed, renamed or retyped, a model added or removed) is proposed first and implemented only after the user has accepted it. The models are the schema of the whole project (`docs/data_model.md`): the relations between them are reflected in the database and in the window, so a view processes the models whose relations it shows; when it is unclear which models a view should process, ask the user.

## The project

- IO Master is a desktop program (Rust 2024, Tauri 2 + Svelte 5, `io-master/`) for security evaluation laboratories: the evaluation of a product against standards kept as NIST OSCAL catalogs, with AI agents that bring their knowledge (RAG) to each work unit. What it is and why: `docs/architecture.md`; the models: `docs/data_model.md`; where everything is in the code: `docs/code_structure.md`.
- **It is being rebuilt on the models of `model/`** (user, 2026-10-28). `test_branch` holds the new code; `main` holds the earlier program as a reference (`git show main:io-master/src/<path>`), whose parts come back one by one, rewritten on the models. The earlier databases are dropped: nothing is moved from them.
- **Branches and commits.** Work is committed to `test_branch` as it goes and pushed to `origin`. **Nothing goes to `main`** until the user revokes this (2026-10-28), not even with a question. No other branches, no version numbers (the program stays 0.1.0).
- **The data of the tests is temporary until version 1** (user, 2026-10-24): the layout of the databases changes as the code needs, with no migration; the user starts the tests anew after a change.
- **The documentation reflects the current state of the code** (user, 2026-10-21): a change of the code changes its documentation in the same commit. The documents describe only what is; the history is in git (user, 2026-10-29). `docs/todo.md` lists only the open work.
- **Architecture, readability and security come first** (user, 2026-10-17): every data flow must be traceable by the user, and every function does one thing (user, 2026-10-21). Code that works but cannot be followed is a critical error, fixed before further functions.

## Way of working

- **Concept first, approved by the user, then code.** After each larger stage ask for the approval of the next, unless the user allowed a whole step; short mechanical phases of an approved step run without asking.
- **Questions to the user are numbered, one short point each**, with a recommendation, so that the user answers by number (user, 2026-10-29).
- Small steps, each tested on its own; window tests only on a private Xvfb display (`docs/handover.md`), never on the user's screen.
- **Comments** (user, 2026-10-23): at every function, what it does, where it is called from and which functions it calls. Descriptions of the architecture are in `docs/`, as Markdown with schemes, never repeated over the code.
- **Tests** live in the file whose functions they test, as its last item: `#[cfg(test)] mod tests { use super::*; ... }`. Help for the tests only in `src/test_support.rs` and `src/test_support/`; the test of the whole program in `src/architecture_tests.rs`.
- `cargo fmt` (`rustfmt.toml`, 120 characters), `cargo clippy --all-targets` and `npm run check` before every commit.
- Prefer few, well-known dependencies, versions pinned; explain each one when it is added.
- Conversation with the user in Polish, addressed as Kamil, informally ("ty"); code, comments and documents in English.

## The rules of the code: "The Power of 10" (user, 2026-10-21)

The user's bible; they must not be broken. As the user gave them (G. J. Holzmann, NASA/JPL, written for C):

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

### The rules for Rust (adapted with the user's leave, 2026-10-21)

They bind **our code** (`src/`, `ui/`). The dependencies cannot follow them; they are kept few and well known and reached through a narrow, documented boundary.

1. **Control flow.** No recursion. No `panic!`, `unreachable!`, `todo!`, `unimplemented!` outside the tests. `?` is allowed. A tree is walked with an explicit stack bounded by a constant. In Svelte: no recursive snippets or components.
2. **Bounded loops.** A `loop` or `while` is written as `for _ in 0..MAX_...` with a named constant. A `for` over a collection is bounded by the limit of that collection, checked when the data entered the program. A stream is read up to a constant number of bytes or events.
3. **Bounded memory.** Every input has a maximum size checked at the boundary: a file, a text, a JSON document, an answer of the AI, the number of objects. No collection grows without a limit. `Vec::with_capacity` where the size is known.
4. **Short functions.** At most 60 lines per function, one statement per line.
5. **Checks with recovery.** No `assert!` or `debug_assert!` outside the tests. A condition that should never happen is checked with `ensure!(condition, error)` (`checks.rs`), which returns the error. On average two per function, for what the types do not guarantee.
6. **Smallest scope.** No `static mut`, no global mutable state besides the session (`session/state.rs`).
7. **Every result handled, every parameter checked.** No `unwrap`, `expect`, `let _ =` on a `Result`, or `.ok()` that drops an error. What comes from the page is checked once at its entry.
8. **Macros.** Own `macro_rules!` only simple (like `ensure!`). The macros of std are allowed. Procedural macros (serde derives, `#[tauri::command]`) only at the boundary to the page. `#[cfg]` only as `#[cfg(test)]`.
9. **References.** `#![forbid(unsafe_code)]`; no raw pointers, no `dyn Trait`, no `Rc<RefCell<...>>`. No function pointers and no closures as parameters of our own functions; closures only where a library requires them, short, the place named; prefer a `for` loop.
10. **Zero warnings.** clippy clean before every commit; in the page TypeScript strict and `svelte-check` clean. **One exception during the rebuild (user, 2026-10-29): the warnings `dead_code` stay**, without `#[allow(dead_code)]`: each reminds which part is still to be joined to a panel.

## The window and the layers

- **The desktop window is the only interface.** The program is started without parameters, opens no network port and is only a client of the AI services.
- **The panels** (`panels/`, user 2026-10-28): `panels/views/<panel>/` holds only the commands the page calls; `panels/logic/<panel>/` holds the work behind them, gets the open database as a parameter and never reaches the session. A command is listed in three places that must agree: `tauri::generate_handler!` in `main.rs`, `build.rs` and `capabilities/default.json`; its function on the page in `ui/src/lib/api/<panel>.ts`, named as the command.
- **The page and the Rust program are two separate projects** (the page: `io-master/ui/`). The page holds no secret and no state that matters; only presentation stays in it (a tab, a selection, an open form). The one secret that reaches it is the recovery code, shown once.
- **Core logic is separate from presentation**: results and errors are plain Rust types; serialization only in `panels/views/` (the models carry no serde); long work reports its progress (a channel of Tauri) and can be stopped.
- **The direction of the dependencies** is checked by `src/architecture_tests.rs`; `main.rs` is the map of every module, in the order of the layers.

## The database

- **Create, read and update only: no delete** (user, 2026-10-28), with one exception: `delete_work_unit`, only for a plan in planning (user, 2026-10-29).
- **The layout of `db/` mirrors `model/`**: one file per file of a model, with its tables and its functions `create_<model>`, `read_<model>`, `update_<model>`.
- **SQL lives only in `db/`.** Columns read by name, statements with several parameters name them, one transaction per operation, a value the program never writes is `DbError::Corrupt`.
- `db/encryption/` holds the encryption of the databases, moved unchanged from the earlier program; the user audits it later.

## The AI

- **`ai/` is the only way to an AI service** (user, 2026-10-29): `prepare` (under the lock of the session), `send` (without it), `record` (the journal), in `ai/ask.rs`; the answer is read only after `record`.
- **Every question of an agent is kept in the journal of that agent**; a question without an agent (the check of an integration) is not kept.
- An agent is a layer of knowledge over the model of an integration; integrations are only in the AI settings, agents and knowledge bases in the panel Agents.

## Security

- Secrets (API keys, the key of the database) only in the encrypted database of the user, never shown back, never printed, never in the system database.
- Every connection leaving the computer uses TLS 1.3 with a verified certificate; plain HTTP only to this computer.
- The password is the only factor (Argon2id); a forgotten password is recovered with the recovery code.

## Code review

- The user reviews the code by hand, file by file; Claude answers precisely, points out errors, and changes code only on request.
- `// ok kk` above a function means: approved by the user; every line added or removed in it needs the user's approval first. Claude never adds, moves or removes the marker.
- `review.md` records the state of the review; a whole file approved in the chat is written there with the date.
- Decisions of the user that matter beyond the current task are written down at once (this file for working rules, `docs/` for the concept).
