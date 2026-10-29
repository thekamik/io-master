# IO Master: the structure of the code

This document says **where** everything is in the code and **which rules keep it in order**. The data, its links and its rules are in `data_model.md`; *what* the program is and *why* in `architecture.md`.

**The program is being rebuilt (user, 2026-10-28).** The data model was made anew (`model/`, `data_model.md`) with its database (`db/`), and the earlier code, which did not suit it, was removed from `test_branch`. The earlier program (its views, its AI layer, its login, its page) is kept in the branch `main` as a reference: `git show main:io-master/src/<path>`. Its parts come back one by one, rewritten on the new models; each brings its folder, its row in `architecture_tests.rs` and its section here. Until then the window opens empty.

---

## 1. The layers

```text
io-master/src/
  main.rs            the map of the program (every folder and its modules, in the order of the
                     layers, each group commented), main(), tauri_startup(); nothing else
  checks.rs          the macro ensure! (rule 5 of CLAUDE.md), known in the whole program
  model/             the data as it is kept in the databases: structures, fields only
  panels/            the panels of the window, each in two layers with the same folders:
    views/             the commands the page calls; their error
    logic/             the work behind them, not called by the page: program_start (the account,
                       its keys), evaluation_plan, evaluation, rag, chat, settings
  session/           what every command shares: the data folder, the user logged in, their database
  ai/                the only way to an AI service: a question prepared, sent (TLS 1.3), its answer
                     read while it streams and written into the journal before it is given out
  db/                create, read and update of the models; the two databases; their encryption
  security/          the building blocks of protection: random numbers, no memory dumps
  pdf/               reading PDFs with Poppler (pdfinfo, pdftotext): another program, in one place
io-master/ui/        the page (Svelte 5), a separate project: the first start, the login, the panels
```

### 1.1 The direction of the dependencies

```text
  panels/views/  ──►  session/
         │
         ▼
  panels/logic/<panel>/     (a panel may use another, in one direction)
         │
         ▼
        ai/                    (also used by panels/views/ for the one step without the database: send)
         │
         ▼
        db/
         │
         ▼
  model/, security/, pdf/     (nothing of the program)
```

A folder uses only itself and the folders below it, never a folder above it: no cycle. **The rule is checked by a test**, `src/architecture_tests.rs`, on the source of every folder (the tests aside). **The panels (user, 2026-10-28):** `panels/` holds the panels of the window in two layers, `panels/views/` and `panels/logic/`, each with one folder per panel and the same names: `program_start`, `evaluation_plan`, `evaluation`, `rag`, `chat`, `settings` (a folder appears in `main.rs` with its first file). `panels/views/<panel>/` holds only the commands the page calls; `panels/logic/<panel>/` the work behind them, which gets the open database as a parameter and never reaches the session, so that it is tested without a window. A panel of `panels/logic/` may use another in one direction only (`chat` and `evaluation` use `rag` and `evaluation_plan`, never the other way). Decided for what comes next: the functions of the making of a plan (the order of the work, the cross references, `blocked`) in `panels/logic/evaluation_plan/`; the check of an OSCAL document against the schema of NIST in a file of its own, not in `db/`; the communication with the AI services in a folder of its own below `panels/logic/`: `ai/` (user, 2026-10-29). The test of the architecture names the two layers by their paths of modules (`panels::views`, `panels::logic`).

## 2. The folders

### 2.1 `model/`: the data

One structure per kind of data kept in the database, its fields each commented, **no functions** (user, 2026-10-28); the objects named by their ids. The models, their links and their rules: `data_model.md`. The models are agreed with the user and not changed without them.

### 2.2 `db/`: the databases

- **Create, read and update, no delete** (user, 2026-10-28). Exceptions, each said in its file: the journal of the AI and the installation are created and read only; `delete_work_unit` removes a work unit only from a plan in planning (`evaluation/work_unit.rs`, user 2026-10-29: a planner changes a plan freely, and work units kept inactive would fill the database), the only deletion; writing a plan only orders its work units.
- **The layout mirrors `model/`**: one file per file of a model, holding its tables (`SCHEMA`), the reading of its rows and its functions `create_<model>`, `read_<model>` (and `read_<model>s` for a list), `update_<model>`. A list of a model is a table of its own, one row per element with its position (`lists.rs`).
- **Two databases**: `users/<id>.v2.db`, encrypted by SQLCipher with the key of the user (`database.rs`), and `system.v2.db`, not encrypted, holding only the installation and its user (`system_database.rs`). Their tables are made when they are missing; the layout changes freely until version 1 (CLAUDE.md).
- **SQL lives only here**: the connection of a database is visible only inside `db/`.
- A new object is given with `id` 0 and gets its id from the database. **What a model names must hold together**, checked at every write: a work unit and its plan (its standard, documents, dependencies, verdict, quotes), an API key and the address it was given for.
- Columns are read by their names; statements with more than one parameter name them; one transaction per operation; a value the program never writes is `DbError::Corrupt`, never replaced in silence; every read of a list is bounded by its limit (rule 2).
- `encryption/` holds the encryption of the database files (their key, SQLCipher), moved unchanged from the earlier program and audited by the user later; the wrapping of the key with the password and the recovery code is in `panels/logic/program_start/`.

### 2.2c `ai/`: the way to the AI

**The only way to an AI service** (user, 2026-10-29), below `panels/logic/` and above `db/`: it reads the integration and its key and writes the journal; nothing else of the program speaks with a service. A question goes through three steps, apart because the database is never used while an answer is awaited (the lock of the session is released meanwhile, so the window stays usable):

```text
panels/logic/<panel>   ──prepare(db, Question)──►  Prepared    the integration, its key (bound to its address),
  (the lock held)                                               the body in its protocol, the size checked
panels/views/<panel>   ──send(&Prepared, pieces)─►  Sent        the request over TLS 1.3; the pieces of the
  (the lock released)                                           answer into a channel; stopped when it is dropped
panels/views/<panel>   ──record(db, Prepared, Sent)► Answer     the entry of the journal written, then the
  (the lock held)                                               answer given out (Sent cannot be read before)
```

- `ask.rs`: `Question` (integration, agent, work unit, instructions, messages), `prepare`, `send`, `record` (the answer of an agent written into its journal first; a question without an agent, such as the check of an integration, is not kept). A question that does not fit the context of the model beside its answer (a fourth of the context, at most 16 000 tokens; three characters a token, on the safe side) is refused before it is sent, never shortened; a conversation that does not end with a message of the user is refused.
- `client.rs`: the connection, with the rules of the earlier program: `https://`, or `http://` only on this computer (`check_address`); TLS 1.3 or newer, the certificates of the system or only the CA certificate of the integration (`check_certificate`); no redirect, no proxy of the environment; the key marked sensitive in its header and masked in every text of an answer; every input bounded (32 MiB of a stream, 1 MiB of a line, 64 KiB of an error).
- `openai.rs` and `anthropic.rs`: the two protocols (`Protocol` of the integration): the body of a question, the header of the key, the reading of one event of the stream (its text, the model, the tokens, how it ended). Anthropic: the header `anthropic-version: 2026-10-21`, only the blocks of text read (the thinking of a model passed over), `refusal` read as `Refused`.
- `sse.rs` (the events of a stream, pieces of any size), `conversation.rs` (the messages as alternating turns that start with the user), `reading.rs` (what an answer said so far), `error.rs` (`AiError`, its kind from the status of the answer, its explanation from its `error.message`).

### 2.1a `panels/views/`: the commands of the page

Only what the page calls (user, 2026-10-28): one folder per part of the window, one file per element, each command with `#[tauri::command]` and its comment (what it does, where the page calls it, what it calls). The data the page gets is a structure of the file (`...View`, with serde): serialization only here. The error of every command is `panels/views/error.rs` (`{kind, message}`), never a password, a key or a code in it. A command is listed in three places that name the same commands: `tauri::generate_handler!` in `main.rs`, `build.rs` and `capabilities/default.json`; its function on the page in `ui/src/lib/api/<folder>.ts`, named as the command. A command that takes a moment (Argon2id) runs apart from the window (`async`).

- `program_start/installation.rs`: `installation_create` (the first start; gives the recovery code once).
- `program_start/login.rs`: `login_get` (installed or not, who is logged in), `login_log_in`, `login_log_out`.
- `evaluation_plan/standards.rs`: `standard_list`, `standard_import` (the dialog of the system opened in Rust, the file read within its limit; the page never sees a path).
- `evaluation_plan/plans.rs`: `plan_list` (the plans in planning), `plan_create`, `plan_get`, `plan_save`, `plan_add_document`.
- `evaluation_plan/work_units.rs`: `work_unit_list`, `work_unit_controls` (the controls a plan does not hold yet), `work_unit_add`, `work_unit_add_all`, `work_unit_remove`, `work_unit_get`, `work_unit_save`, `work_unit_add_proof`.
- `agents/agent.rs`: `agent_list`, `agent_create` (a name and an integration), `agent_get` (the agent with every integration and knowledge base to choose), `agent_save` (the form as one structure), `agent_knowledge_test` (the passages the agent would give with a question; nothing is sent to an AI service).
- `agents/knowledge.rs`: `knowledge_base_list`, `knowledge_base_create`, `knowledge_base_save`, `knowledge_source_list` (without the texts), `knowledge_source_add` and `knowledge_source_replace` (a `.txt`, `.md` or `.pdf` file chosen in the dialog of the system opened in Rust), `knowledge_source_rename`, `knowledge_search` (the passages of one base).
- `agents/journal.rs`: `agent_journal_list` (the entries of one agent, pages of 50, the newest first), `agent_journal_get` (the request and the answer); the journal is only read.
- `settings/integrations.rs`: `integration_list` (the active one, whether each has a key; never the key), `integration_create`, `integration_save` (the form as one structure), `integration_activate`, `integration_set_key` (the key goes in only), `integration_check` (one short question through `ai/`, the lock released while the answer is awaited; not kept in a journal).
- `opened.rs`: the open database of the session for a command, or "nobody is logged in"; the lock of the session is held for the whole command (except `integration_check`, which releases it while the AI answers).

### 2.1b `session/`: the session

What every command shares (user, 2026-10-28), kept by Tauri: the data folder (`data_folder.rs`: `$XDG_DATA_HOME/io-master` or `~/.local/share/io-master`) and, behind a lock, the user logged in with their open database (`state.rs`), the only global state of the program. The database is opened at the login and closed at the logout and at every end of the program (`main.rs`, `RunEvent::Exit`).

### 2.2b `panels/logic/evaluation_plan/`: the making of a plan

The backend of the panel of the plan (user, 2026-10-29), without SQL: `oscal.rs` (an OSCAL catalog checked against the schema of NIST built into the program, and its controls in the order of the document, the tree walked with an explicit stack), `standards.rs` (`import_standard`), `plans.rs` (`create_plan` with the verdicts of Common Criteria, `update_plan` only in planning and never taking away a standard or a document a work unit uses, `add_document_place`), `work_units.rs` (`add_work_unit`, `add_all_work_units`, `remove_work_unit`, `update_work_unit_parameters` (the agent named must exist), `add_proof_place`), `fill.rs` ("Fill with AI", user 2026-10-29: the agent of the planner proposes every parameter of each work unit, in the order of the plan, over what was there; `fill_units`, refused without an agent of the planner; `fill_question`, the instructions of the agent and the task with its knowledge, the work unit with its parameters now, the labels of the plan, the documents and the agents; `read_proposal`, the JSON object of the answer; `apply_proposal`, only the names the plan and the user hold, a dependency that would close a cycle left out (also on a work unit not filled yet), a proof of the same name kept, a note for everything left out), `order.rs` (Kahn's algorithm: the order of the work, the cross references, `blocked`, marked after every change of the work units), `error.rs` (`PlanError`).

### 2.2d `panels/logic/settings/`: the settings of the AI

The backend of the panel "AI settings", which handles only the model `Integration` (user, 2026-10-29), without SQL: `integrations.rs` (an integration checked before it is stored: a name, an address and a CA certificate `ai/` accepts, a model, a context of 512 tokens at least; the first one made becomes the active one; the key stored trimmed and bound to the present address, replaced by a new one, never read back but as "has a key"; `check_question`, the question that checks an integration), `error.rs` (`SettingsError`).

### 2.2e `panels/logic/agents/`: the agents and their knowledge

The backend of the panel "Agents" (user, 2026-10-29), which handles the models `Agent`, `KnowledgeBase` and `Source`, reads `JournalEntry` (the journal of an agent) and `Integration` (to choose one), without SQL. An agent is a layer of knowledge over the model of an integration; it is changed freely until its answers are good enough. `agent.rs` (`new_agent` with default instructions and limits; `change_agent`, checked: an integration and bases that exist, each base once, the glossary trimmed with its empty rows taken out, the limits in their ranges; `agent_knowledge`, **the knowledge an agent gives with a question**: the question widened by the meanings of the terms of its glossary it names, the passages of its bases that fit best, at most `max_passages`, as many as fit whole in `max_characters`, the same in the test of the panel and in an evaluation), `knowledge.rs` (the bases, shared by the agents; a source from a `.txt` or `.md` file in UTF-8 or a `.pdf` read through `pdf/` by parts of pages; its text replaced by a newer file or its name changed, never removed; the search in one base), `error.rs` (`AgentError`). It is the only panel of `panels/logic/` that uses `pdf/`.

### 2.2a `panels/logic/program_start/`: the account

The backend of the panel of the first start and the login (user, 2026-10-28), also of the recovery of a forgotten password, which starts at the login; without SQL: `access.rs` (the first start: the data folder, `system.v2.db`, the installation, the user, the two copies of the key, the database of the user, the recovery code given once; the login: the key unwrapped with the password, the database opened; a wrong password and an unknown name fail the same way; a first start that fails half way removes the files it made). The building blocks, moved unchanged from the earlier program and audited by the user later: `keys.rs` (the password, Argon2id, AES-256-GCM), `recovery.rs` and `recovery_code.rs` (the recovery code), `generate_key.rs` (the random key). A copy of the key is bound to the id of its user, which never changes.

### 2.3 `security/` and `pdf/`

`security/`: `random.rs` (random bytes from the kernel) and `process.rs` (no memory dumps of the process). `pdf/poppler.rs`: the number of pages and the text of a range of pages of a PDF, the PDF given on the standard input, never through a shell.

### 2.4 `ui/`: the page

A separate project (Svelte 5, TypeScript strict), built into `ui/dist` and embedded in the program. `App.svelte` shows the first start, the login or the frame of the user logged in (the side bar with the panels and "Log out"), from `login_get`; `views/program_start/` (`InstallationForm.svelte`, `LoginForm.svelte`), `views/evaluation_plan/` (user, 2026-10-29: the screens follow the model, where the work units belong to their plan; which screen is shown stays in the page: `EvaluationPlan.svelte`, the tabs and the screen open, with the page and the filter of the list kept for "back"; `StandardsTab.svelte`, the standards and the import; `PlansTab.svelte`, the plans and a new plan; `PlanEditor.svelte`, "← Plans", the data of the plan, `PlanForm.svelte`, and its work units, `PlanWorkUnits.svelte`, by pages of 50 with a filter and "Add from a standard" folded; `WorkUnitEditor.svelte`, "← the plan", "Previous / Next", the requirement, the dependencies found by a filter, "Depended on by", the documents, the agent that will work on it (chosen among the agents of the panel Agents), the guidance, the proofs), `views/agents/` (user, 2026-10-29: `Agents.svelte`, two tabs, the two models made there; `AgentList.svelte`, the agents and a new one with its integration; `AgentEditor.svelte`, "← Agents", the name, the integration, the instructions, the knowledge bases, the glossary edited by rows, the limits, the test of the knowledge, and `AgentJournal.svelte`, the journal of the agent by pages with one entry whole; `KnowledgeBases.svelte`, the bases and a new one; `BaseEditor.svelte`, "← Knowledge bases", the name and the description, the sources added from a file, renamed or their text replaced, the search in the base), `views/settings/` (only the integrations: `Settings.svelte`; `IntegrationList.svelte`, the list, the active one, whether each has a key; `IntegrationForm.svelte`, the data of one integration, its key only written, the check of the connection with its answer, model and tokens), `components/recovery_code/` (the code shown once, confirmed by typing one of its groups), `components/save_status/` (what a form says of its last write: "Saving…" while it runs, the buttons locked, then "Saved at" its local time, kept nowhere; user 2026-10-29, ready for the automatic writes planned), `lib/api/` (one function per command, `errors.ts`). The styles of the earlier page are kept in `ui/src/styles/` for the views that come back. The page holds no secret: the passwords are cleared after each call.

## 3. Conventions

| Topic | Rule |
|---|---|
| Names | No file repeats the name of its folder. Functions are verbs; the functions of `db/` are `create_`, `read_`, `update_` with the name of their model. |
| Modules | **`main.rs` is the map of the program**: every folder and its modules are declared there, in the order of the layers from the bottom up, each group with a comment; there is no `mod.rs`. Code not used by the program yet carries no `#[allow(dead_code)]`: its warnings remind what is still to be joined to a panel (user, 2026-10-29, the one exception to "zero warnings"). |
| Rules of the code | CLAUDE.md, "The rules for Rust" (The Power of 10 adapted). A condition that should never happen is checked with `ensure!(condition, error)` (`checks.rs`), which returns the error; never `assert!` outside the tests. |
| Comments | At every function: what it does, where it is called from, which functions it calls. The head of every file says what it holds; the tables of a file of `db/` are drawn there. |
| Tests | In the file whose functions they test, as its last item: `#[cfg(test)] mod tests { use super::*; ... }`. Help for the tests in `src/test_support.rs` (temporary folders) and `src/test_support/` (`database.rs`: a new database, a plan, a work unit, an integration, an agent; `pdf.rs`: a small PDF; `ai_service.rs`: a fake AI service on 127.0.0.1, real TLS 1.3, 1.2 or plain HTTP, that answers in either protocol and keeps the requests it got); the test of the whole program in `src/architecture_tests.rs`. |
| Formatting | `rustfmt.toml` (120 characters); `cargo fmt --check` must be clean before a commit. |
| Checks | `cargo test` (with the test of the architecture), `cargo clippy --all-targets`, `npm run check` (svelte-check), all clean. |
| Serialization | Only at the boundary to the page (`panels/views/`); the models carry no serde attributes. |
| Secrets | Only in the encrypted database of the user; never shown back, never printed, never in the page. |

---

## 4. The whole tree

Generated from the heads of the files (the tests left out):

```text
main.rs                      Entry point of IO Master.
checks.rs                    The checks of conditions that should never happen (rule 5 of
test_support.rs              Help for the tests of the whole program: temporary directories, a new
architecture_tests.rs        The direction of the dependencies between the folders of the
model/
  ai/
    agent.rs                     An AI agent made in the program: which model it asks, what it is told
    api_key.rs                   The API key of an integration with an AI service. It is a password to
    chat.rs                      A chat: the place where the user consults an agent, for example the
    integration.rs               An integration with an external AI service: where the program sends
    journal.rs                   The journal of the AI: every question an agent asked an AI service and
    rag.rs                       The knowledge for the AI (RAG, retrieval-augmented generation): the
  evaluation/
    evaluation_plan.rs           An evaluation plan: the work units of an evaluation, with the
    lab_proof.rs                 A proof of the laboratory: the result of a functional test the
    oscal_standard_document.rs    An OSCAL document of a standard: the catalog of a standard (the CEM,
    product_document.rs          A document of the product: a document of the client (ST, IXIT, ...)
    work_unit.rs                 A work unit: one piece of the work of an evaluation, as the standard
  user/
    program_data.rs              The data of the program: the installation, made at the first start,
    user_data.rs                 The user of the installation: one Linux account holds one
    user_keys.rs                 The keys of the database of a user, as they are kept in the system
db/
  database.rs                  The database of the user, made anew for the models of `model/`
  error.rs                     The error of every function of `db/`: why a database could not be
  lists.rs                     The lists of a model kept in a table of their own: a list of ids (the
  system_database.rs           The system database, made anew for the models of `model/` (2026-10-28):
  ai/
    agent.rs                     The AI agents (`Agent`), in the table `ai_agents` of the database of the
    api_key.rs                   The API keys of the integrations (`ApiKey`), in the table
    chat.rs                      The chats (`Chat`) and their messages (`Message`), in the tables
    integration.rs               The integrations with AI services (`Integration`), in the table
    journal.rs                   The journal of the AI (`JournalEntry`), in the table `ai_journal` of the
    rag.rs                       The knowledge for the AI (RAG): the knowledge bases (`KnowledgeBase`)
  encryption/
    database_key.rs              The key that encrypts a database of the program.
    sqlcipher.rs                 Opening and creating encrypted databases.
  evaluation/
    evaluation_plan.rs           The evaluation plans (`EvaluationPlan`: a plan, an evaluation copied
    lab_proof.rs                 The proofs of the laboratory (`LabProof`: the results of the functional
    oscal_standard_document.rs    The OSCAL documents of the standards (`OscalStandardDocument`), in the
    product_document.rs          The documents of the product (`ProductDocument`: the ST, the IXIT), in
    work_unit.rs                 The work units of the plans (`WorkUnit`), in the table `work_units` of
  user/
    program_data.rs              The data of the program: the installation (`Installation`), in the table
    user_data.rs                 The user of the installation (`User`), in the table `users` of the
    user_keys.rs                 The keys of the database of a user (`UserKeys`), in the table
panels/
  logic/
    agents/
      agent.rs                     The agents, as their panel makes and changes them, and the knowledge an
      error.rs                     The error of the panel of the agents: a change refused, a file of a
      knowledge.rs                 The knowledge bases and their sources, as the panel of the agents makes
    evaluation_plan/
      error.rs                     The error of the making of a plan: a document that is no catalog, a
      fill.rs                      "Fill with AI" (user, 2026-10-29): the agent of the planner of a plan
      order.rs                     The order of the work of a plan and its cross references (user,
      oscal.rs                     The OSCAL catalogs of the standards, read for the plan: a document
      plans.rs                     The plans in planning: a new plan, its data changed by the planner
      standards.rs                 The standards of the plans: an OSCAL catalog imported by the user,
      work_units.rs                The work units of a plan in planning: added from a control of one of the
    program_start/
      access.rs                    The account of the user: the first start (the installation, the user,
      generate_key.rs              Generating the key that encrypts the database.
      keys.rs                      The keys of the databases of the evaluators
      recovery.rs                  The second copy of the key of a database, wrapped with the recovery
      recovery_code.rs             The recovery code of a forgotten password (step 49 of docs/history.md,
    settings/
      error.rs                     The error of the settings of the AI: a change refused, an integration
      integrations.rs              The integrations with AI services, as the settings of the AI change
  views/
    error.rs                     The error of a command, as the page gets it:
    opened.rs                    The open database of the user logged in, as every command of a panel
    agents/
      agent.rs                     The agents of the panel of the agents, as the page calls them: the list,
      journal.rs                   The journal of an agent, as the page reads it: one page of its entries,
      knowledge.rs                 The knowledge bases of the panel of the agents, as the page calls them:
    evaluation_plan/
      plans.rs                     The plans of the panel of the plan, as the page calls them: the list of
      standards.rs                 The standards of the panel of the plan, as the page calls them: their
      work_units.rs                The work units of the panel of the plan, as the page calls them: the list
    program_start/
      installation.rs              The first start, as the page calls it (`ui/src/views/program_start/`):
      login.rs                     The login, as the page calls it (`ui/src/views/program_start/`): what
    settings/
      integrations.rs              The integrations of the settings of the AI, as the page calls them: the
ai/
  anthropic.rs                 The messages of Anthropic (`/v1/messages`): the body of a question, its
  ask.rs                       **The only way to an AI service** (user, 2026-10-29): every question of
  client.rs                    The connection to an AI service: one request, the answer read while it
  conversation.rs              The conversation as both protocols take it: turns of the user and of
  error.rs                     Why a question to an AI service failed, in kinds the window can tell
  openai.rs                    The chat completions of OpenAI (`/v1/chat/completions`), which most
  reading.rs                   What an answer said so far: its text, the model that wrote it, the
  sse.rs                       Reading a stream of server-sent events (SSE), the form in which both
session/
  data_folder.rs               Where the data of the program is: `$XDG_DATA_HOME/io-master`, or
  state.rs                     The session: what every command of the window shares (user,
security/
  process.rs                   The protection of the memory of the process (step 47 of docs/history.md):
  random.rs                    Random bytes from the kernel.
pdf/
  poppler.rs                   Reading the text of a PDF, page by page, with the tools of
test_support/
  ai_service.rs                A fake AI service for the tests: a server on 127.0.0.1 that answers
  database.rs                  Help for the tests of `db/`: a new database of the user in a temporary
  pdf.rs                       Help for the tests that run the tools of Poppler (`pdf/poppler.rs`): a
```
