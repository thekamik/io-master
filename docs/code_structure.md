# IO Master: the structure of the code

This document says **where** everything is in the code and **which rules keep it in order**. *What* the program is and *why* it is built so is in `architecture.md`; the open work in `todo.md`, the steps built in `history.md`. The structure was set by step 40 (2026-10-17) on the guidelines of the user, and changed by step 42 (2026-10-21: the views instead of a layer of business logic and a layer of commands) and step 43 (2026-10-23: one vocabulary, the components):

1. `app/` is a minimal layer that only joins the back end and the front end: what every view uses, as the session and the error handling of a web framework.
2. The work of the window is done by the **views** (`views/`, section 2.9), built like the views of Django: one folder per view and one per interactive element, whose commands take the parameters from the page, do the work with the folders below and give the data back.
3. Every operation on a database is in `db/`, ordered as CRUD.
4. All communication with AI is in `ai/`, the intermediary between the external services and the program: one folder for the whole integration, for the audit.

---

## 1. The layers

```text
io-master/src/
  main.rs            the map of the program (every folder and its modules, in the order of the
                     layers, each group commented), main(), tauri_startup(); nothing else
  routes.rs          the list of the commands the page may call (as urls.py of Django), above views/
  checks.rs          the macro ensure! (rule 5 of CLAUDE.md), known in the whole program
  model/             the new models (2026-10-28): the data as it is kept in the databases
  model_old/         the earlier models: the data as plain Rust types and the rules of their
                     validity; no I/O; replaced by model/ step by step
  db/                every operation on a database, as CRUD; depends only on model/, model_old/
                     and security/
  ai/                the whole communication with AI services
  pdf/               reading PDFs with Poppler (pdfinfo, pdftotext): another program, in one place
  security/          the building blocks of protection: random numbers, no memory dumps
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
        ┌──────────┬────────┼──────────────┬───────────┐
        ▼          ▼        ▼              ▼           ▼
      app/        ai/  installation/      pdf/      (model/,
  (state, errors)  │        │                       security/)
        │          │        │
        └──────────┴──► db/ ◄┘
                            │
          all of them ───► model/, security/
```

- A folder uses only itself and the folders below it: **no cycle**.
- **The rule is checked by a test**, `src/architecture_tests.rs`: it reads the source of every folder (the tests aside) and fails on a use against the direction. `views/` may use every folder below it, `components/` every folder below `views/` and itself; **a view never uses another view**, by its name or by a chain of `super::` that leaves its folder, and **a component of the page never imports a view of the page** (both checked by the same test). `app/` uses `installation/` and `model/` (the state of the window) and, listed there with their reason, only the errors of the folders below (`AiError`, `ConnectionSaveError`, `ModelError`, the errors of `db/` and of `db/oscal/`), which it turns into errors of the page; never a function of them.
- The tests (the module `tests` at the end of a file, `test_support.rs`) are not held to it: they set up data across the layers. The same test checks that the module of tests is the last item of its file, so that no code of the program hides below it.

### 1.2 Where the information is

At every moment it is clear where the information the program works on is (the user, 2026-10-17):

```text
the page (ui/src/views/<view>/<Element>.svelte)   what it shows; no secret, no state that matters
  │  a command of the element (ui/src/lib/api/<view>.ts), its arguments as JSON
  ▼
views/<view>/<view>.rs               the entry of the view: its command, the parameters checked, the work done:
  ├──► model/                        the rules of the data
  ├──► db/                           written through at once, in one transaction
  └──► ai/api/, pdf/                 the only ways to a model and to a PDF
  │
  ▼
views/<view>/<element>/view.rs       the data the element gets back
```

The page never holds a secret. The key of the database of an evaluator opens the database once at the login and is wiped (`installation/session.rs`); the database stays open until the logout.

---

## 2. The folders

### 2.1 `model/` and `model_old/`: the data

**The models are built anew (user, 2026-10-28).** The earlier folder `model/` was renamed `model_old/`; the new `model/` gets the models one by one, each a structure that represents the data kept in the database, and the code is moved to them piece by piece so that the program works all the time. `model_old/` may use `model/`; everything above them may use both. What follows describes `model_old/`.

Plain types of every area, their rules of validity (`check`) and their normal form (`normalized`). **No I/O**: no database, no network, no file. The errors of the rules carry enums of the fields and of the objects (`model_old/text.rs`: `Field`), never text that is matched.

- **The normal form is the only place where text is trimmed.** A command of a view normalizes what it gets before it checks and stores it; `db/` writes what it gets; so memory and the database hold the same text.

### 2.2 `db/`: the databases

- **SQL lives only here** (the connection of the database of an evaluator is visible only inside `db/`, so SQL elsewhere does not compile).
- One folder per area of data (`system/`, `user/`, `ai/chats/`, `ai/connection/`, `ai/instructions/`, `ai/journal/`, `standards/`, `oscal/`, `evaluation_support/`, `conversion/`, `own_requirements/`, `evaluation_project/`, `evaluation/`), each with `schema.rs` (its tables), `rows.rs` (row ↔ type of `model/`) and the files of CRUD `create.rs`, `read.rs`, `update.rs`, `delete.rs`; a file an area does not need is missing.
- **Free functions everywhere:** `db::<area>::<operation>::<object>(db, ...)`. A caller imports the area as `db_<area>` and calls `db_<area>::<operation>::<object>`, so that every call shows which operation on which data it is.
- **Columns are read by their names; statements with more than one parameter name them** (`:title`); every statement is in the function that runs it.
- **A value the program never writes is an error**, `DbError::Corrupt { table, column, value }`, never replaced in silence. `DbError::Sql` is SQLite only; `NoRow` for a change of a row that does not exist; `Inconsistent` for input that does not hold together.
- **One transaction per operation**; an entry of a journal is written in the transaction of the change it records.
- **Migrations by version**: `db/user/migrations.rs`, led by `PRAGMA user_version`, run once when the database is opened. The history started again at version 1 with the fresh installation of step 48 (2026-10-24); a database of a version the program does not know is refused (`DbError::Corrupt`).
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
- A task works on plain data of `model/` and gives back plain data (`ai/tasks/catalog_outline.rs`, `catalog_section.rs`: groups and controls of OSCAL); applying a proposal is the work of the view editor of a catalog (`views/converter/catalog_editor/proposals/accept.rs`).

### 2.4a `pdf/`: reading PDFs

`poppler.rs`: the number of pages and the title of a PDF (`pdfinfo`), and the text of a range of pages (`pdftotext`), the PDF given on the standard input, never through a shell. It uses nothing of the program; the loading of the documents of a standard (`views/converter/standard_documents/load.rs`) reads a document part by part with it, reports every part and can stop between them.

### 2.5 `security/`

`random.rs` (random bytes from the kernel) and `process.rs` (no memory dumps of the process).

### 2.5a `installation/`: the installation, the account, the session

The core of the protection: the first start and every later start (`lifecycle.rs`), the check before the window opens (`verify.rs`, called by `main.rs`), the account with the keys of its database (`accounts.rs`: the password through Argon2id), and the session of the logged-in evaluator (`session.rs`: the account and the database opened once at the login). It uses `db/`, `security/` and `model/`; the window (`app/state.rs`) and the views use it.

### 2.7 `app/`: what the whole window shares

It is no interface, but what every view uses, as the session and the error handling of a web framework.

```text
state.rs        the installation, the session of the evaluator, the stop signals; nothing else
error.rs        every error of a command, in kinds the page tells apart
serializer/     how an error reaches the page (serde)
```

- **No command and no rule is here**: the commands are in the folders of the elements (`views/`).
- A command is callable only when it is listed in **four places**: the entry of its view or component (`views/<view>/<view>.rs`, `components/<component>/<component>.rs`), `routes.rs` (`generate_handler!`, used by the window and by the tests of the commands), `build.rs` (`AppManifest`), `capabilities/default.json`. A test (`src/command_lists_tests.rs`) reads the four lists and fails when they differ.

### 2.8 `ui/`: the page

```text
ui/src/views/<view>/  the views: the view and one component per element, named as the folders
                      of src/views/ (Chat.svelte, ChatList.svelte, Conversation.svelte)
ui/src/lib/api/<view>.ts
                      the commands of one view as typed functions, named as the commands; the types
                      mirror the view.rs of its elements
ui/src/lib/api/errors.ts
                      the error of a command, as app/serializer/error_view.rs sends it
ui/src/components/<component>/
                      the components: one folder each, with its Svelte component and what only
                      it uses (pdf_viewer/: PdfViewer.svelte, and pdf.ts, the code of pdf.js)
ui/src/lib/api/components/<component>.ts
                      the commands and the types of one component (block_node.ts, ai_work.ts: types only)
ui/src/styles/        the style, one sheet per part of the window, in the order of the cascade
                      (style.css lists them)
```

### 2.8a `components/`: the components (step 43)

A component is a small element used more than once: by several views, or many times in one view (`architecture.md`, section 10.2). It is built as a small view: its entry, the file of its name (`pdf_viewer/pdf_viewer.rs`), with its commands and their tests, and its data in `view.rs`; its commands are named `<component>_<action>`, their functions on the page are in `ui/src/lib/api/components/<component>.ts`, and its page is a folder of `ui/src/components/`. The views use it; it never uses a view. The first ones are the viewer of a standard (`pdf_viewer/`), shown by the view Converter, by its editor of a catalog and, embedded, beside the scheme of a task (step 51.2), the parts of an object (`part_rows/`), shown by the editor of a catalog and the browser of the standards, the tree of a catalog (`catalog_tree/`, step 51.1), shown by the editor of a catalog and the browser of the standards, the recovery code (`recovery_code/`, step 49), shown by four elements of the view Users, the block of a scheme (`block_node/`, a component of the page only, step 52), drawn by the Evaluation plan and the Evaluation lab, the work of the AI (`ai_work/`: in Rust one question with its events, steps 51 and 52; on the page `components/ai_work/ai_progress.ts`, what a panel shows of it), used by the Evaluation plan, the Evaluation lab and the proving ground, and the agent with knowledge of a work of the AI (`knowledge_agent/`, steps 57.3 and 57.6: in Rust its passages and the list of the agents, on the page `AgentPicker.svelte`), chosen in the chat, a master project, a catalog in conversion and an evaluation, and the button that asks before it deletes (`confirm_button/`, a component of the page only, the audit of 2026-10-25); a component that only shows data the elements give it has no entry, only its `view.rs`.

### 2.9 `views/`: the views of the window (steps 42 and 46)

```text
views/<view>/<view>.rs               the entry of the view: every command the page calls for it, first those
                                     called when the view opens, then the actions of its elements; each with
                                     what it does, where the page calls it, what it calls; its tests at the end
views/<view>/<element>/view.rs       the data an element gets (serde); its logic beside it (turn.rs, ...); an
                                     element with no data of its own has no folder, only its component on the page
ui/src/lib/api/<view>.ts             the commands of the view on the page, one function per command (chatListGet)
```

The modules of a view are declared in the map of `main.rs`; its flow of data is drawn in `architecture.md`, section 12.

- **The entry is the only file of the program in the folder of a view** (user, 2026-10-23), so that it is seen at once: what the elements of the view share belongs to the element that owns it (the editor of a catalog: the checked quote in `object_quotes/`, the object read from the page in `object_form/`); what several views share goes down to the folders below. Only the help of the tests may stand beside it (`test_support.rs`).
- **An element** is an interactive part of the window that needs the program: to process or store data, to scan, to ask the AI. What is only presentation (an open menu, a selection, a tab, the position during a drag) stays in the page and has no folder here; only the final result is sent (CLAUDE.md, "What goes to Rust").
- **A command is named `<element>_<action>`** (`chat_list_get`, `connection_form_save`) and lives in the entry of its view; a test (`src/command_lists_tests.rs`) checks both.
- **A command does its work with the folders below** (`model/`, `db/`, `ai/`, `pdf/`), never through a layer in between. What two elements need goes down to those folders; what only joins them for one view is a file of the view.
- **The flow between elements goes through the owner of the data:** an element that changed data gets back its own data and tells the view; the element that shows the data reads it again with its own command (Settings: a connection is saved, the list of connections reads again).
- **Commands run off the thread of the window:** `#[tauri::command(async)]` on a plain function runs it on a thread of the runtime of Tauri: the work of milliseconds (the database) and Argon2id at the login. A work that waits for the model is an `async fn` command that awaits `ai/api/` (the chat, the AI of the editor of a catalog); the reading of PDFs runs on the threads for blocking work (`spawn_blocking`, `views/converter/standard_documents/load.rs`).
- **Long works** (the loading of PDFs, the AI of the editor of a catalog, a message of the chat, the works of the AI of the Evaluation plan and the Evaluation lab) report their progress as an enum of their own (`LoadEvent`, `BuildEvent`, `ReplyEvent`, `AiEvent` of the component `ai_work`) through a `Channel` of Tauri, stop on the stop signal of `app/state.rs`, and hold no lock of the session while they wait for files or for the model.
- **Documents of OSCAL** (`docs/oscal.md`): every change is made on the document read from the database, checked against the schema of NIST and written back at once, in one transaction (`db/oscal/`); the page always gets the document as stored. What the program adds (quotes, proposals of the AI) is the evaluation support, in tables of its own, linked by UUID and id.
- **Tests call a command as the page does**: by name, with JSON, through the IPC of Tauri, in a window of its mock runtime (`views/test_support.rs`, the feature `test` of Tauri), on a real installation in a temporary folder.
- No `mod.rs` and no file that only declares modules: the modules of the views and their elements are written in the map of `main.rs` (steps 45 and 46).

---

## 3. Conventions

| Topic | Rule |
|---|---|
| Names | No file repeats the name of its folder (`db/system/schema.rs`, not `system/system.rs`). Functions of a resource are verbs. |
| Modules | **`main.rs` is the map of the program** (step 45, user 2026-10-23): every folder and its modules are declared there, in the order of the layers from the bottom up, each group with a comment that says which part of the project it is; a file that would only declare modules does not exist. A file with code that has modules of its own (`model_old/oscal.rs`) declares them itself. |
| Rules of the code | CLAUDE.md, "The rules for Rust" (The Power of 10 adapted). A condition that should never happen is checked with `ensure!(condition, error)` (`checks.rs`), which returns the error; never `assert!` outside the tests. |
| Comments | Code comments and documentation in English, thorough; the head of every file says what it holds and, where it helps, draws the path of the data. |
| Tests | In the file whose functions they test, as its last item: `#[cfg(test)] mod tests { use super::*; ... }` (step 44; a file `*_tests.rs` besides the two tests of the whole program is refused by `architecture_tests.rs`). Help for the tests in `test_support.rs` (of an area), `src/test_support.rs` (temporary folders) and `src/test_support/` (`db.rs`: the data of the tests of `db/`; `standards.rs`: a database with a small standard); a test of a whole area of CRUD is split by the function it checks, so that `read.rs` ends with the tests of reading; the tests of the whole program in `src/architecture_tests.rs` and `src/command_lists_tests.rs`. |
| Formatting | `rustfmt.toml` (120 characters); `cargo fmt --check` must be clean before a commit. |
| Checks | `cargo test` (with the test of the architecture and the test of the command lists), `npm run check` (svelte-check, clean), clippy. |
| Serialization | Only at the boundary to the page: `views/**/view.rs` and `app/serializer/`; the types of the core carry no serde attributes. **The one exception** (user, 2026-10-18, `docs/oscal.md`): the types of `model_old/oscal/` *are* the published format of NIST and carry serde with the names of OSCAL. |
| Secrets | Only in the encrypted database of the evaluator; never shown back, never printed, never in the page. |

---

## 4. The whole tree

Generated from the heads of the files (the tests left out):

```text
main.rs                      Entry point of IO Master.
routes.rs                  The routes of the window: the list of every command the page may
checks.rs                  The checks of conditions that should never happen (rule 5 of
model/                     The new models (2026-10-28).
  work_unit.rs                 A work unit: one piece of the work of an evaluation, as the standard
model_old/                 The data of the program as plain Rust types, and the rules of
  ai.rs                        The data of the communication with AI services: a connection as
  api_key.rs                   The API key of an AI connection of one evaluator
  chats.rs                     The chats of an evaluator with an AI model, as data: a chat in the
  conversion.rs                The conversion of the documents of a standard (PDF) into a catalog of
  evaluation_project.rs        The master project of an evaluation (step 51 of docs/todo.md): the
  evaluation.rs                An evaluation of the Evaluation lab (step 52): a work package carried
                                 out for one product; verdicts, evidence, fragments, results.
  evaluation_support.rs        The evaluation support (decided by the user, 2026-10-18): what the
  installation.rs              What an installation is: its identifier, made at the first start,
  knowledge.rs                 The knowledge bases of the AI (step 57.2): sources, passages of at most
                                 1500 characters, the query of a search (FTS5); the agents, their glossary,
                                 the passages chosen, the trial of the proving ground (step 57.3).
  own_requirement.rs           A requirement of the laboratory's own, outside the standards (step 51.1e
  recovery_code.rs             The recovery code of a forgotten password (step 49): 160 random bits
  task_scheme.rs               The scheme of a task of a master project (step 51.2), the template of the
                                 outputs of a project and the first scheme of a new task (step 51.3): its blocks
  work_package.rs              A work package of a master project (step 51.5): the tasks delegated
                                 to one evaluator; a task in one package at most.
  oscal.rs/                    The documents of NIST OSCAL (Open Security Controls Assessment
    assessment_plan.rs           An OSCAL assessment plan: the controls and objectives reviewed, the
    assessment_results.rs        OSCAL assessment results: observations (the evidence), findings per
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
      read.rs                      Reading the journal of the AI: a page by task and words, an entry whole.
      rows.rs                      An entry of the journal of the AI and its row.
      schema.rs                    The journal of the questions to the AI (P3 of `architecture.md`:
  conversion/                  The conversions of the documents of standards into catalogs (step 50.2).
    create.rs                    Starting the conversion of a catalog.
    delete.rs                    Deleting a conversion.
    read.rs                      Reading the conversions.
    rows.rs                      The rows of the conversions and the types of `model_old/conversion.rs` they
    schema.rs                    The tables of the conversions of the documents of standards into
    update.rs                    Changing a conversion: its stage, the pages of its table of contents,
  evaluation/                  The evaluations of the Evaluation lab (step 52): the frozen copy of a
                                 work package, the evidence, the chats, the fragments, the results and
                                 the texts of the outputs.
    create.rs                    Storing evaluations and what the evaluator adds to them.
    delete.rs                    Deleting evaluations and what the evaluator added to them.
    read.rs                      Reading evaluations and what the evaluator added to them.
    rows.rs                      The rows of the evaluations and the types of `model_old/evaluation.rs`.
    schema.rs                    The tables of the evaluations.
    update.rs                    The state of a task, the fragments accepted, the results, the outputs,
                                 the agents with knowledge and the passages unticked (step 57.4).
  evaluation_project/          The master projects of the evaluations and their tasks (step 51), the
                                 template of their outputs (standard_outputs, step 51.3).
    create.rs                    Storing master projects and their tasks.
    delete.rs                    Deleting master projects and their tasks.
    read.rs                      Reading master projects and their tasks.
    rows.rs                      The rows of the master projects and the types of
    schema.rs                    The tables of the master projects of the evaluations
    update.rs                    Changing master projects: their fields and catalogs, the order of their
                                 tasks, the fields and places of the blocks of a scheme (the tables
                                 task_blocks and task_connections of schema.rs, step 51.2)
  knowledge/                   The knowledge bases of the AI (step 57.2): the bases, their sources, and
                                 their passages with the index of their words (FTS5 of SQLite, BM25); the
                                 agents, their bases, the last trial of each (step 57.3).
    create.rs                    Storing a base, a source with its passages, an agent; an agent imported
                                 with its bases in one transaction, under free names (step 57.7).
    delete.rs                    Deleting a base, a source, an agent with what belongs to them.
    read.rs                      Reading bases, sources, agents and trials; the search of the passages.
    rows.rs                      The rows of the knowledge bases and the types of `model_old/knowledge.rs`.
    schema.rs                    The tables of the knowledge bases (a virtual table of FTS5).
    update.rs                    A base, an agent with its bases, the last trial of an agent.
  encryption/                  All code of encryption of the databases.
    database_key.rs              The key that encrypts a database of the program.
    generate_key.rs              Generating the key that encrypts the database.
    keys.rs                      The keys of the databases of the evaluators
    recovery.rs                  The second copy of the key of a database, wrapped with the recovery
    sqlcipher.rs                 Opening and creating encrypted databases.
  error.rs                     The error of every function of the database layer (`db/`).
  own_requirements/            Which catalog is the shared catalog of the requirements of the laboratory.
    delete.rs                    Forgetting the shared catalog of the requirements of the laboratory.
    read.rs                      Reading which catalog is the shared catalog of the requirements of the
    update.rs                    Storing which catalog is the shared catalog of the requirements of the
  evaluation_support/          The evaluation support (step 41): what the program adds to the
    create.rs                    Storing quotes and proposals of the evaluation support.
    delete.rs                    Deleting from the evaluation support.
    read.rs                      Reading the evaluation support.
    rows.rs                      The rows of the evaluation support and the types they become
    schema.rs                    The tables of the evaluation support (`model_old/evaluation_support.rs`),
  oscal/                       The documents of NIST OSCAL (step 41): catalogs, mappings, profiles,
    create.rs                    Storing a new document of OSCAL, checked against the schema of NIST.
    delete.rs                    Deleting a document of OSCAL. Its text is overwritten in the file,
    error.rs                     The error of an operation on a stored document of OSCAL: the rules of
    ids.rs                       New UUIDs and the time of a change, for the documents of OSCAL the
    read.rs                      Reading the documents of OSCAL.
    rows.rs                      The rows of the documents of OSCAL and the types they become
    schema.rs                    The table of the documents of OSCAL, in the database of an
    update.rs                    Replacing a document of OSCAL by its new version, checked against
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
    roles.rs                     The model a question goes to: the active connection, for every role
    tasks.rs                     The tasks of the AI: what a question is for. Every task is asked
  presets.rs/                  Presets: ready settings of well-known AI services, to fill the
    field_help.rs                Instructions for a chatbot: help to fill one field of an AI
  tasks/                       The tasks of the AI: for each, the question sent to the model and
    assessment.rs                The task `assessment` of the AI (step 52): the four results and a
                                 verdict proposed, on the accepted fragments only.
    candidates.rs                The requirements given to the AI for a choice, word for word under
                                 short references, in parts of a bounded size (step 51.4).
    catalog_contents.rs          The task `catalog_contents` of the AI (role *text analysis*, step 50.2
    catalog_outline.rs           The task `catalog_outline` of the AI (role *text analysis*): the
    catalog_section.rs           The task `catalog_section` of the AI (role *text analysis*): the
    chat.rs                      The task `chat` of the AI (role *consultation*): the memory of a
    evidence_fragments.rs        The task `evidence_fragments` of the AI (step 52): the exact quotes of
                                 a piece of evidence that matter for a requirement.
    evidence_text.rs             The text of the evidence in a question: by page, in bounded parts.
    input_chat.rs                The task `input_chat` of the AI (step 52): the chat of an input.
    knowledge_answer.rs          The task `knowledge_answer` of the AI (step 57.3): an agent answers from
                                 the passages of its bases; its quotes name their passages.
    knowledge_context.rs         The context of an agent after a question, in a section of its own (57.6).
    json.rs                      The JSON object in an answer of a model, and its texts.
    output_draft.rs              The task `output_draft` of the AI (step 52): the text of an output.
    related_elements.rs          The task `related_elements` of the AI (step 51.4): the work units and
                                 other elements related to the requirement of a task.
    requirement_choice.rs        The task `requirement_choice` of the AI (step 51.4): the requirements
                                 of a catalog a project has to cover.
    requirement_context.rs       The whole requirement for the works of the AI of the Evaluation lab
                                 (step 57.1): where it stands, its parameters, its links, the text of
                                 its related elements.
    task_scheme.rs               The task `task_scheme` of the AI (role *text analysis*, step 51.3): the
                                 inputs of the scheme of a task and corrections of its blocks
pdf/                       Reading PDFs with the tools of Poppler (`pdfinfo`, `pdftotext`): the
  poppler.rs                   Reading the text of a PDF, page by page, with the tools of
installation/              The installation of this Linux account, its one user and the
  accounts.rs                  The accounts of the evaluators: creating them, logging in, and
  lifecycle.rs                 The life of an installation: the first start, every later
  permissions.rs               The permissions of the folder of the installation: open to other
                                 accounts, or of another owner (shown as a bar; the audit of 2026-10-25).
  session.rs                   The logged-in evaluator (step 40): their account, their database,
  verify.rs                    The check of the installation at the start of the program.
security/                  The building blocks of protection: random numbers from the system,
  process.rs                   The protection of the memory of the process: no memory dumps.
  random.rs                    Random bytes from the kernel.
app/                       What the whole window shares (Tauri 2): the state the commands of the
  error.rs                     The errors the commands give back to the page, in kinds the page
  serializer/                  How an error of a command reaches the page (serde). The data of the
    error_view.rs                How an error of a command reaches the page:
  state.rs                     The state of the desktop interface, kept in Rust and never in
components/                The components of the window: small elements used more than once,
  ai_work/                     One question of a work of the AI with its events (steps 51 and 52):
                                 the views Evaluation plan and Evaluation lab.
    ask.rs                       One question of a work of the AI, with its events.
    view.rs                      The events of a work of the AI, as the page gets them.
  knowledge_agent/             The agent with knowledge of a work of the AI (steps 57.3 and 57.6): the
                                 passages it gives, the agents to choose; on the page `AgentPicker.svelte`.
    knowledge_agent.rs           The entry of the component: the list of the agents.
    find.rs                      The question widened by the glossary, searched in the bases of the agent;
                                 the context of the agent of a project, a catalog or a chat.
    view.rs                      The agents to choose, as the page gets them.
  catalog_tree/                The component "tree of a catalog": its groups and controls as rows (step
    catalog_tree.rs              The entry of the component "tree of a catalog": the groups and controls
    view.rs                      The data of the component "tree of a catalog", as the page gets it.
  part_rows/                   The component "parts of an object": the parts of a group or a control
    view.rs                      The data of the component "parts of an object": the parts of a group or
  recovery_code/               The component "recovery code": the code shown once, after the setup,
    view.rs                      The data of the component "recovery code": the code shown once, after
  pdf_viewer/                  The component "viewer of a standard": the original page of a PDF, with a
    pdf_viewer.rs                The entry of the viewer of a standard: its two commands
    view.rs                      The data of the viewer, as the page gets it: the documents it can
views/                     The views of the window, one folder per view and one folder per
  chat/                        The view Chat: the chats of the evaluator with the model of the role
    chat.rs                      The entry of the view Chat: every command the page calls for it
    chat_list/                   The element "list of chats" of the view Chat
      view.rs                      The data of the list of chats, as the page gets it.
    connection_line/             The element "connection line" of the view Chat: which AI connection
      view.rs                      The data of the connection line, as the page gets it.
    conversation/                The element "conversation" of the view Chat: the open chat, a new
      stored.rs                    What a chat keeps (`db/ai/chats/`, tables `chats` and `messages`, in
      turn.rs                      One turn of a chat: the message of the evaluator is checked and
      view.rs                      The data of the conversation as the page gets it: an open chat, and
  converter/                   The view Converter: the documents of a standard (PDF) converted into a
    converter.rs                 The entry of the view Converter: the conversion of the documents of a
    catalog_list/                The element "list of catalogs" of the view Converter: the catalogs of
      view.rs                      The data of the list of catalogs of the view Converter, as the page gets
    catalog_editor/              The editor of a catalog, a part of the view Converter: a catalog
      catalog_editor.rs            The entry of the editor of a catalog: every command the page calls
      ai_builder/                  The element "the AI" of the editor of a catalog: the AI reads a table
        controls.rs                  The controls of a group of a catalog proposed by the AI from the pages
        groups.rs                    The groups of a catalog proposed by the AI from a table of contents
        parts.rs                     The pages of a standard cut into parts that fit the context of a
        steps.rs                     The steps both works of the AI share: the model and the instructions
        view.rs                      The data of the AI of the editor, as the page gets and sends it, and
      conversion_wizard/           The element "wizard" of the editor of a catalog: the conversion step by
        contents.rs                  The AI finds the table of contents of a document of a standard (step
        locate.rs                    Where every group of a catalog starts in its document (step 50.2 of
        progress.rs                  How far a conversion is, read for the page: the stage and the pages of
        view.rs                      The data of the wizard of a conversion, as the page gets it and sends
      new_object/                  The element "new object" of the editor of a catalog: a dialog that adds
        view.rs                      The place of a new object, as the page sends it.
      object_form/                 The element "object" of the editor of a catalog: the group or control
        oscal_object.rs              A group or a control of OSCAL sent by the page as JSON, read into the
        view.rs                      The data of the object chosen, as the page gets it.
      object_quotes/               The element "quotes" of the editor of a catalog: the passages of the
        checked_quote.rs             A quote of a document of a standard, looked for on the page it names
        view.rs                      The data of the quotes of an object, as the page gets it.
      proposals/                   The element "proposals" of the editor of a catalog: what the AI
        accept.rs                    A proposal of the AI accepted into its catalog.
        view.rs                      The data of the proposals, as the page gets it.
    document_search/             The element "search" of the view Converter: the text of the
      search.rs                    The search of the text of the documents of standards: in the one form
      view.rs                      The data of the search, as the page gets it.
    new_catalog/                 The element "new catalog" of the view Converter: a catalog of
      made.rs                      A new, empty catalog of OSCAL: its metadata, and the documents of the
      view.rs                      The data of a new catalog, as the page gets it: the documents it can
    standard_documents/          The element "documents of standards" of the view Converter: the
      dialog.rs                    The dialog of the system for the PDFs of a standard.
      load.rs                      The loading of documents of standards from PDF files: a long work.
      stored.rs                    What is stored of a document of a standard, with its rules: a file is
      view.rs                      The data of the list of documents of standards, as the page gets and
  evaluation_lab/              The view Evaluation lab (step 52): a work package imported and carried
                                 out for one product; the report exported.
    evaluation_lab.rs            The entry of the view Evaluation lab: every command the page calls for it.
    test_support.rs              Help for the tests of the view: an evaluation and a fake AI.
    assessment/                  The element "assessment": the agent of a requirement.
      context.rs                   The agent with knowledge of a task and its passages (step 57.4).
      propose.rs                   The four results and a verdict proposed, on the accepted fragments.
    evaluation_list/             The element "list of evaluations".
      view.rs                      The evaluations with their progress, as the page gets them.
    evidence/                    The element "evidence" of an input.
      add.rs                       A PDF, a text file or a pasted text, kept as delivered.
      dialog.rs                    The dialog of the system for a file of evidence.
    fragments/                   The element "fragments" of an input.
      propose.rs                   The AI proposes the fragments of the evidence, quotes looked for.
      view.rs                      A fragment written by the evaluator, as the page sends it.
    input_chat/                  The element "chat of an input".
      chat.rs                      A question of the evaluator and the answer of the AI, kept.
    lab_task/                    The element "screen of a task".
      read.rs                      The screen of a task: scheme, requirement, inputs, results, outputs.
      view.rs                      The data of the screen, as the page gets it and sends it.
    lab_tasks/                   The element "screen of an evaluation".
      view.rs                      The evaluation with its tasks, how far each is and the whole (step 59).
    outputs/                     The element "outputs" of a task.
      draft.rs                     The AI drafts the text of an output whose condition holds.
    package_import/              The element "import of a work package".
      dialog.rs                    The dialog of the system for the file of a package.
      read.rs                      The file read: its format, the package, the catalogs attached.
      store.rs                     The package stored as an evaluation, all or nothing; notices.
      view.rs                      The product of the evaluation, as the page sends it.
    report/                      The element "report".
      dialog.rs                    The dialog of the system for the folder of the report.
      export.rs                    One file per output: the accepted texts, task by task.
  knowledge/                   The view Knowledge (RAG) (step 57): the knowledge bases of the AI, the
                                 agents made of them and their proving ground.
    knowledge.rs                 The entry of the view Knowledge (RAG): every command the page calls for it.
    agent_form/                  The element "form of an agent": its fields and its bases, ticked.
      dialog.rs                    The dialog of the system for the file of an agent exported.
      export.rs                    The file of an agent: the agent, its bases, sources and passages (57.7).
      view.rs                      The form of an agent with every base, as the page gets and sends it.
    agent_list/                  The element "list of agents".
      dialog.rs                    The dialog of the system for the file of an agent to import.
      import.rs                    The file of an agent read and checked against every limit (57.7).
      view.rs                      The agents with the numbers of their bases, as the page gets them.
    knowledge_base/              The element "knowledge base": its name, its sources.
      add.rs                       A PDF, a text file, a catalog of OSCAL or a note, cut into passages.
      dialog.rs                    The dialog of the system for a file of a source.
      view.rs                      The screen of a base, the catalogs to choose, as the page gets them.
    knowledge_list/              The element "list of knowledge bases".
      view.rs                      The bases with their numbers, as the page gets them; the form of a base.
    knowledge_search/            The element "search" of a base.
      search.rs                    The words made a query of FTS5, the passages ranked by BM25.
      view.rs                      The passages found, as the page gets them.
    proving_ground/              The element "proving ground" of an agent.
      ask.rs                       A question answered by the agent, its quotes checked, the trial kept.
      view.rs                      The last trial, as the page gets it.
  evaluation_plan/             The view Evaluation plan: it works only on the documents of OSCAL
    evaluation_plan.rs           The entry of the view Evaluation plan: it works only on the documents of
    new_project/                 The element "new project" of the view Evaluation plan: a master project
      catalogs.rs                  The catalogs a master project can use: the finished ones, and the check
      view.rs                      The data of a new master project, as the page gets it: the catalogs it
    oscal_documents/             The element "documents of OSCAL" of the view Evaluation plan: the
      dialog.rs                    The dialogs of the system for a file of OSCAL: import, export.
      file.rs                      A file of OSCAL (JSON) read into the database, and written from it:
      view.rs                      The data of the list of documents of OSCAL, as the page gets it.
    own_requirement/             The element "requirement of the laboratory": one outside the standards.
      catalog.rs                   The shared catalog of the requirements of the laboratory (step 51.1e of
      view.rs                      The data of a requirement of the laboratory, as the page sends it and
    project_form/                The element "project" of the view Evaluation plan: its title, scope, catalogs.
      view.rs                      The data of the form of a master project, as the page gets it: what the
    project_list/                The element "list of master projects" of the view Evaluation plan.
      view.rs                      The data of the list of master projects, as the page gets it.
    standard_browser/            The element "browser of the standards" of the view Evaluation plan.
      requirement.rs               A requirement of a catalog of a master project, read for the browser of
      search.rs                    The search of the requirements of the catalogs of a master project: a
      view.rs                      The data of the browser of the standards of a master project, as the
    standard_outputs/            The element "standard outputs": the template of the outputs of the
                                 tasks of a master project (step 51.3).
      view.rs                      The data of the template, as the page gets it and sends it.
    task_links/                  The element "related elements" of a task (step 51.4).
      propose.rs                   The AI finds the elements of other catalogs related to a task.
      view.rs                      The related elements, as the page gets them.
    task_list/                   The element "list of tasks" of the view Evaluation plan.
      view.rs                      The data of the list of tasks of a master project, as the page gets it:
    task_proposals/              The element "tasks proposed by the AI" (step 51.4).
      propose.rs                   The AI proposes the tasks of a project, part by part.
      view.rs                      The proposals, as the page gets them.
    task_scheme/                 The element "scheme of a task": its blocks input, requirement, output.
      propose.rs                   The AI proposes inputs of a scheme and corrections of its blocks
                                 (step 51.3): one question, the proposals stored for the planner.
      scheme.rs                    The scheme of a task read for the page: the task with its requirement
      view.rs                      The data of the scheme of a task, as the page gets it and sends it: the
    work_packages/               The element "work packages" (step 51.5): the tasks delegated to the
                                 evaluators, and the export of a package.
      export.rs                    The file of a package: our format and the catalogs as attachments.
      view.rs                      The packages, as the page gets them and sends them.
  settings/                    The view Settings: the settings of the program, the connections to
    settings.rs                  The entry of the view Settings: every command the page calls for it
    ai_journal/                  The element "journal of the AI" of the view Settings (step 58), read only.
      view.rs                      A page of the entries and one entry whole, as the page gets them.
    api_key/                     The element "API key" of the view Settings, tab AI: the key of one
      view.rs                      The data of the API key, as the page gets it: never the key itself.
    connection_check/            The element "check of a connection" of the view Settings, tab AI
      view.rs                      The report of a check of a connection, as the page gets it; the key
    connection_form/             The element "connection form" of the view Settings, tab AI: one
      view.rs                      The data of the connection form, as the page gets it and sends it.
    connection_list/             The element "list of connections" of the view Settings, tab AI
      view.rs                      The data of the list of connections, as the page gets it.
  shell/                       The view Shell: the frame of the window around every page of a
    shell.rs                     The entry of the view Shell: the state of the program
    program/                     The element "program" of the view Shell: the state of the program the
      view.rs                      The state of the program as the page gets it: whether the program
  users/                       The view Users: the account, the session and the encryption of its database
    users.rs                     The entry of the view Users: the setup, the login, the password, the logout
```
