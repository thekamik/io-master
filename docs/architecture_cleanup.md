# Step 40: The cleanup of the architecture (proposal)

**State: approved 2026-10-17 (decisions in section 6). The work is done on the branch `v0.5`, part by part (section 4).**

This document joins the analysis of the code made on 2026-10-17 (findings A1 to D below) with the guidelines the user gave the same day. It describes the target layout of the code, the rules that keep it, the order of the work, and the new documentation. When it is approved, its rules go into `CLAUDE.md` and `docs/code_structure.md`, its plan into `docs/todo.md` (step 40), and this file is removed.

---

## 1. The guidelines of the user (2026-10-17)

| # | Guideline |
|---|---|
| U1 | `tauri_ui/` is a **minimal intermediate layer** that only joins the back end and the front end. |
| U2 | **The business logic is in a folder of its own.** The interface reaches it in the form of a REST API: no server, but the functions are designed as resources and operations. |
| U3 | **Every operation on a database is in `db/`**, ordered as CRUD. |
| U4 | **All communication with AI is in `ai/`**: the intermediary between the external services and the program. One folder for the whole integration, for the audit. |
| U5 | **Everything that talks to hardware** (devices, UART, Bluetooth, permissions of the ports, ...) **is in `hardware_tests/`**. |
| U6 | **The documentation is renewed**: many concepts changed during the work; a new README states the aims of the project as they are now. |

## 2. Findings of the analysis that this step fixes

| # | Finding | Where today |
|---|---|---|
| A1 | The long works (analysis of chapters, reading a table of contents, loading PDFs) run in the window layer; their progress type is a serde type of the serializer | `tauri_ui/scheme_actions.rs` (`run`), `project_actions.rs` (`propose_schemes`), `standard_actions.rs` (`load`) |
| A2 | The session and the objects in memory live in the window layer; the database is opened, its tables created and old data deleted at **every command**; no `busy_timeout` | `tauri_ui/state.rs`, `tauri_ui/user_data.rs`, `db/user/database.rs` |
| A3 | Cycles between modules: `db → ai`, `ai ↔ test_projects`; `tauri_ui` and the serializer call `db/` directly; static presets in `db/` | `db/ai/connection/`, `ai/api/tasks.rs`, `db/ai/presets/`, `tauri_ui/ai_config_actions.rs` |
| A4 | Saving an AI connection is not one transaction, and the address an API key is bound to is not checked when the key is used | `db/ai/connection/{create,update,delete}.rs`, `ai/api/roles.rs` (`model_of`) |
| B1 | The same code several times: writing and reading tiles, clearing a project, `check_text`, `now` | `db/test_projects/`, `db/evaluations/`, `test_projects/rules.rs`, `evaluation/model.rs` |
| B2 | An SQL statement and its parameters in two files, numbered `?2` to `?22`; columns read by index | `db/test_projects/rows.rs`, `db/ai/connection/rows.rs` |
| B3 | Tuples instead of types (`ProjectRow`, results as `(String, String, String)`) | `db/test_projects/rows.rs`, `db/evaluations/read.rs` |
| B4 | Unknown values in the database replaced in silence (a kind, a state, an action of the journal); `DbError::Sql` used for errors that are not SQL | `db/*/rows.rs` |
| B5 | Errors as text (`Invalid(&'static str, String)`, `NotFound("project")`), matched by their text; conversions to `UiError` in three files | `test_projects/rules.rs`, `tauri_ui/*` |
| B6 | Text trimmed in three places, which must agree for memory to equal the database | `db/`, `test_projects/*/create.rs`, `db/evaluations/create.rs` |
| B7 | Different conventions: methods and free functions in `db/`; the alias `store` for create, update and delete; names of functions; two types `Workspace`; `struct Copy`; `db/ai/memory` holds the chats | everywhere |
| B8 | No migrations: `SCHEMA_VERSION` never read; a column added by hand; old data deleted at every opening | `db/user/database.rs` |
| B9 | The whole module tree is in `main.rs`, so every new file changes an approved file | `main.rs` |
| B10 | One test file covers a whole folder, attached to one module | `test_projects/schemes/read_tests.rs`, ... |
| B11 | No `rustfmt.toml`; 904 differences to `cargo fmt` | the crate |
| C | Front end: pages and components mixed; 1860 lines of global CSS; a command is listed in four places without a check | `ui/src/` |
| D | Documentation out of date (web UI, setup code in the terminal, branch v0.3, the README of v0.2) | `README.md`, `docs/` |

---

## 3. The target layout

### 3.1 Folders

```text
io-master/src/
  main.rs            mod list of the top folders, main(), tauri_startup(); nothing else
  model/             the data of the program as plain Rust types and their rules of validity;
                     no I/O, no database, no network
  db/                every operation on a database (U3); depends only on model/
  ai/                the whole communication with AI services (U4)
  hardware_tests/    the whole communication with hardware (U5)
  security/          the building blocks of protection: TPM, random numbers, later the keyring
  business/          the business logic, as resources and operations (U2)
  tauri_ui/          the thin layer between business/ and the page (U1)
io-master/ui/        the page (Svelte)
```

### 3.2 The direction of the dependencies

```text
               tauri_ui/
                  │
                  ▼
               business/
          ┌───────┼──────────────┐
          ▼       ▼              ▼
         ai/   hardware_tests/   │
          │       │              │
          └───────┴──────► db/ ◄─┘
                           │
          all of them ───► model/, security/
```

- A folder uses only the folders below it. **No cycle.**
- `tauri_ui/` uses only `business/` (and `model/` to convert forms and views). It never reaches `db/`, `ai/` or `hardware_tests/`.
- `db/` uses only `model/` and `security/` (its encryption).
- `ai/` uses `db/` only for its own data (connections, roles, instructions, journal) and never `business/`.
- **The direction is checked by a test** (`architecture_tests.rs`): it reads the `use crate::...` lines of every file and fails on a forbidden direction. An auditor sees the rule and its check in one place.

### 3.3 `model/`: the data

The types that today live in `test_projects/model.rs`, `evaluation/model.rs`, `standards/model.rs`, `ai/api/connection.rs` (the struct `Connection`, `AnswerMode`), `ai/chat/chats.rs` (`ChatMessage`, `Ending`), `accounts/`, `installation/installation.rs` (`Role`, `Installation`).

- One file per area: `model/standards.rs`, `model/test_projects.rs`, `model/evaluations.rs`, `model/ai.rs`, `model/chats.rs`, `model/accounts.rs`, `model/installation.rs`.
- The rules of validity of a type stay next to it (`ProjectFields::check`, `TileFields::check`), and **the normalization once** (`Fields::normalized()`: text trimmed, documents without repetition). `business/` normalizes before it checks; `db/` writes what it gets (B6).
- The enums of the errors of the rules use enums for the field and the object (`Field::Name`, `Entity::Project`), not text (B5).
- Shared helpers once: `model/text.rs` (`check_text`), `model/time.rs` (`now`) (B1).

### 3.4 `db/`: the databases (U3)

The rule of step 37 stays: one folder per area, `schema.rs`, `rows.rs`, `create.rs`, `read.rs`, `update.rs`, `delete.rs`. Changes:

- **Open once.** The database of the evaluator is opened at the login and kept in the session (3.6). Tables and migrations run only then: `db/user/migrations.rs`, led by `PRAGMA user_version` (B8). `busy_timeout` is set when a connection is opened.
- **One style:** free functions in every area, `db::<area>::<operation>::<object>(db, ...)`; `db/system/` and `db/user/` follow it too (B7).
- **Parameters by name** (`named_params!`, `:title`) and columns read by name; every statement in the function that runs it (B2).
- **Types, not tuples**; `rows.rs` turns every row into a type of `model/` (B3).
- **An unknown value is an error:** `DbError::Corrupt { table, column, value }`; `DbError::Sql` only for SQLite (B4).
- **One transaction per operation**, also in `db/ai/connection/` (A4).
- **Shared pieces once:** `clear_content(tx, project)`; reading and writing tiles with their sources in one place (`db/test_projects/tiles.rs`, used by `db/evaluations/` for the frozen copy) (B1).
- Unique names enforced by the database with `COLLATE NOCASE`, as the code checks them (B7).
- **Areas renamed and split:** `db/ai/memory/` → `db/ai/chats/`; the journal of an evaluation in `db/evaluations/journal/`; `db/ai/presets/` (no SQL) moves to `ai/presets/` (A3).

### 3.5 `ai/`: the integration with AI (U4)

```text
ai/
  api/          the only door to the AI, as resources and operations (step 38):
                connections, roles, tasks, completions, checks
    provider/   HTTP, TLS, request, answer, stream; private to ai/api/
  tasks/        one file per task: the question it builds and how its answer is read
                (chapter.rs, contents.rs, chat.rs; later the phases of the evaluation)
  presets/      the ready settings of known services, and the help for the fields
```

- **`ai/api/connections.rs`**: the AI connections as a resource (`list`, `get`, `create`, `update`, `delete`, `set_key`, `delete_key`, `activate`), so that the window never reaches `db/ai/connection/` (A3).
- **The questions and the reading of the answers of a task move here** from `test_projects/analysis/` and `ai/chat/memory.rs`: the text sent to a model and the parsing of what comes back are the integration with the AI. A task gets plain data of `model/` (the known tiles, the pages) and gives back a typed proposal (`model::ChapterProposal`). Applying a proposal to a project is a business rule and stays in `business/`.
- **The journal is written by `ai/api/` itself**: `completions::ask` asks and records in one operation, so no question can leave the program without its entry. (Today every caller must remember `record`.)
- **The key is checked at its use:** `roles::model` compares the address the key is bound to with the address of the connection (A4).
- `ai/` depends on nothing of `business/`; `ai ↔ test_projects` is gone (A3).

### 3.6 `business/`: the business logic (U2)

**The form of REST, without a server.** A resource is a module; its operations are functions with the names of REST verbs; the head of the file lists them as in `ai/api/`:

```text
//! list()     GET    /projects
//! get()      GET    /projects/{id}
//! create()   POST   /projects
//! update()   PUT    /projects/{id}
//! delete()   DELETE /projects/{id}
//! ready()    POST   /projects/{id}/ready
```

A large resource is a folder whose files group the operations (`create.rs`, `read.rs`, ...), but the names of the functions stay the verbs.

```text
business/
  session/          the logged-in evaluator: the account, the open database, the open
                    projects and evaluations; log in, log out, change the password
  installation/     the first start, the state of the installation (today installation/)
  standards/        documents: list, get, load (with progress), update, delete, page, pdf, search
  test_projects/    projects, schemes, tiles, links, undo, checks, export, import
    analysis/       the long works with AI: the table of contents, the chapters
                    (the loop of today's tauri_ui/scheme_actions.rs::run)
  evaluations/      evaluations, results, proposals, journal
  chats/            chats: list, get, delete, send (with the stream of the reply)
  hardware/         the scan of ports and its settings, as resources over hardware_tests/
  work.rs           what all long works share: Progress, Cancel (a cancellation token)
```

- **The session (A2):** `Session { account, db: UserDb, projects: ProjectWorkspace, evaluations: EvaluationWorkspace }`, made at the login, dropped at the logout (the key of the database wiped). Every operation of a resource takes the session. While a long work waits for the model, it holds no lock: it takes the session for each step only.
- **Long works (A1)** report a `Progress` enum of `business/` through a callback and stop on `Cancel`. The window only forwards them to its channel.
- **Errors**: one enum per resource, with the kinds the page tells apart (not found, invalid with its field, name taken, ...) (B5).
- Accounts (`accounts/`) become a part of `session/` and `installation/`.

### 3.7 `hardware_tests/`: hardware (U5)

```text
hardware_tests/
  devices/       the scan of serial ports (today devices/devices.rs, port_property.rs)
  permissions/   the access rights to the ports (step 6; today an empty placeholder)
  uart/          later
  bluetooth/     later
```

- It reads and writes the hardware only. Its settings (today `devices/scan_settings.rs`, which reads the database) go through `db/` and are offered by `business/hardware/`.
- Rule P2 is kept here: no function of `hardware_tests/` sends to a device without an operation of `business/` started by an action of the evaluator.

### 3.8 `security/`

`security/tpm/` (today `tpm/`), `security/random.rs` (today `random/random.rs`), later `security/keyring/` (step 13). `db/encryption/` stays in `db/` unchanged, as decided in step 37 (the user audits it).

### 3.9 `tauri_ui/`: the thin layer (U1)

```text
tauri_ui/
  commands/      one file per resource of business/; a command only: takes the session,
                 turns the form into a type of model/, calls business/, turns the result
                 into a view, forwards progress to the channel
  serializer/    the views and forms (serde), only here
  state.rs       Option<Session> and the stop signals; nothing else
  error.rs       every conversion of an error of business/ into UiError
```

- The files `*_actions.rs` and `user_data.rs` disappear: their work is in `business/`.
- A command has no `if` of the business: a rule found in a command is a finding.
- **A test checks the four lists of commands** (`main.rs`, `build.rs`, `capabilities/default.json`, `commands/`) against each other (C).

### 3.10 Module files and tests

- **The module tree leaves `main.rs` (B9).** Each folder declares its own modules and their visibility in its own file (`db.rs` next to `db/`, the style of Rust 2018, without `mod.rs`); `main.rs` keeps `mod ai; mod business; mod db; ...`. Names without repetition: `business::session::Account`, not `accounts::accounts::Account`. The whole tree for the overview is in `docs/code_structure.md`.
- **Tests (B10):** one `<file>_tests.rs` per file, as `CLAUDE.md` says; the tests that cover a whole folder (the CRUD of a resource through its operations) are `<folder>/tests.rs`. The rule in `CLAUDE.md` is stated this way.
- **Formatting (B11):** `rustfmt.toml` with `max_width = 120`, one run over the crate.

### 3.11 The page (`ui/`)

- `ui/src/pages/` only the pages of the sidebar; `ui/src/components/` the parts (`TileNode`, `ShadowNode`, the panels, `PdfViewer`).
- `ui/src/lib/api.ts` split by the resources of `business/` (`api/projects.ts`, ...), with one file of the shared types and errors.
- `style.css`: the tokens and the base stay global; the styles of a component go into its `<style>`.

---

## 4. The plan of the work

Each part is one small step: concept approved, implementation, all tests and `clippy` pass, the user reviews, a commit. The program works after every part. Parts that touch a function marked `// ok kk` (today only `main.rs`) need the user's approval of the change first.

| Part | What | Fixes |
|---|---|---|
| 40.1 | The documentation: README, `docs/architecture.md`, `docs/code_structure.md`, `CLAUDE.md`, `todo.md`, `handover.md` (section 5) | U6, D |
| 40.2 | `rustfmt.toml` and one run of `cargo fmt` (touches `main.rs`: approval) | B11 |
| 40.3 | The module tree out of `main.rs` into the files of the folders; names without repetition (touches `main.rs`: approval) | B9 |
| 40.4 | The safety of the AI connections: one transaction per save, the address of the key checked at its use | A4 |
| 40.5 | `model/`: the types moved, normalization once, errors with enums of fields | A3, B5, B6 |
| 40.6 | `db/`: open once, migrations, named parameters, types for rows, `Corrupt`, shared tile code, `COLLATE NOCASE`, renamed areas | A2, B1–B4, B7, B8 |
| 40.7 | `security/` and `hardware_tests/`: the folders moved | U5 |
| 40.8 | `ai/`: `connections`, `tasks/`, `presets/`, the journal written by `ask`; no dependency on the business | U4, A3 |
| 40.9 | `business/`: the session, the resources, the long works with `Progress` and `Cancel` | U2, A1, A2 |
| 40.10 | `tauri_ui/`: commands per resource, the actions removed, all error conversions in `error.rs`, the test of the command lists | U1, C |
| 40.11 | The test of the direction of the dependencies | A3 |
| 40.12 | The page: `components/`, `api/` per resource, the CSS of the components | C |
| 40.13 | The documentation checked against the code once more; `review.md` reset: every file of v0.5 is reviewed from the start | U6 |

**Order of the whole program:** step 40 comes before step 31 (the window of the evaluation), which would otherwise build on the old layout. The review of the code (step 12) goes on, on the new layout.

---

## 5. The new documentation (U6)

| File | For whom | Content |
|---|---|---|
| `README.md` (new) | anyone who opens the repository | what IO Master is today and what it is for; its aims (see the open question 4); the principles P1 to P5; the state (what works, what is planned); the system requirements; build and test; the map of the documents. The README of v0.2 (C, shells, JSON files) is replaced. |
| `docs/architecture.md` (rewritten) | the user, an auditor | *what* and *why*: the roles, the installation, the secrets and the encryption, the AI, the collaboration through git, the work on devices, the test projects and the evaluation. Everything of v0.3 removed: the web UI, the browser, the setup code in the terminal, `--address`, "the server starts". |
| `docs/code_structure.md` (new) | whoever reads or audits the code | *where*: section 3 of this file as it is built, the direction of the dependencies and its test, the conventions (REST form, CRUD, names, errors, progress, tests, formatting), the full module tree, the way of the data (page → command → business → db / ai / hardware). |
| `docs/todo.md` | the user, Claude | the plan; the steps done move to `docs/history.md` so the plan stays short; the stale lines fixed (commit to `v0.4`). |
| `docs/handover.md` | a new session | the state, short; numbers that do not age (no counts of tests). |
| `CLAUDE.md` | Claude | the working rules only; the rules of the code point to `docs/code_structure.md`. |
| `v0.2_functionality.md` | reference | stays as it is. |

---

## 6. Decisions of the user (2026-10-17)

1. **The folders of section 3 are approved**, the business folder named `business/`.
2. **The operations of a resource are functions with the names of REST verbs** (`projects::create`), as in `ai/api/`.
3. **The questions and the answers of the AI tasks are in `ai/tasks/`.**
4. **The README keeps both aims** (the investigation of devices and the support of evaluations), but every list of aims and features is written anew to match the project as it is now; the lists of the README were made at an earlier stage.
5. **No approval moves with the code.** Every new or moved file is reviewed again from the start; `review.md` is reset for the code of v0.5.
6. **Branches:** the state before the cleanup is committed to `v0.4`; all the modified code goes to the new branch **`v0.5`** (on GitHub).
