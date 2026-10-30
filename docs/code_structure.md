# IO Master: the structure of the code

*Where* everything is in the code and which rules keep it in order. **`io-master/src/main.rs` is the map**: every module, in the order of the layers, each group commented; there is no `mod.rs`.

## 1. The layers

```text
io-master/src/
  checks.rs       ensure!, the check with recovery (rule 5), known in the whole program
  model/          the data as kept in the databases: structures, fields only (data_model.md)
  security/       random bytes from the kernel; no memory dumps of the process
  pdf/            pdfinfo and pdftotext of Poppler, the PDF on their standard input
  db/             create, read, update of the models; the two databases; their encryption
  ai/             the only way to an AI service
  session/        the data folder; the user logged in with their open database; a long work and its stop
  panels/logic/   the work behind each panel, given the open database
  panels/views/   the commands the page calls
io-master/ui/     the page (Svelte 5, TypeScript strict), a separate project
```

**The direction of the dependencies**, checked on the source by `src/architecture_tests.rs` (the tests aside):

| Folder | May use |
|---|---|
| `model`, `security`, `pdf` | nothing of the program |
| `db` | `model`, `security` |
| `ai` | `db`, `model` |
| `session` | `db`, `model` |
| `panels::logic` | `ai`, `db`, `model`, `security`, `pdf` |
| `panels::views` | `panels::logic`, `ai`, `db`, `model`, `session` |

`panels/views/` uses `ai/` for the one step without the database (`send`), so that the lock of the session is released while the AI answers. A panel of `panels/logic/` may use another in one direction only (`evaluation_plan` uses `agents`).

## 2. The folders

### `db/`

One file per file of `model/`, holding its tables (`SCHEMA`) and its functions `create_<model>`, `read_<model>` (`read_<model>s` for a list), `update_<model>`; a list of a model is a table of its own, one row per element with its position (`lists.rs`). No delete, but `delete_work_unit` for a plan in planning. A new object is given with `id` 0. What a model names must hold together, checked at every write (a work unit and its plan, an API key and its address, an agent of the planner only in planning). The tables are made when missing (`database.rs`, `system_database.rs`); `encryption/` opens the files with SQLCipher. The knowledge: `ai/rag.rs` cuts every source into passages of 1500 characters and indexes them (FTS5); a search passes over common English words.

### `ai/`

`ask.rs` (`Question`; `prepare`, `send`, `record`; `Answer`), `client.rs` (the connection: the address and the CA certificate checked, TLS 1.3, no redirect, no proxy, the key sensitive and masked, every input bounded), `openai.rs` and `anthropic.rs` (the body of a question, the key header, the reading of one event of the stream), `sse.rs` (events from pieces of any size), `conversation.rs` (alternating turns starting with the user), `reading.rs` (the answer read so far), `error.rs` (`AiError`).

### `session/`

`data_folder.rs` (where the data is) and `state.rs` (`Session`, kept by Tauri: the user logged in with their open database behind a lock, the only global state; whether a long work of the AI runs and whether it was asked to stop).

### `panels/`

Each panel has a folder in both layers. The logic gets the open database as a parameter and never reaches the session, so it is tested without a window. The commands take what the page sends, check its size, call the logic and give back a `...View` (serde only here); their error is `panels/views/error.rs` (`{kind, message}`). `opened.rs` gives the open database or "nobody is logged in"; `answer.rs` awaits a whole answer of the AI; `chat/chats.rs` passes each piece of an answer to the page while it comes, and stops it at once on "Stop". One long work of the AI runs at a time (a filling or a question of a chat).

| Panel | `logic/` | `views/` commands |
|---|---|---|
| `program_start` | `access.rs` (first start, login), `keys.rs`, `recovery.rs`, `recovery_code.rs`, `generate_key.rs` | `installation_create`, `login_get`, `login_log_in`, `login_log_out` |
| `evaluation_plan` | `oscal.rs` (a catalog checked, its controls), `standards.rs`, `plans.rs`, `work_units.rs`, `order.rs` (Kahn: the order, the cross references, `blocked`), `fill.rs` ("Fill with AI": parameters, dependencies, the preview) | `standard_*`, `plan_*`, `work_unit_*`, `plan_fill_preview`, `plan_fill_parameters`, `plan_fill_dependencies`, `plan_fill_stop` |
| `agents` | `agent.rs` (an agent checked; the knowledge it gives with a question), `knowledge.rs` (bases, sources from files, the search) | `agent_*`, `knowledge_*`, `agent_journal_list`, `agent_journal_get` |
| `chat` | `chats.rs` (a chat made and renamed; the question with the knowledge of the agent; the question and the answer kept) | `chat_list`, `chat_create`, `chat_get`, `chat_rename`, `chat_ask` (the answer streamed into the page), `chat_stop` |
| `settings` | `integrations.rs` (an integration checked, its key, the question that checks it) | `integration_*` |

A command is listed in `tauri::generate_handler!` (`main.rs`), `build.rs` and `capabilities/default.json`; its function on the page is in `ui/src/lib/api/<panel>.ts`, named as the command.

### `ui/`

`App.svelte` (the first start, the login, or the frame with the side bar), `views/<panel>/` (the screens of each panel; which screen is shown stays in the page), `components/` (used in more than one place: `recovery_code`, `save_status`), `lib/api/` (one function per command, `errors.ts`), `styles/` (the sheets, one dark theme).

## 3. Conventions

| Topic | Rule |
|---|---|
| Names | No file repeats the name of its folder. Functions are verbs; `db/` functions are `create_`, `read_`, `update_` with the name of their model. |
| Comments | At every function: what it does, where it is called from, which functions it calls. The head of every file says what it holds; a file of `db/` draws its tables. |
| Rules of the code | `CLAUDE.md`, "The Power of 10" and its rules for Rust. |
| Tests | In the file they test, its last item. Help in `src/test_support.rs` and `src/test_support/` (`database.rs`: a database with its data; `pdf.rs`: a small PDF; `ai_service.rs`: a fake AI service on 127.0.0.1 with real TLS, either protocol). |
| Checks | `cargo test`, `cargo clippy --all-targets` (clean but `dead_code` during the rebuild), `npm run check`, `cargo fmt --check`. |
