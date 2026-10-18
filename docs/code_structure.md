# IO Master: the structure of the code

This document says **where** everything is in the code and **which rules keep it in order**. *What* the program is and *why* it is built so is in `architecture.md`; the plan of the work in `todo.md`. The structure was set by step 40 of `todo.md` (2026-10-17, on the branch `v0.5`), on the guidelines of the user:

1. `tauri_ui/` is a minimal intermediate layer that only joins the back end and the front end.
2. The business logic is in a folder of its own (`business/`), reached by the interface in the form of a REST API: no server, but the functions are designed as resources and operations.
3. Every operation on a database is in `db/`, ordered as CRUD.
4. All communication with AI is in `ai/`, the intermediary between the external services and the program: one folder for the whole integration, for the audit.
5. Everything that talks to hardware is in `hardware_tests/`.

---

## 1. The layers

```text
io-master/src/
  main.rs            the list of the folders, main(), tauri_startup(); nothing else
  model/             the data as plain Rust types and the rules of their validity; no I/O
  db/                every operation on a database, as CRUD; depends only on model/ and security/
  ai/                the whole communication with AI services
  hardware_tests/    the whole communication with hardware
  security/          the building blocks of protection: random numbers, the TPM
  business/          the business logic, as resources and operations
  tauri_ui/          the thin layer between business/ and the page
io-master/ui/        the page (Svelte 5)
```

### 1.1 The direction of the dependencies

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

- A folder uses only itself and the folders below it: **no cycle**.
- **The rule is checked by a test**, `src/architecture_tests.rs`: it reads the source of every folder (the tests aside) and fails on a use against the direction. The few uses of the window below `business/` are listed there with their reason: the resources of the AI settings in `ai/api/` (connections, roles, checks) and the presets, the type of the open database (`UserDb`) and the error of a database, the types of a port.
- The tests (`*_tests.rs`, `test_support.rs`) are not held to it: they set up data across the layers.

### 1.2 Where the information is

At every moment it is clear where the information the program works on is (the user, 2026-10-17):

```text
the page (ui/)             what it shows; no secret, no state that matters
  │  a command (tauri_ui/commands.rs), its arguments as JSON
  ▼
tauri_ui/commands/         the form becomes a type of model/; the session is taken
  │
  ▼
business/                  the rules; the objects in memory of the session:
  │                        Session (account, open database)
  ├──► db/                 written through at once (write-through), in one transaction
  ├──► ai/api/             the only way to a model; every question in the journal of the AI
  └──► hardware_tests/     the only way to a device
```

The page never holds a secret. The key of the database of an evaluator opens the database once at the login and is wiped (`business/session.rs`); the database stays open until the logout.

---

## 2. The folders

### 2.1 `model/`: the data

Plain types of every area, their rules of validity (`check`) and their normal form (`normalized`). **No I/O**: no database, no network, no file. The errors of the rules carry enums of the fields and of the objects (`model/text.rs`: `Field`, `Entity`), never text that is matched.

- **The normal form is the only place where text is trimmed.** The business normalizes what it gets before it checks and stores it; `db/` writes what it gets; so memory and the database hold the same text.

### 2.2 `db/`: the databases

- **SQL lives only here** (the connection of the database of an evaluator is visible only inside `db/`, so SQL elsewhere does not compile).
- One folder per area of data (`system/`, `user/`, `ai/chats/`, `ai/connection/`, `ai/instructions/`, `ai/journal/`, `ai/roles/`, `standards/`, `oscal/`, `evaluation_support/`), each with `schema.rs` (its tables), `rows.rs` (row ↔ type of `model/`) and the files of CRUD `create.rs`, `read.rs`, `update.rs`, `delete.rs`; a file an area does not need is missing.
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
- A task works on plain data of `model/` and gives back plain data (`ai/tasks/catalog_outline.rs`, `catalog_section.rs`: groups and controls of OSCAL); applying a proposal is the work of the business.

### 2.4 `hardware_tests/`: hardware

The scan of the serial ports (`devices/`); later the access rights (`permissions/`, step 6), UART and Bluetooth. No function here sends to a device without an operation of `business/` started by an action of the evaluator (P2).

### 2.5 `security/`

`random.rs` (random bytes from the kernel) and `tpm/` (the TPM 2.0, the optional further factor). Later the keyring (step 13).

### 2.6 `business/`: the business logic

**The form of REST, without a server.** A resource is a module; its operations are functions named after REST verbs, listed with their REST form at the head of the file:

```text
//! create()           POST    /catalogs                         a new, empty catalog
//! get()              GET     /catalogs/{uuid}
//! add_group()        POST    /catalogs/{uuid}/groups           at the top or in a group
//! update_group()     PUT     /catalogs/{uuid}/groups/{id}      its own fields; its content stays
//! delete_object()    DELETE  /catalogs/{uuid}/objects/{id}     a group or a control, with its content
```

- **The session** (`session.rs`): the account and the database opened once at the login, behind a lock.
- **Documents of OSCAL** (`oscal/`, step 41, `docs/oscal.md`): every change is made on the document read from the database, checked against the schema of NIST and written back at once, in one transaction; the page always gets the document as stored. What the program adds (quotes, proposals of the AI) is the evaluation support, in tables of its own, linked by UUID and id.
- **Long works** (`work.rs`): the loading of PDFs (`standards/load.rs`), the building of a catalog with the AI (`oscal/building.rs`), a message of the chat (`chats/conversation.rs`). Each reports its progress as an enum of its own (`Progress`, `LoadProgress`, `ChatEvent`) to a function of the caller, stops on `Cancel`, and holds no lock of the session while it waits for files or for the model.
- **Errors**: one enum per area (`OscalError`, `DocumentError`, `WorkError`, ...), with the error of the rules of `model/` inside.

### 2.7 `tauri_ui/`: the window

```text
commands.rs     every command the page may call: the whole surface, in one file
commands/       per resource: the session, the form into a type of model/, the call of
                business/, the result into a view, the progress into the events of the page
serializer/     the views and forms (serde): the only place of serialization
session.rs      the session of the logged-in evaluator, for a command
state.rs        the session and the stop signals; nothing else
error.rs        every error of the business as an error of the page
```

- **No rule of the business is here**: a rule found in a command is a finding of the review.
- A command is callable only when it is listed in **four places**: `commands.rs`, `main.rs` (`generate_handler!`), `build.rs` (`AppManifest`), `capabilities/default.json`. A test (`commands_tests.rs`) reads the four lists and fails when they differ.

### 2.8 `ui/`: the page

```text
ui/src/pages/         the pages: the sidebar (Chat, Evaluation plan, Evaluation, Documentation,
                      Scan, Settings), the frame (Shell), Login, Setup
ui/src/components/    their parts: the tab AI, the editor of a catalog, the viewer of a standard
ui/src/lib/api/       the commands as typed functions, one file per resource of the program
                      (errors, app, ai, scan, chat, documents, oscal, catalogs); the types mirror
                      the views of tauri_ui/serializer/
ui/src/lib/pdf.ts     the viewer of a standard (pdf.js)
ui/src/styles/        the style, one sheet per part of the window, in the order of the cascade
                      (style.css lists them)
```

---

## 3. Conventions

| Topic | Rule |
|---|---|
| Names | No file repeats the name of its folder (`security/tpm/chip.rs`, not `tpm/tpm.rs`). Functions of a resource are verbs. |
| Modules | Every folder declares its modules in the file of the same name next to it (`db.rs` next to `db/`); `main.rs` lists only the top folders, so a new file never changes it. |
| Comments | Code comments and documentation in English, thorough; the head of every file says what it holds and, where it helps, draws the path of the data. |
| Tests | `<file>_tests.rs` next to the file, attached with `#[cfg(test)] #[path = "<file>_tests.rs"] mod tests;`. A file of tests that covers a whole resource is named after the resource. Help for the tests in `test_support.rs` (of an area) and `src/test_support.rs` (temporary folders). |
| Formatting | `rustfmt.toml` (120 characters); `cargo fmt --check` must be clean before a commit. |
| Checks | `cargo test` (with the test of the architecture and the test of the command lists), `npm run check` (svelte-check, clean), clippy. |
| Serialization | Only in `tauri_ui/serializer/`; the types of the core carry no serde attributes. **The one exception** (user, 2026-10-18, `docs/oscal.md`): the types of `model/oscal/` *are* the published format of NIST and carry serde with the names of OSCAL. |
| Secrets | Only in the encrypted database of the evaluator; never shown back, never printed, never in the page. |

---

## 4. The whole tree

Generated from the heads of the files (the tests left out):

```text
main.rs                      Entry point of IO Master.
model.rs/                  The data of the program as plain Rust types, and the rules of
  ai.rs                        The data of the communication with AI services: a connection as
  api_key.rs                   The API key of an AI connection of one evaluator
  chats.rs                     The chats of an evaluator with an AI model, as data: a chat in the
  evaluation_support.rs        The evaluation support (decided by the user, 2026-10-18): what the
  installation.rs              What an installation is: its identifier and its role, chosen at
  oscal.rs/                    The documents of NIST OSCAL (Open Security Controls Assessment
    assessment_plan.rs           An OSCAL assessment plan: the evaluation plan (function a of
    assessment_results.rs        OSCAL assessment results: the evaluation (function b of
    catalog.rs                   An OSCAL catalog: the requirements (controls) of a standard, in
    common.rs                    The pieces every model of OSCAL shares: the metadata of a document,
    mapping.rs                   An OSCAL mapping collection (new in OSCAL 1.2.0): relations between
    profile.rs                   An OSCAL profile: a selection of the controls of catalogs (or of
    schema.rs                    The check of an OSCAL document against the JSON schema of NIST, of
    values.rs                    The values OSCAL defines by a pattern, made by the program: a UUID
  standards.rs                 The documents of standards an evaluator has loaded (steps 16 to 22
  text.rs                      The rules every text field of the program follows, the error when
  time.rs                      The time stamps of the program: seconds since 1970 (UTC), as the
db.rs/                     Every operation on a database (CLAUDE.md, "Database code").
  ai.rs/                       The data of the AI, in the database of the evaluator.
    chats.rs/                    The chats with the AI.
      create.rs                    Storing chats and their messages. What a chat keeps of a reply
      delete.rs                    Deleting chats.
      read.rs                      Reading chats and their messages.
      rows.rs                      The rows of the chats and their messages, and the Rust types they
      schema.rs                    The tables of the memory of the chat: the conversations with an AI
    connection.rs/               The AI connections.
      create.rs                    Storing a new AI connection. Its fields are checked by
      delete.rs                    Deleting an AI connection and its API key.
      read.rs                      Reading the AI connections, the active one, and the API key of a
      rows.rs                      The rows of the AI connections and the Rust types they become
      schema.rs                    The table of the AI connections, in the database of an evaluator.
      update.rs                    Changing an AI connection, the active one, and the API key.
    instructions.rs/             The instructions of the tasks of the AI.
      delete.rs                    Removing the instructions of a task: it uses the default ones again.
      read.rs                      Reading the instructions of a task, as the evaluator changed them.
      schema.rs                    The table of the instructions of the tasks of the AI, as the
      update.rs                    Storing the instructions of a task (made or replaced).
    journal.rs/                  The journal of the AI.
      create.rs                    Adding an entry to the journal of the AI (the only way an entry
      read.rs                      Reading the journal of the AI.
      rows.rs                      An entry of the journal of the AI and its row.
      schema.rs                    The journal of the questions to the AI (P3 of `architecture.md`:
    roles.rs/                    The connection each role of the AI uses.
      read.rs                      Reading which connection a role of the AI uses.
      schema.rs                    The table of the roles of the AI, in the database of an evaluator:
      update.rs                    Giving a role of the AI a connection of its own, or taking it away.
  encryption.rs/               All code of encryption of the databases.
    database_key.rs              The key that encrypts a database of the program.
    generate_key.rs              Generating the key that encrypts the database.
    keys.rs                      The keys of the databases of the evaluators
    sqlcipher.rs                 Opening and creating encrypted databases.
  error.rs                     The error of every function of the database layer (`db/`).
  evaluation_support.rs/       The evaluation support (step 41): what the program adds to the
    create.rs                    Storing quotes and proposals of the evaluation support.
    delete.rs                    Deleting from the evaluation support.
    read.rs                      Reading the evaluation support.
    rows.rs                      The rows of the evaluation support and the types they become
    schema.rs                    The tables of the evaluation support (`model/evaluation_support.rs`),
  oscal.rs/                    The documents of NIST OSCAL (step 41): catalogs, mappings, profiles,
    create.rs                    Storing a new document of OSCAL, checked against the schema of NIST.
    delete.rs                    Deleting a document of OSCAL. Its text is overwritten in the file,
    read.rs                      Reading the documents of OSCAL.
    rows.rs                      The rows of the documents of OSCAL and the types they become
    schema.rs                    The table of the documents of OSCAL, in the database of an
    update.rs                    Replacing a document of OSCAL by its new version, checked against
  standards.rs/                The documents of standards.
    create.rs                    Storing a document of a standard: the file and the text of its
    delete.rs                    Removing a document of a standard. Whether a project still uses it
    read.rs                      Reading the documents of standards, their pages and PDFs.
    rows.rs                      The rows of the documents of standards and the Rust type they
    schema.rs                    The tables of the documents of standards, in the database of an
    update.rs                    Changing what the user told about a document of a standard.
  system.rs/                   The system database (not encrypted).
    create.rs                    Creating rows of the system database: the record of the
    database.rs                  The system database: `system.db` in the data directory.
    delete.rs                    Removing rows of the system database: an account. (Removing the
    read.rs                      Reading the system database: the installation and the accounts.
    rows.rs                      The rows of the system database and the Rust types they become.
    schema.rs                    The tables of the system database (`database.rs`). They are made
    update.rs                    Changing rows of the system database: the key of an account.
  user.rs/                     The database of one evaluator (encrypted).
    database.rs                  The database of one evaluator: `users/<id>.db` in the data
    delete.rs                    Removing settings and secrets of an evaluator.
    migrations.rs                The changes of the layout of the database of an evaluator, by its
    read.rs                      Reading the settings and secrets of an evaluator.
    schema.rs                    The tables of the database of an evaluator that belong to no other
    update.rs                    Storing the settings and secrets of an evaluator. A value is made
ai.rs/                     The communication with AI services (CLAUDE.md, "AI layer").
  api.rs/                      The only way to the AI, in the form of REST: resources and operations.
    checks.rs                    The check of an AI connection: one short question, and a report of
    completions.rs               Questions to the AI model: the one way the rest of the program asks
    connection.rs                A connection to an AI service, as the evaluator sets it up on the
    connections.rs               The AI connections of the evaluator, as a resource (step 40): the
    provider.rs/                 The communication with the provider: HTTP, TLS, request, answer, stream.
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
  tasks.rs/                    The tasks of the AI: for each, the question sent to the model and
    catalog_outline.rs           The task `catalog_outline` of the AI (role *text analysis*): the
    catalog_section.rs           The task `catalog_section` of the AI (role *text analysis*): the
    chat.rs                      The task `chat` of the AI (role *consultation*): the memory of a
    json.rs                      The JSON object in an answer of a model, and its texts.
hardware_tests.rs/         Everything that talks to hardware (step 40 of docs/todo.md).
  devices.rs/                  The scan of the serial ports.
    port_property.rs             The properties of a serial port that can be shown, one per
    serial_ports.rs              Discovery of serial ports.
security.rs/               The building blocks of protection: random numbers from the system,
  random.rs                    Random bytes from the kernel.
  tpm.rs/                      The TPM 2.0, the optional further factor (`docs/architecture.md`,
    chip.rs                      Talking to the TPM 2.0.
    device_secret.rs             The secret of the device (`docs/architecture.md`, section 5.2).
    emulator.rs                  The TPM 2.0 emulator used until the beta version.
business.rs/               The business logic of the program (step 40 of docs/todo.md): what
  chats.rs/                    The chats of the evaluator with the AI: the stored chats and one
    conversation.rs              One turn of a chat, without any user interface ([`send`]): the
    history.rs                   The chats of an evaluator with an AI model, kept in their
  hardware.rs/                 The work on hardware as the evaluator sees it: the scan of the
    ports.rs                     The serial ports as a resource (step 5 of `docs/todo.md`, moved in
    scan_settings.rs             The settings of the scanner of an evaluator: which kinds of port
  installation.rs/             The installation: its life (first start, removal) and its check.
    lifecycle.rs                 The life of an installation: the first start, every later
    uninstall.rs                 (not compiled: "DO NOT USE: TO BE REWORKED", step 12)
    verify.rs                    The check of the installation at the start of the program.
  oscal.rs/                    The documents of NIST OSCAL of the evaluator (step 41,
    building.rs                  The long works that make a catalog of OSCAL from the documents of a
    catalogs.rs                  The catalogs of the evaluator as a resource (step 41.3): a standard
    documents.rs                 The documents of OSCAL as a resource (step 41.2): loaded from files
    ids.rs                       New UUIDs and the time of a change, for the documents the program
    proposals.rs                 The proposals of the AI for an OSCAL document (evaluation support,
    quotes.rs                    The passages of the documents of a standard (PDF) the objects of an
  session.rs/                  The logged-in evaluator (step 40): their account, their database,
    accounts.rs                  The accounts of the evaluators: creating them, logging in, and
  standards.rs/                The documents of standards: loading, reading, searching, quotes.
    documents.rs                 The documents of standards as a resource (steps 16 to 22 of
    error.rs                     The error of an operation on a document of a standard.
    load.rs                      The loading of documents of standards from PDF files: a long work
    parts.rs                     The pages of a standard cut into parts that fit the context of a
    pdf_text.rs                  Reading the text of a PDF, page by page, with the tools of
    quote.rs                     Finding a quote in the text of a page of a standard.
  work.rs                      What the long works of the business share (step 40): the stop
tauri_ui.rs/               The desktop window (Tauri 2), the thin layer between the business
  blocking.rs                  Blocking work of the commands, kept off the threads that keep
  commands.rs/                 The commands the page can call: the whole surface between the
    account.rs                   The first start, the login, the logout and the password.
    ai.rs                        The tab AI of Settings: the AI connections of the evaluator, their
    catalogs.rs                  The catalogs of OSCAL in the tool Evaluation plan (step 41.3): made
    chats.rs                     The chats: the list, an open chat, deleting, and a message with
    documents.rs                 The documents of standards in the tool Evaluation plan:
    oscal.rs                     The documents of OSCAL in the tool Evaluation plan (step 41.2): the
    scan.rs                      The scan of serial ports with the settings of the evaluator
  error.rs                     The errors the commands give back to the page, in kinds the page
  serializer.rs/               The views and forms of the page (serde): the only place of serialization.
    ai_config_view.rs            The AI connections as the tab AI of Settings shows and sends them,
    app_view.rs                  What the page needs to know to show itself: whether the program
    catalog_view.rs              A catalog of OSCAL as the editor of the page shows it: the catalog
    chat_view.rs                 The chats as the page shows them, and the events of a reply
    error_view.rs                How an error of a command reaches the page:
    oscal_view.rs                The documents of OSCAL as the page shows them in a list.
    scan_view.rs                 The scan page as the page shows it: the settings of the scanner,
    standard_view.rs             The documents of standards as the page shows them, and the events
  session.rs                   The session of the logged-in evaluator (`business/session.rs`), for
  state.rs                     The state of the desktop interface, kept in Rust and never in
```
