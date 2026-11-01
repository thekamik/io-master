# IO Master: the structure of the code

*Where* everything is in the code and which rules keep it in order. **`io-master/src/main.rs` is the map**: every module, in the order of the layers, each group commented; there is no `mod.rs`.

## 1. The layers

```text
io-master/src/
  checks.rs       ensure!, the check with recovery (rule 5), known in the whole program
  model/          the data as kept in the databases: structures, fields only (data_model.md)
  security/       random bytes from the kernel; no memory dumps of the process
  pdf/            pdfinfo and pdftotext of Poppler, the PDF on their standard input
  db/             storage/: the two database files and their encryption; models/: create, read, update of the models
  domain/         the rules of the data (models/, a mirror of model/): where a plan is in its life, the order of its work
  ai/             the AI on the models (models/, a mirror of model/): the integration, the journal, the agent, the chat
  session/        the data folder; the user logged in, their open database, the progress of the long work of the AI
  panels/logic/   the work behind each panel, given the open database
  panels/views/   the commands the page calls
io-master/ui/     the page (Svelte 5, TypeScript strict), a separate project
```

**The direction of the dependencies**, checked on the source by `src/architecture_tests.rs` (the tests aside):

| Folder | May use |
|---|---|
| `model`, `security`, `pdf` | nothing of the program |
| `db` | `model`, `security` (inside: `models/` uses `storage/`; `storage/` uses only `models/schema.rs`) |
| `domain` | `db`, `model` |
| `ai` | `domain`, `db`, `model` (inside: the journal, the agent and the chat use the integration; the integration only the API key) |
| `session` | `db`, `model` |
| `panels::logic` | `ai` (of the integration only `question.rs`, `sending.rs`, `address.rs`), `domain`, `db`, `model`, `security`, `pdf` |
| `panels::views` | `panels::logic`, `ai` (of the integration only `question.rs`, `sending.rs`), `domain`, `db`, `model`, `session` |

`panels/views/` uses `ai/models/ai/integration/sending.rs` for the one step without the database (`send`), so that the lock of the session is released while the AI answers. A panel of `panels/logic/` may use another in one direction only (`evaluation` uses `evaluation_plan` for the order of the work).

**Models × concerns** (user, 2026-11-01). The program is a grid: `model/` declares the structures, and each concern mirrors `model/` with the functions on each model in the file of its name, so that the place of a function follows from its model and its concern:

```text
                      model/ai/integration.rs     model/evaluation/work_unit.rs     model/session/progress.rs
db/models/  (store)   db/models/ai/integration.rs db/models/evaluation/work_unit.rs  (never stored)
domain/models/ (rules)                            domain/models/evaluation/work_unit.rs
ai/models/  (the AI)  ai/models/ai/integration/   ai/models/evaluation/work_unit.rs
```

The whole program is brought to it step by step (`todo.md`); `panels/logic/` keeps only what a panel shows, and only `panels/views/` knows the window (Tauri, serde), so that another front end (an HTTPS server) would replace it alone.

## 2. The folders

### `db/`

Two parts (user, 2026-10-31), and `error.rs` (`DbError`), the error of both:

```text
db/
  error.rs       DbError
  storage/       the database files: user_database.rs (users/<id>.v2.db, SQLCipher), system_database.rs
                 (system.v2.db, not encrypted): where each is, made, opened; connection.rs: what both do
                 with a connection just opened (the wait for a busy database, the tables); encryption/
  models/        the models in them: ai/, evaluation/, user/ (the mirror of model/), schema.rs, lists.rs

models/<area>/<model>.rs ──SCHEMA──► models/schema.rs ──USER_TABLES / SYSTEM_TABLES──► storage/
storage/ ──UserDatabase / SystemDatabase (the connection, seen only in db/)──► models/<area>/<model>.rs
```

`models/` holds one file per file of `model/`, with its tables (`SCHEMA`) and its functions `create_<model>`, `read_<model>` (`read_<model>s` for a list), `update_<model>`; `schema.rs` lists the tables of each database (parted by the database, not by the folder; the order does not matter to SQLite); a list of a model is a table of its own, one row per element with its position (`lists.rs`). No delete, but `delete_work_unit` for a plan in planning. A new object is given with `id` 0. What a model names must hold together, checked at every write (a work unit and its plan, an API key and its address; in an evaluation, only the results of a work unit change; a document or a proof is frozen once given). `storage/` makes the missing tables whenever it opens a database; `storage/encryption/` opens the files with SQLCipher. The knowledge: `models/ai/rag.rs` cuts every source into passages of 1500 characters and indexes them (FTS5); a search passes over common English words. The text of a document of the product is cut and indexed the same way when its file is given (`models/evaluation/product_document.rs`), with the place of each passage: it gives the text whole again, and the passages that fit a work unit.

### `domain/`

The rules of the data (user, 2026-11-01: `data_model.md`, "the rules of the data live outside `model/`"), in a mirror of `model/`; `error.rs` (`DomainError`: refused, or a failure of the database), which each panel turns into its own error.

```text
domain/models/evaluation/
  evaluation_plan.rs   check_planning, plan_in_planning, evaluation_in_progress (where a plan is in its life);
                       work_order (Kahn: the order of the work, the cross references), check_order (cross_reference
                       and blocked written after every change of the work units of a plan)
  work_unit.rs         WorkUnitParameters, update_work_unit_parameters (the parameters of a work unit in planning,
                       then check_order)
```

### `ai/`

The core of the program that talks with the AI (user, 2026-11-01: a core part gets a folder of its own, as `db/` has; `panels/logic/` holds only what a panel shows). **`ai/models/` mirrors `model/`**, as `db/models/` does: the functions of the AI on each model are in the file (or the folder) of its name. `error.rs` (`AiError`) is the error of all of them.

```text
ai/
  error.rs              AiError
  models/ai/
    integration/        Integration, the service that answers
                        question.rs: Question, prepare ──► Prepared (the integration, its key, the body; the size checked)
                        sending.rs:  send ──► Sent (the answer streamed, no database); send_whole (awaited whole)
                        address.rs (the address and the CA certificate checked), connection.rs (TLS 1.3, no
                        redirect, no proxy, the stream read, every input bounded), openai.rs, anthropic.rs (the body,
                        the key header, one event read), sse.rs (events from pieces of any size), turns.rs
                        (alternating turns starting with the user), reading.rs (the answer read so far)
    api_key.rs          ApiKey: the key masked in every text taken from an answer
    journal.rs          JournalEntry: record ──► Answer (the journal of the agent written before the answer is given)
    agent.rs            Agent: the passages of its bases that fit a question (its glossary, its limits);
                        agent_question (its instructions, the task, its knowledge, the messages ──► Question)
    chat.rs             Chat: a question prepared with its oldest messages left out until it fits
  models/session/
    progress.rs         Progress (a helper model): a step of a long work written into it (record_step)

panels ──► agent.rs: agent_question ──► integration/question.rs: prepare ──► integration/sending.rs: send
                                                                   ──► integration/connection.rs ──► the service
       ──► journal.rs: record ──► db (the journal of the agent)
```

The integration knows of the other models only the API key (masked in its errors); the journal, the agent and the chat ask through the integration (checked by `architecture_tests.rs`). The panels never see the connection or a protocol.

### `session/`

`data_folder.rs` (where the data is) and `state.rs` (`Session`, kept by Tauri, the only global state: the user logged in, their open database and the `Progress` of the long work of the AI (`model/session/progress.rs`), each behind its lock; `begin_work` begins a work when none runs, and the guard it gives marks it ended however it ends).

### `panels/`

Each panel has a folder in both layers. The logic gets the open database as a parameter and never reaches the session, so it is tested without a window. The commands take what the page sends, check its size, call the logic and give back a `...View` (serde only here); their error is `panels/views/error.rs` (`{kind, message}`). `opened.rs` gives the open database or "nobody is logged in"; `progress.rs` gives the progress of the last long work on one plan (`ProgressView`), which the page asks for while it runs and when a panel is opened again (`evaluation_assess_progress`, `plan_fill_progress`); each step of a filling or an assessment is written into it (`ai/models/session/progress.rs`: `record_step`); `chat/chats.rs` passes each piece of an answer to the page while it comes, and stops it at once on "Stop". One long work of the AI runs at a time (a filling, an assessment or a question of a chat); the evaluator saves no results while it runs.

| Panel | `logic/` | `views/` commands |
|---|---|---|
| `program_start` | `access.rs` (first start, login), `keys.rs`, `recovery.rs`, `recovery_code.rs`, `generate_key.rs` | `installation_create`, `login_get`, `login_log_in`, `login_log_out` |
| `evaluation_plan` | `oscal.rs` (a catalog checked, its controls), `standards.rs`, `plans.rs`, `work_units.rs`, `fill.rs` ("Fill with AI": parameters, dependencies, the preview) | `standard_*`, `plan_*`, `work_unit_*`, `plan_fill_preview`, `plan_fill_parameters`, `plan_fill_dependencies`, `plan_fill_progress`, `plan_fill_stop` |
| `agents` | `agent.rs` (an agent made, its changes checked; its knowledge and its question are in `ai/models/ai/agent.rs`), `knowledge.rs` (bases, sources from files, the search) | `agent_*`, `knowledge_*`, `agent_journal_list`, `agent_journal_get` |
| `evaluation` | `evaluations.rs` (an evaluation begun on a copy of a plan with its agent; the agent changed), `inputs.rs` (the documents and proofs given once; whether a file is a PDF told by its first bytes), `question.rs` (the question of a work unit: its JSON, its dependencies, its documents whole or by passages, its proofs), `answer.rs` (the answer read; the quotes checked in the documents; the verdict with the dependencies), `run.rs` (the work units in the order of the work, the preview with the inputs missing, the question prepared), `results.rs` (the results of the evaluator, of author User) | `evaluation_list`, `evaluation_create`, `evaluation_get`, `evaluation_set_agent`, `evaluation_document_give`, `evaluation_proof_give`, `evaluation_assess_preview`, `evaluation_assess`, `evaluation_assess_progress`, `evaluation_assess_stop`, `evaluation_unit_list`, `evaluation_unit_get`, `evaluation_unit_save` |
| `chat` | `chats.rs` (a chat made and renamed; the question with the knowledge of the agent; the question and the answer kept) | `chat_list`, `chat_create`, `chat_get`, `chat_rename`, `chat_ask` (the answer streamed into the page), `chat_stop` |
| `settings` | `integrations.rs` (an integration checked, its key, the question that checks it) | `integration_*` |

A command is listed in `tauri::generate_handler!` (`main.rs`), `build.rs` and `capabilities/default.json`; its function on the page is in `ui/src/lib/api/<panel>.ts`, named as the command.

### `ui/`

`App.svelte` (the first start, the login, or the frame with the side bar), `views/<panel>/` (the screens of each panel; which screen is shown stays in the page), `components/` (used in more than one place: `recovery_code`, `save_status`), `lib/api/` (one function per command, `errors.ts`, `progress.ts`: the type of the progress the panels ask for), `styles/` (the sheets, one dark theme).

## 3. Conventions

| Topic | Rule |
|---|---|
| Names | No file repeats the name of its folder. Functions are verbs; `db/` functions are `create_`, `read_`, `update_` with the name of their model. |
| Comments | At every function: what it does, `Input:`, `Output:`, `Called by:` and `Calls:` with the file of each function, a comment over every step that is not plain (`CLAUDE.md`, "Comments"). The head of every file says what it holds; a file of `db/models/` draws its tables. |
| Rules of the code | `CLAUDE.md`, "The Power of 10" and its rules for Rust. |
| Tests | In the file they test, its last item. Help in `src/test_support.rs` and `src/test_support/` (`database.rs`: a database with its data; `pdf.rs`: a small PDF; `ai_service.rs`: a fake AI service on 127.0.0.1 with real TLS, either protocol; `evaluation.rs`: an evaluation begun on a plan, its ST given). |
| Checks | `cargo test`, `cargo clippy --all-targets` (clean but `dead_code` during the rebuild), `npm run check`, `cargo fmt --check`. |
