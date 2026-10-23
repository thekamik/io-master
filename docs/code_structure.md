# IO Master: the structure of the code

This document says **where** everything is in the code and **which rules keep it in order**. *What* the program is and *why* it is built so is in `architecture.md`; the plan of the work in `todo.md`. The structure was set by step 40 of `todo.md` (2026-10-17), on the guidelines of the user:

1. `tauri_ui/` is a minimal intermediate layer that only joins the back end and the front end (renamed `app/` in step 43, 2026-10-23).
2. The business logic is in a folder of its own (`business/`), reached by the interface in the form of a REST API: no server, but the functions are designed as resources and operations.
3. Every operation on a database is in `db/`, ordered as CRUD.
4. All communication with AI is in `ai/`, the intermediary between the external services and the program: one folder for the whole integration, for the audit.
5. Everything that talks to hardware is in `hardware_tests/`.

**Changed on 2026-10-21 (step 42 of `todo.md`, user):** `business/` and the commands of `tauri_ui/` were two layers that did the same thing (one action of the page went through three Rust functions of the same name). They are replaced by the **views** (`views/`, section 2.9), built like the views of Django: one folder per view of the window and one folder per interactive element, whose commands take the parameters from the page, do the work with the folders below and give the data back. The page (`ui/`) stays a separate project with the same names. **Every view is moved** (Scan, the pilot; Start, Shell, Chat, Settings, Evaluation plan, the editor of a catalog): `business/` and the commands of `tauri_ui/` are removed.

---

## 1. The layers

```text
io-master/src/
  main.rs            the map of the program (every folder and its modules, in the order of the
                     layers, each group commented), main(), tauri_startup(); nothing else
  routes.rs          the list of the commands the page may call (as urls.py of Django), above views/
  checks.rs          the macro ensure! (rule 5 of CLAUDE.md), known in the whole program
  model/             the data as plain Rust types and the rules of their validity; no I/O
  db/                every operation on a database, as CRUD; depends only on model/ and security/
  ai/                the whole communication with AI services
  hardware_tests/    the whole communication with hardware
  pdf/               reading PDFs with Poppler (pdfinfo, pdftotext): another program, in one place
  security/          the building blocks of protection: random numbers, the TPM
  installation/      the installation, its one account and the session: the core of the protection
  app/               what the whole window shares: the state, the errors of the commands
  components/        the elements used more than once (by several views, or many times in one)
  views/             one folder per view and per element: its commands and the data it gets
io-master/ui/        the page (Svelte 5), a separate project
```

### 1.1 The direction of the dependencies

```text
                         views/
                  (one folder per view of the window)
                            │
                            ▼
                       components/
                  (the elements used more than once)
        ┌──────────┬────────┼─────────┬──────────────┬───────────┐
        ▼          ▼        ▼         ▼              ▼           ▼
      app/        ai/   hardware_  installation/    pdf/      (model/,
  (state, errors)  │     tests/        │                       security/)
        │          │                   │
        └──────────┴──────► db/ ◄──────┘
                            │
          all of them ───► model/, security/
```

- A folder uses only itself and the folders below it: **no cycle**.
- **The rule is checked by a test**, `src/architecture_tests.rs`: it reads the source of every folder (the tests aside) and fails on a use against the direction. `views/` may use every folder below it, `components/` every folder below `views/` and itself; **a view never uses another view**, by its name or by a chain of `super::` that leaves its folder, and **a component of the page never imports a view of the page** (both checked by the same test). `app/` uses `installation/` and `model/` (the state of the window) and, listed there with their reason, only the errors of the folders below (`AiError`, `ConnectionSaveError`, `ModelError`, the errors of `db/` and of `db/oscal/`, `ScanSettingsError`), which it turns into errors of the page; never a function of them.
- The tests (the module `tests` at the end of a file, `test_support.rs`) are not held to it: they set up data across the layers. The same test checks that the module of tests is the last item of its file, so that no code of the program hides below it.

### 1.2 Where the information is

At every moment it is clear where the information the program works on is (the user, 2026-10-17):

```text
the page (ui/src/views/<view>/<Element>.svelte)   what it shows; no secret, no state that matters
  │  a command of the element (ui/src/lib/api/<view>/<element>.ts), its arguments as JSON
  ▼
views/<view>/<element>/commands.rs   the session taken, the parameters checked, the work done:
  ├──► model/, hardware_tests/       the rules of the data
  ├──► db/                           written through at once, in one transaction
  └──► ai/api/, hardware_tests/      the only ways to a model and to a device
  │
  ▼
views/<view>/<element>/view.rs       the data the element gets back
```

The page never holds a secret. The key of the database of an evaluator opens the database once at the login and is wiped (`installation/session.rs`); the database stays open until the logout.

---

## 2. The folders

### 2.1 `model/`: the data

Plain types of every area, their rules of validity (`check`) and their normal form (`normalized`). **No I/O**: no database, no network, no file. The errors of the rules carry enums of the fields and of the objects (`model/text.rs`: `Field`, `Entity`), never text that is matched.

- **The normal form is the only place where text is trimmed.** A command of a view normalizes what it gets before it checks and stores it; `db/` writes what it gets; so memory and the database hold the same text.

### 2.2 `db/`: the databases

- **SQL lives only here** (the connection of the database of an evaluator is visible only inside `db/`, so SQL elsewhere does not compile).
- One folder per area of data (`system/`, `user/`, `ai/chats/`, `ai/connection/`, `ai/instructions/`, `ai/journal/`, `ai/roles/`, `standards/`, `oscal/`, `evaluation_support/`, `scan_settings/`; the last keeps its two texts in the table `settings` of `user/`, so it has no `schema.rs`), each with `schema.rs` (its tables), `rows.rs` (row ↔ type of `model/`) and the files of CRUD `create.rs`, `read.rs`, `update.rs`, `delete.rs`; a file an area does not need is missing.
- **Free functions everywhere:** `db::<area>::<operation>::<object>(db, ...)`. A caller imports the area as `db_<area>` and calls `db_<area>::<operation>::<object>`, so that every call shows which operation on which data it is.
- **Columns are read by their names; statements with more than one parameter name them** (`:title`); every statement is in the function that runs it.
- **A value the program never writes is an error**, `DbError::Corrupt { table, column, value }`, never replaced in silence. `DbError::Sql` is SQLite only; `NoRow` for a change of a row that does not exist; `Inconsistent` for input that does not hold together.
- **One transaction per operation**; an entry of a journal is written in the transaction of the change it records.
- **Migrations by version**: `db/user/migrations.rs`, led by `PRAGMA user_version`, run once when the database is opened.
- `db/encryption/` holds all code of encryption; it is moved as it was and audited by the user. The head of every area states in which database its data lives and how it is protected.

### 2.3 `ai/`: the communication with AI

```text
ai/api/          the only door to the AI, as resources: connections, roles, tasks, completions, checks
  provider/      HTTP, TLS, request, answer, stream; private to ai/api/
ai/tasks/        per task: the question sent to the model and the reading of its answer
ai/presets/      the ready settings of known services, and the help for the fields
```

- **Every question is in the journal of the AI**: `completions::ask` asks and writes the entry itself, also when the reply was stopped or failed; the caller learns only whether the entry could be written. The check of a connection writes its entry the same way.
- A key is given out only for the address it was entered for (`db/ai/connection/read.rs`, checked at every use).
- A task works on plain data of `model/` and gives back plain data (`ai/tasks/catalog_outline.rs`, `catalog_section.rs`: groups and controls of OSCAL); applying a proposal is the work of the view editor of a catalog (`views/evaluation_plan/catalog_editor/proposals/accept.rs`).

### 2.4 `hardware_tests/`: hardware

The scan of the serial ports (`devices/`) and the settings of the scanner (`devices/scan_settings.rs`: plain data and its rules, no I/O, stored by `db/scan_settings/`); later the access rights (`permissions/`, step 6), UART and Bluetooth. No function here sends to a device without a command started by an action of the evaluator (P2).

### 2.4a `pdf/`: reading PDFs

`poppler.rs`: the number of pages and the title of a PDF (`pdfinfo`), and the text of a range of pages (`pdftotext`), the PDF given on the standard input, never through a shell. It uses nothing of the program; the loading of the documents of a standard (`views/evaluation_plan/standard_documents/load.rs`) reads a document part by part with it, reports every part and can stop between them.

### 2.5 `security/`

`random.rs` (random bytes from the kernel) and `tpm/` (the TPM 2.0, the optional further factor). Later the keyring (step 13).

### 2.5a `installation/`: the installation, the account, the session

The core of the protection (moved from `business/` in step 42, unchanged): the first start and every later start (`lifecycle.rs`), the check before the window opens (`verify.rs`, called by `main.rs`), the account with the keys of its database (`accounts.rs`: Argon2id, the TPM factor), and the session of the logged-in evaluator (`session.rs`: the account and the database opened once at the login). It uses `db/`, `security/` and `model/`; the window (`app/state.rs`) and the views use it.

### 2.6 `business/` (removed in step 42)

Until step 42 the business logic was a folder of its own, as resources and operations named after REST verbs; the commands of `tauri_ui/` called it. Both are replaced by the commands of the elements (section 2.9). Where its parts went: the catalogs and their editor to `views/evaluation_plan/catalog_editor/` (the changes of a catalog in memory to `model/oscal/catalog/`, the new UUIDs and times to `db/oscal/ids.rs`, a new catalog to `views/evaluation_plan/new_catalog/made.rs`), the loading of PDFs to `views/evaluation_plan/standard_documents/`, the chat to `views/chat/conversation/`, the installation and the session to `installation/`.

### 2.7 `app/`: what the whole window shares

Named `tauri_ui/` until step 43 (2026-10-23), its types `UiState` and `UiError` until 43.2: it is no interface, but what every view uses, as the session and the error handling of a web framework.

```text
state.rs        the installation, the session of the evaluator, the stop signals; nothing else
error.rs        every error of a command, in kinds the page tells apart
serializer/     how an error reaches the page (serde)
```

- **No command and no rule is here**: the commands are in the folders of the elements (`views/`).
- A command is callable only when it is listed in **four places**: the `commands.rs` of an element of `views/`, `routes.rs` (`generate_handler!`, used by the window and by the tests of the commands), `build.rs` (`AppManifest`), `capabilities/default.json`. A test (`src/command_lists_tests.rs`) reads the four lists and fails when they differ.

### 2.8 `ui/`: the page

```text
ui/src/views/<view>/  the views: the view and one component per element, named as the folders
                      of src/views/ (Devices.svelte, PortsTable.svelte, ScannerSettings.svelte)
ui/src/lib/api/<view>/<element>.ts
                      the commands of one element as typed functions; the types mirror its view.rs
ui/src/lib/api/errors.ts
                      the error of a command, as app/serializer/error_view.rs sends it
ui/src/components/<component>/
                      the components: one folder each, with its Svelte component and what only
                      it uses (pdf_viewer/: PdfViewer.svelte, and pdf.ts, the code of pdf.js)
ui/src/lib/api/components/<component>.ts
                      the commands of one component, as those of an element
ui/src/views/evaluation_lab/
                      the view Evaluation lab: a placeholder until step 41.5 (no folder in src/views/
                      yet: it has no element and no command)
ui/src/styles/        the style, one sheet per part of the window, in the order of the cascade
                      (style.css lists them)
```

### 2.8a `components/`: the components (step 43)

A component is a small element used more than once: by several views, or many times in one view (`architecture.md`, section 10.2). It is built as an element of a view (`mod.rs`, `commands.rs`, `view.rs`, its tests), its commands are named `<component>_<action>`, and its page is a folder of `ui/src/components/`. The views use it; it never uses a view. The first ones are the viewer of a standard (`pdf_viewer/`), shown by the view Evaluation plan and by its editor of a catalog, and the parts of an object (`part_rows/`), shown twice by the editor of a catalog; a component that only shows data the elements give it has no `commands.rs`.

### 2.9 `views/`: the views of the window (step 42)

```text
views/<view>/mod.rs                  the view: its elements, and the flow of data between them (drawn)
views/<view>/<element>/mod.rs        the element: its actions, their parameters, the data back
views/<view>/<element>/commands.rs   its commands: parameters -> work -> data
views/<view>/<element>/view.rs       the data it gets (serde)
views/<view>/<file>.rs               what the elements of one view share (Devices: stored_settings.rs, column_view.rs)
```

- **An element** is an interactive part of the window that needs the program: to process or store data, to scan, to ask the AI. What is only presentation (an open menu, a selection, a tab, the position during a drag) stays in the page and has no folder here; only the final result is sent (CLAUDE.md, "What goes to Rust").
- **A command is named `<element>_<action>`** (`ports_table_scan`, `scanner_settings_save`) and lives in the folder of its element; a test (`src/command_lists_tests.rs`) checks both.
- **A command does its work with the folders below** (`model/`, `db/`, `ai/`, `hardware_tests/`), never through a layer in between. What two elements need goes down to those folders; what only joins them for one view is a file of the view.
- **The flow between elements goes through the owner of the data:** an element that changed data gets back its own data and tells the view; the element that shows the data reads it again with its own command (Devices: the settings are saved, the table scans again).
- **Commands run off the thread of the window:** `#[tauri::command(async)]` on a plain function runs it on a thread of the runtime of Tauri: the work of milliseconds (the database, the scan) and Argon2id at the login. A work that waits for the model is an `async fn` command that awaits `ai/api/` (the chat, the AI of the editor of a catalog); the reading of PDFs runs on the threads for blocking work (`spawn_blocking`, `views/evaluation_plan/standard_documents/load.rs`).
- **Long works** (the loading of PDFs, the AI of the editor of a catalog, a message of the chat) report their progress as an enum of their own (`LoadEvent`, `BuildEvent`, `ReplyEvent`) through a `Channel` of Tauri, stop on the stop signal of `app/state.rs`, and hold no lock of the session while they wait for files or for the model.
- **Documents of OSCAL** (step 41, `docs/oscal.md`): every change is made on the document read from the database, checked against the schema of NIST and written back at once, in one transaction (`db/oscal/`); the page always gets the document as stored. What the program adds (quotes, proposals of the AI) is the evaluation support, in tables of its own, linked by UUID and id.
- **Tests call a command as the page does**: by name, with JSON, through the IPC of Tauri, in a window of its mock runtime (`views/test_support.rs`, the feature `test` of Tauri), on a real installation in a temporary folder.
- `mod.rs` in the folder of a view and of an element (user, 2026-10-21), so that an element is one folder; the other folders keep `<folder>.rs` next to `<folder>/`.

---

## 3. Conventions

| Topic | Rule |
|---|---|
| Names | No file repeats the name of its folder (`security/tpm/chip.rs`, not `tpm/tpm.rs`). Functions of a resource are verbs. |
| Modules | **`main.rs` is the map of the program** (step 45, user 2026-10-23): every folder and its modules are declared there, in the order of the layers from the bottom up, each group with a comment that says which part of the project it is; a file that would only declare modules does not exist. A file with code that has modules of its own (`model/oscal.rs`) declares them itself. **A plan:** the folders not written in `main.rs` yet still declare their modules in the file of their name (`db.rs` next to `db/`); the views and their elements use `mod.rs` inside their folder (section 2.9) until step 46. |
| Rules of the code | CLAUDE.md, "The rules for Rust" (The Power of 10 adapted). A condition that should never happen is checked with `ensure!(condition, error)` (`checks.rs`), which returns the error; never `assert!` outside the tests. |
| Comments | Code comments and documentation in English, thorough; the head of every file says what it holds and, where it helps, draws the path of the data. |
| Tests | In the file whose functions they test, as its last item: `#[cfg(test)] mod tests { use super::*; ... }` (step 44; a file `*_tests.rs` besides the two tests of the whole program is refused by `architecture_tests.rs`). Help for the tests in `test_support.rs` (of an area), `src/test_support.rs` (temporary folders) and `src/test_support/` (`db.rs`: the data of the tests of `db/`; `standards.rs`: a database with a small standard); a test of a whole area of CRUD is split by the function it checks, so that `read.rs` ends with the tests of reading; the tests of the whole program in `src/architecture_tests.rs` and `src/command_lists_tests.rs`. |
| Formatting | `rustfmt.toml` (120 characters); `cargo fmt --check` must be clean before a commit. |
| Checks | `cargo test` (with the test of the architecture and the test of the command lists), `npm run check` (svelte-check, clean), clippy. |
| Serialization | Only at the boundary to the page: `views/**/view.rs` and `app/serializer/`; the types of the core carry no serde attributes. **The one exception** (user, 2026-10-18, `docs/oscal.md`): the types of `model/oscal/` *are* the published format of NIST and carry serde with the names of OSCAL. |
| Secrets | Only in the encrypted database of the evaluator; never shown back, never printed, never in the page. |

---

## 4. The whole tree

Generated from the heads of the files (the tests left out):

```text
main.rs                      Entry point of IO Master.
routes.rs                  The routes of the window: the list of every command the page may
checks.rs                  The checks of conditions that should never happen (rule 5 of
model/  (main.rs)           The data of the program as plain Rust types, and the rules of
  ai.rs                        The data of the communication with AI services: a connection as
  api_key.rs                   The API key of an AI connection of one evaluator
  chats.rs                     The chats of an evaluator with an AI model, as data: a chat in the
  evaluation_support.rs        The evaluation support (decided by the user, 2026-10-18): what the
  installation.rs              What an installation is: its identifier and its role, chosen at
  oscal.rs/                    The documents of NIST OSCAL (Open Security Controls Assessment
    assessment_plan.rs           An OSCAL assessment plan: the evaluation plan (function a of
    assessment_results.rs        OSCAL assessment results: the evaluation (function b of
    catalog.rs/                  An OSCAL catalog: the requirements (controls) of a standard, in
      edit.rs                      The changes of a catalog in memory: a group or a control added,
      tree.rs                      Where the objects of a catalog are, and the walk through all of them,
    common.rs                    The pieces every model of OSCAL shares: the metadata of a document,
    mapping.rs                   An OSCAL mapping collection (new in OSCAL 1.2.0): relations between
    profile.rs                   An OSCAL profile: a selection of the controls of catalogs (or of
    schema.rs                    The check of an OSCAL document against the JSON schema of NIST, of
    values.rs                    The values OSCAL defines by a pattern, made by the program: a UUID
  standards.rs/                The documents of standards an evaluator has loaded (steps 16 to 22
    quote.rs                     Finding a quote in the text of a page of a standard.
  text.rs                      The rules every text field of the program follows, the error when
  time.rs                      The time stamps of the program: seconds since 1970 (UTC), as the
db/                        Every operation on a database (CLAUDE.md, "Database code").
  ai/                          The data of the AI, in the database of the evaluator.
    chats/                       The chats with the AI.
      create.rs                    Storing chats and their messages. What a chat keeps of a reply
      delete.rs                    Deleting chats.
      read.rs                      Reading chats and their messages.
      rows.rs                      The rows of the chats and their messages, and the Rust types they
      schema.rs                    The tables of the memory of the chat: the conversations with an AI
    connection/                  The AI connections.
      create.rs                    Storing a new AI connection. Its fields are checked by
      delete.rs                    Deleting an AI connection and its API key.
      read.rs                      Reading the AI connections, the active one, and the API key of a
      rows.rs                      The rows of the AI connections and the Rust types they become
      schema.rs                    The table of the AI connections, in the database of an evaluator.
      update.rs                    Changing an AI connection, the active one, and the API key.
    instructions/                The instructions of the tasks of the AI.
      delete.rs                    Removing the instructions of a task: it uses the default ones again.
      read.rs                      Reading the instructions of a task, as the evaluator changed them.
      schema.rs                    The table of the instructions of the tasks of the AI, as the
      update.rs                    Storing the instructions of a task (made or replaced).
    journal/                     The journal of the AI.
      create.rs                    Adding an entry to the journal of the AI (the only way an entry
      read.rs                      Reading the journal of the AI.
      rows.rs                      An entry of the journal of the AI and its row.
      schema.rs                    The journal of the questions to the AI (P3 of `architecture.md`:
    roles/                       The connection each role of the AI uses.
      read.rs                      Reading which connection a role of the AI uses.
      schema.rs                    The table of the roles of the AI, in the database of an evaluator:
      update.rs                    Giving a role of the AI a connection of its own, or taking it away.
  encryption/                  All code of encryption of the databases.
    database_key.rs              The key that encrypts a database of the program.
    generate_key.rs              Generating the key that encrypts the database.
    keys.rs                      The keys of the databases of the evaluators
    sqlcipher.rs                 Opening and creating encrypted databases.
  error.rs                     The error of every function of the database layer (`db/`).
  evaluation_support/          The evaluation support (step 41): what the program adds to the
    create.rs                    Storing quotes and proposals of the evaluation support.
    delete.rs                    Deleting from the evaluation support.
    read.rs                      Reading the evaluation support.
    rows.rs                      The rows of the evaluation support and the types they become
    schema.rs                    The tables of the evaluation support (`model/evaluation_support.rs`),
  oscal/                       The documents of NIST OSCAL (step 41): catalogs, mappings, profiles,
    create.rs                    Storing a new document of OSCAL, checked against the schema of NIST.
    delete.rs                    Deleting a document of OSCAL. Its text is overwritten in the file,
    error.rs                     The error of an operation on a stored document of OSCAL: the rules of
    ids.rs                       New UUIDs and the time of a change, for the documents of OSCAL the
    read.rs                      Reading the documents of OSCAL.
    rows.rs                      The rows of the documents of OSCAL and the types they become
    schema.rs                    The table of the documents of OSCAL, in the database of an
    update.rs                    Replacing a document of OSCAL by its new version, checked against
  scan_settings.rs/            The settings of the scanner of serial ports, as two texts
    read.rs                      Reading the settings of the scanner.
    update.rs                    Storing the settings of the scanner.
  standards/                   The documents of standards.
    create.rs                    Storing a document of a standard: the file and the text of its
    delete.rs                    Removing a document of a standard. Whether a project still uses it
    read.rs                      Reading the documents of standards, their pages and PDFs.
    rows.rs                      The rows of the documents of standards and the Rust type they
    schema.rs                    The tables of the documents of standards, in the database of an
    update.rs                    Changing what the user told about a document of a standard.
  system/                      The system database (not encrypted).
    create.rs                    Creating rows of the system database: the record of the
    database.rs                  The system database: `system.db` in the data directory.
    delete.rs                    Removing rows of the system database: an account. (Removing the
    read.rs                      Reading the system database: the installation and the accounts.
    rows.rs                      The rows of the system database and the Rust types they become.
    schema.rs                    The tables of the system database (`database.rs`). They are made
    update.rs                    Changing rows of the system database: the key of an account.
  user/                        The database of one evaluator (encrypted).
    database.rs                  The database of one evaluator: `users/<id>.db` in the data
    delete.rs                    Removing settings and secrets of an evaluator.
    migrations.rs                The changes of the layout of the database of an evaluator, by its
    read.rs                      Reading the settings and secrets of an evaluator.
    schema.rs                    The tables of the database of an evaluator that belong to no other
    update.rs                    Storing the settings and secrets of an evaluator. A value is made
ai/                        The communication with AI services (CLAUDE.md, "AI layer").
  api/                         The only way to the AI, in the form of REST: resources and operations.
    checks.rs                    The check of an AI connection: one short question, and a report of
    completions.rs               Questions to the AI model: the one way the rest of the program asks
    connection.rs                A connection to an AI service, as the evaluator sets it up on the
    connections.rs               The AI connections of the evaluator, as a resource (step 40): the
    provider/                    The communication with the provider: HTTP, TLS, request, answer, stream.
      ai_error.rs                  Why a request to the AI service failed, in kinds that a front
      client.rs                    The connection to the AI service of a connection
      json_path.rs                 Paths into a JSON answer: how the evaluator tells the program
      request.rs                   The request to the AI service, made from the template of a
      response.rs                  Reading the answer of the AI service with the paths of its
      sse.rs                       Reading a stream of server-sent events (SSE), the format in
    roles.rs                     The roles of the AI (user, 2026-10-17): the AI acts in several
    tasks.rs                     The tasks of the AI: what a question is for. Every task is asked
  presets.rs/                  Presets: ready settings of well-known AI services, to fill the
    field_help.rs                Instructions for a chatbot: help to fill one field of an AI
  tasks/                       The tasks of the AI: for each, the question sent to the model and
    catalog_outline.rs           The task `catalog_outline` of the AI (role *text analysis*): the
    catalog_section.rs           The task `catalog_section` of the AI (role *text analysis*): the
    chat.rs                      The task `chat` of the AI (role *consultation*): the memory of a
    json.rs                      The JSON object in an answer of a model, and its texts.
hardware_tests.rs/         Everything that talks to hardware (step 40 of docs/todo.md).
  devices.rs/                  The scan of the serial ports, and the settings of the scanner.
    port_property.rs             The properties of a serial port that can be shown, one per
    scan_settings.rs             The settings of the scanner: which kinds of port a scan looks for,
    serial_ports.rs              Discovery of serial ports.
pdf.rs/                    Reading PDFs with the tools of Poppler (`pdfinfo`, `pdftotext`): the
  poppler.rs                   Reading the text of a PDF, page by page, with the tools of
installation.rs/           The installation of this Linux account, its one user and the
  accounts.rs                  The accounts of the evaluators: creating them, logging in, and
  lifecycle.rs                 The life of an installation: the first start, every later
  session.rs                   The logged-in evaluator (step 40): their account, their database,
  uninstall.rs                 (not compiled: "DO NOT USE: TO BE REWORKED", step 12)
  verify.rs                    The check of the installation at the start of the program.
security/                  The building blocks of protection: random numbers from the system,
  random.rs                    Random bytes from the kernel.
  tpm/                         The TPM 2.0, the optional further factor (`docs/architecture.md`,
    chip.rs                      Talking to the TPM 2.0.
    device_secret.rs             The secret of the device (`docs/architecture.md`, section 5.2).
    emulator.rs                  The TPM 2.0 emulator used until the beta version.
app.rs/                    What the whole window shares (Tauri 2): the state the commands of the
  error.rs                     The errors the commands give back to the page, in kinds the page
  serializer.rs/               How an error of a command reaches the page (serde). The data of the
    error_view.rs                How an error of a command reaches the page:
  state.rs                     The state of the desktop interface, kept in Rust and never in
components.rs/             The components of the window: small elements used more than once,
  part_rows/                   The component "parts of an object": the parts of a group or a control
    view.rs                      The data of the component "parts of an object": the parts of a group or
  pdf_viewer/                  The component "viewer of a standard": the original page of a PDF, with a
    commands.rs                  The command of the viewer of a standard.
    view.rs                      The data of the viewer, as the page gets it: the documents it can
views.rs/                  The views of the window, one folder per view and one folder per
  chat/                        The view Chat: the chats of the evaluator with the model of the role
    chat_list/                   The element "list of chats" of the view Chat
      commands.rs                  The command of the list of chats.
      view.rs                      The data of the list of chats, as the page gets it.
    connection_line/             The element "connection line" of the view Chat: which AI connection
      commands.rs                  The command of the connection line.
      view.rs                      The data of the connection line, as the page gets it.
    conversation/                The element "conversation" of the view Chat: the open chat, a new
      commands.rs                  The commands of the conversation.
      stored.rs                    What a chat keeps (`db/ai/chats/`, tables `chats` and `messages`, in
      turn.rs                      One turn of a chat: the message of the evaluator is checked and
      view.rs                      The data of the conversation as the page gets it: an open chat, and
  devices/                     The view Devices: the serial ports of the computer and the settings of
    column_view.rs               A column of the table of ports, as both elements of the view show
    ports_table/                 The element "table of ports" of the view Devices
      commands.rs                  The commands of the table of ports.
      view.rs                      The data of the table of ports, as the page gets it: a ready table
    scanner_settings/            The element "scanner settings" of the view Devices
      commands.rs                  The commands of the settings of the scanner.
      view.rs                      The data of the settings of the scanner, as the page gets it: the
    stored_settings.rs           The settings of the scanner as they are stored: read by both
  evaluation_plan/             The view Evaluation plan: the documents of the standards in NIST OSCAL
    catalog_editor/              The editor of a catalog, a part of the view Evaluation plan: a catalog
      ai_builder/                  The element "the AI" of the editor of a catalog: the AI reads a table
        commands.rs                  The commands of the AI of the editor.
        controls.rs                  The controls of a group of a catalog proposed by the AI from the pages
        groups.rs                    The groups of a catalog proposed by the AI from a table of contents
        parts.rs                     The pages of a standard cut into parts that fit the context of a
        steps.rs                     The steps both works of the AI share: the model and the instructions
        view.rs                      The data of the AI of the editor, as the page gets and sends it, and
      ai_instructions/             The element "instructions to the AI" of the editor of a catalog: the
        commands.rs                  The commands of the instructions to the AI.
      catalog_tree/                The element "tree" of the editor of a catalog: its groups and controls
        commands.rs                  The command of the tree.
        view.rs                      The data of the tree, as the page gets it.
      checked_quote.rs             A quote of a document of a standard, looked for on the page it names
      new_object/                  The element "new object" of the editor of a catalog: a dialog that adds
        commands.rs                  The commands of a new object.
        view.rs                      The place of a new object, as the page sends it.
      object_form/                 The element "object" of the editor of a catalog: the group or control
        commands.rs                  The commands of the object chosen.
        view.rs                      The data of the object chosen, as the page gets it.
      object_quotes/               The element "quotes" of the editor of a catalog: the passages of the
        commands.rs                  The commands of the quotes of an object.
        view.rs                      The data of the quotes of an object, as the page gets it.
      oscal_object.rs              A group or a control of OSCAL sent by the page as JSON, read into the
      proposals/                   The element "proposals" of the editor of a catalog: what the AI
        accept.rs                    A proposal of the AI accepted into its catalog.
        commands.rs                  The commands of the proposals.
        view.rs                      The data of the proposals, as the page gets it.
    document_search/             The element "search" of the view Evaluation plan: the text of the
      commands.rs                  The command of the search.
      search.rs                    The search of the text of the documents of standards: in the one form
      view.rs                      The data of the search, as the page gets it.
    new_catalog/                 The element "new catalog" of the view Evaluation plan: a catalog of
      commands.rs                  The command of a new catalog.
      made.rs                      A new, empty catalog of OSCAL: its metadata, and the documents of the
      view.rs                      The data of a new catalog, as the page gets it: the documents it can
    oscal_documents/             The element "documents of OSCAL" of the view Evaluation plan: the
      commands.rs                  The commands of the documents of OSCAL. The files are chosen in the
      file.rs                      A file of OSCAL (JSON) read into the database, and written from it:
      view.rs                      The data of the list of documents of OSCAL, as the page gets it.
    standard_documents/          The element "documents of standards" of the view Evaluation plan: the
      commands.rs                  The commands of the documents of standards.
      load.rs                      The loading of documents of standards from PDF files: a long work.
      stored.rs                    What is stored of a document of a standard, with its rules: a file is
      view.rs                      The data of the list of documents of standards, as the page gets and
  settings/                    The view Settings: the password of the evaluator, the connections to
    api_key/                     The element "API key" of the view Settings, tab AI: the key of one
      commands.rs                  The commands of the API key.
      view.rs                      The data of the API key, as the page gets it: never the key itself.
    connection_check/            The element "check of a connection" of the view Settings, tab AI
      commands.rs                  The command of the check of a connection.
      view.rs                      The report of a check of a connection, as the page gets it; the key
    connection_form/             The element "connection form" of the view Settings, tab AI: one
      commands.rs                  The commands of the connection form.
      view.rs                      The data of the connection form, as the page gets it and sends it.
    connection_list/             The element "list of connections" of the view Settings, tab AI
      commands.rs                  The command of the list of connections.
      view.rs                      The data of the list of connections, as the page gets it.
    password_form/               The element "password form" of the view Settings, tab User
      commands.rs                  The command of the password form.
  shell/                       The view Shell: the frame of the window around every page of a
    program/                     The element "program" of the view Shell: the state of the program the
      commands.rs                  The command of the state of the program.
      view.rs                      The state of the program as the page gets it: whether the program
    user_menu/                   The element "user menu" of the view Shell: the logged-in evaluator at
      commands.rs                  The command of the user menu.
  start/                       The view Start: what the window shows before anybody is logged in
    login_form/                  The element "login form" of the view Start
      commands.rs                  The command of the login form.
    setup_form/                  The element "setup form" of the view Start
      commands.rs                  The command of the setup form.
```
