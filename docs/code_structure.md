# IO Master: the structure of the code

*Where* everything is in the code and which rules keep it in order; model by model, where each is stored, processed and shown: `architecture_map.md`. **`io-master/src/main.rs` is the map**: every module, in the order of the groups, each commented; there is no `mod.rs`.

## 1. The three groups

```text
io-master/src/
  checks.rs        ensure!, the check with recovery (rule 5), known in the whole program
  infrastructure/  what belongs to no object and no panel, and knows no object
  objects/         every model with the functions that work on it, a folder per object
  panels/          the window: session.rs, views/ (the commands the page calls), logic/ (what a panel shows)
io-master/ui/      the page (Svelte 5, TypeScript strict), a separate project
```

**Three groups** (user, 2026-11-01), the dependencies one way: `panels/ ──► objects/ ──► infrastructure/`. Only `panels/views/` knows the window (Tauri, serde), so that another front end (an HTTPS server) would replace it alone.

**The direction of the dependencies**, checked on the source by `src/architecture_tests.rs` (the tests aside):

| Folder | May use |
|---|---|
| `infrastructure::clock`, `::data_folder`, `::security`, `::pdf` | nothing of the program |
| `infrastructure::storage` | `infrastructure::security` (a new key of a database made of random bytes) |
| `objects` | `infrastructure`; inside, the roles one way (below) |
| `objects::ai::integration` | of the other objects only the API key, the model of the journal (`Ending`) and the model of a chat (`Message`, `Speaker`) |
| `objects::ai::api_key` | the model and the store of the integration it is bound to |
| `panels::session` | `objects::user`, `objects::session`, `infrastructure` |
| `panels::logic` | every object, but of the AI of the integration only `question.rs`, `sending.rs`, `address.rs`; `infrastructure` |
| `panels::views` | what `panels::logic` may use, `panels::logic`, `panels::session` |

**The roles of an object go one way** (`the_roles_of_an_object_go_one_way`): a file uses only the roles at or below its own, `model ◄── store ◄── rules ◄── ai`, also between objects; `objects/schema.rs` serves as a store, `objects/error.rs` and `objects/text.rs` as rules, `objects/ai_error.rs` as the AI.

**SQL only in the store**: `rusqlite` and the connection of an open database (`.connection()`) appear only in `infrastructure/storage/` and the stores of the objects (`<object>_store.rs`), checked by `architecture_tests.rs` (`the_sql_is_only_in_the_store`).

`panels/views/` uses `objects/ai/integration/integration_ai/sending.rs` for the one step without the database (`send`), so that the lock of the session is released while the AI answers. A panel of `panels/logic/` may use another in one direction only; a panel whose work is all on objects has no folder in `panels/logic/`.

## 2. The folders

### `infrastructure/`

What belongs to no object and no panel, and knows no object (user, 2026-11-01): `clock.rs` (`now()`: the time of the computer, as every time written into the databases is read), `data_folder.rs` (`$XDG_DATA_HOME/io-master`), `security/` (`random.rs`: random bytes from the kernel; `process.rs`: no memory dumps of the process), `pdf/` (`poppler.rs`: `pdfinfo` and `pdftotext`, the PDF on their standard input), and the database files:

```text
infrastructure/storage/
  error.rs       DbError, the error of every function on a database
  user_database.rs (users/<id>.v2.db, SQLCipher), system_database.rs (system.v2.db, not encrypted): where each
                 is, made, opened; connection.rs: what both do with a connection just opened (the wait for a
                 busy database, the tables given by the caller); lists.rs: a list of ids as rows of its own table
  encryption/    the key of the database files (database_key.rs, generate_key.rs: a new random key at the first
                 start) and SQLCipher (sqlcipher.rs)

<object>_store.rs ──SCHEMA──► objects/schema.rs ──USER_TABLES / SYSTEM_TABLES──►
objects/user/user_data/user_data_rules.rs ──create / open(data_dir, ..., tables)──► storage/
storage/ ──UserDatabase / SystemDatabase──► the stores
```

### `objects/`

Every model with the functions that work on it (user, 2026-11-01), a folder per object, its files named by the object and their role:

| File | Holds | May use |
|---|---|---|
| `<object>_model.rs` | the structure, fields only (`data_model.md`) | the models |
| `<object>_store.rs` | its tables (`SCHEMA`), `create_`, `read_`, `update_` (the only SQL) | the stores, the models, `infrastructure/storage/` |
| `<object>_rules.rs` | the rules of its data | the rules, the stores, the models, `infrastructure/` |
| `<object>_ai.rs` | the work of the AI on it | every role, `infrastructure/` |

A big role becomes a folder of the same name (`user_keys_rules/`, `integration_ai/`), whose files carry no prefix. A loop over the children of a model belongs to the parent (the assessment of an evaluation in `evaluation_plan_ai.rs`, of one work unit in `work_unit_ai/`).

**The stores.** One store per model, with its tables (`SCHEMA`) and its functions `create_<model>`, `read_<model>` (`read_<model>s` for a list), `update_<model>`; `objects/schema.rs` lists the tables of each database (parted by the database, not by the folder; the order does not matter to SQLite); a list of a model is a table of its own, one row per element with its position (`infrastructure/storage/lists.rs`). No delete, but `delete_work_unit` for a plan in planning. A new object is given with `id` 0. What a model names must hold together, checked at every write (a work unit and its plan, an API key and its address; in an evaluation, only the results of a work unit change; a document or a proof is frozen once given). The knowledge: `rag_store.rs` cuts every source into passages of 1500 characters and indexes them (FTS5); a search passes over common English words. The text of a document of the product is cut and indexed the same way when its file is given (`product_document_store.rs`), with the place of each passage: it gives the text whole again, and the passages that fit a work unit.

**The errors.** `objects/error.rs` (`DomainError`: refused, a document that is no OSCAL catalog, a PDF whose text cannot be read, or a failure of the database) is the error of the rules, which each panel turns into its own error; `objects/ai_error.rs` (`AiError`) the error of every work of the AI; `objects/text.rs` (`checked_text`, `check_length`) the one rule of a text the user gives.

The integration knows of the other objects only the API key (masked in its errors), the model of the journal and the model of a chat; the journal, the agent and the chat ask through it; the panels see of it only `question.rs`, `sending.rs`, `address.rs`, its model and its store, never the connection or a protocol. Their fields read by the journal alone (`Prepared`, `Sent`) are visible only in `objects/ai/` (`pub(in crate::objects::ai)`).

```text
objects/
  schema.rs, error.rs, ai_error.rs, text.rs
objects/ai/
  integration/   integration_model.rs (Integration, Protocol), integration_store.rs,
                 integration_ai/ (the service that answers):
                   question.rs: Question, prepare ──► Prepared (the request, the names for the journal; the size and
                   the number of messages checked)
                   sending.rs:  send ──► Sent (the answer streamed, no database); send_whole (awaited whole)
                   address.rs (the address and the CA certificate checked), connection.rs (TLS 1.3, no redirect, no
                   proxy, the stream read, every input bounded); protocol.rs, the one place that chooses the protocol
                   (needs_key, body, key_headers, read_event) ──► openai.rs, anthropic.rs (the body, the key header,
                   one event read); sse.rs (events from pieces of any size), turns.rs (alternating turns starting
                   with the user), reading.rs (the answer read so far)
  agent/         agent_model.rs (Agent, GlossaryEntry), agent_store.rs,
                 agent_rules.rs (new_agent: the default instructions and limits; change_agent; checked: a name, an
                 integration and bases that exist, limits in their ranges, the glossary),
                 agent_ai.rs (the passages of its bases that fit a question: its glossary, its limits;
                 agent_question: its instructions, the task, its knowledge, the messages ──► Question)
  chat/          chat_model.rs (Chat, Message, Speaker), chat_store.rs,
                 chat_rules.rs (new_chat, rename_chat, checked_question, add_question),
                 chat_ai.rs (chat_question with the knowledge of its agent, prepared: its oldest messages left out
                 until it fits; add_answer: the answer written into the chat with its mark)
  api_key/       api_key_model.rs (ApiKey), api_key_store.rs (bound to the origin of its integration),
                 api_key_ai.rs (the key masked in every text taken from an answer)
  journal/       journal_model.rs (JournalEntry, Ending), journal_store.rs,
                 journal_ai.rs (record ──► Answer: the journal of the agent written before the answer is given)
  rag/           rag_model.rs (KnowledgeBase, Source), rag_store.rs,
                 rag_rules.rs (add_base, change_base; source_text: .txt, .md, .pdf within its limit; add_source,
                 replace_source_text, rename_source; search_base, the test of a base)
objects/evaluation/
  evaluation_plan/  evaluation_plan_model.rs (EvaluationPlan, PlanState), evaluation_plan_store.rs,
                 evaluation_plan_rules/
                   planning.rs: the life of a plan and its data: create_plan (the verdicts of Common Criteria),
                   update_plan, add_document_place; check_planning, plan_in_planning, plan_to_fill
                   evaluation.rs: a plan in evaluation: new_evaluation (a copy for one product), change_agent,
                   evaluation_in_progress (the evaluation, or refused), needed_proofs
                   order.rs: work_order (Kahn: the order of the work, the cross references), check_order
                   (cross_reference and blocked written after every change of the work units of a plan)
                   work_units.rs: the work units as children of the plan: available_controls, add_work_unit,
                   add_all_work_units (from the controls of a standard of the plan), remove_work_unit
                 evaluation_plan_ai.rs: the work units to assess (units_ready, the preview) and to fill
                 (fill_units, fill_preview); the loops assess_evaluation and fill_plan, a step per work unit, each
                 written into the progress, "Stop" read before each
  work_unit/     work_unit_model.rs (WorkUnit, Quote, Verdict, Author), work_unit_store.rs,
                 work_unit_rules/
                   parameters.rs: WorkUnitParameters, update_work_unit_parameters (then check_order);
                   add_proof_place
                   results.rs: Results, save_results (the results of the evaluator, of author User); quotes_found
                   (a quote word for word in its document, for the evaluator and the AI alike)
                 work_unit_ai/
                   assessment.rs (assess_work_unit: the question prepared, sent, the answer written),
                   assessment_question.rs, assessment_answer.rs (the answer read, the quotes checked, the verdict
                   with the dependencies); filling.rs (fill_work_unit, FillTask), proposal.rs (the questions of the
                   parameters and of the dependencies, their answers read and written)
  product_document/  product_document_model.rs, product_document_store.rs,
                 product_document_rules.rs (document_file: .pdf, .txt, .md within its limit; document_text,
                 give_document)
  lab_proof/     lab_proof_model.rs, lab_proof_store.rs, lab_proof_rules.rs (give_proof: its whole record)
  oscal_standard_document/  oscal_standard_document_model.rs, oscal_standard_document_store.rs,
                 oscal_standard_document_rules.rs (import_standard; check_catalog: the JSON schema of NIST, built
                 in; catalog_controls: the tree walked with a bounded stack; OscalError)
objects/session/
  progress/      progress_model.rs (Progress, Problem: a helper model, never kept),
                 progress_ai/ step.rs (record_step: a step of a long work written into it),
                 lock.rs (the locks of the session as a long work takes them: the database only for the short
                 steps)
objects/user/
  program_data/  program_data_model.rs (Installation, Settings), program_data_store.rs
  user_data/     user_data_model.rs (User), user_data_store.rs,
                 user_data_rules.rs (create_account: the installation, the user, the keys, the databases;
                 installed_user; log_in; AccountError)
  user_keys/     user_keys_model.rs (UserKeys, WrappedKey, RecoveryWrappedKey), user_keys_store.rs,
                 user_keys_rules/ password.rs (the key wrapped with the password: Argon2id, HKDF, AES-256-GCM),
                 recovery.rs (wrapped with the recovery code), recovery_code.rs (RecoveryCode: 160 bits, groups,
                 check sum)

panels ──► agent_ai.rs: agent_question ──► integration_ai/question.rs: prepare ──► integration_ai/sending.rs: send
                                                                   ──► integration_ai/connection.rs ──► the service
                                                                       (integration_ai/protocol.rs: the protocol chosen)
       ──► journal_ai.rs: record ──► journal_store.rs (the journal of the agent)
```

### `panels/`

`session.rs` (`Session`, kept by Tauri, the only global state: the user logged in, their open database and the `Progress` of the long work of the AI, each behind its lock; `begin_work` begins a work when none runs, and the guard it gives marks it ended however it ends).

Each panel has a folder in `views/`, and in `logic/` when it shows something the objects do not give. The logic gets the open database as a parameter and never reaches the session, so it is tested without a window. The commands take what the page sends, check its size, call the objects and the logic and give back a `...View` (serde only here); their error is `panels/views/error.rs` (`{kind, message}`). `opened.rs` gives the open database or "nobody is logged in"; `progress.rs` gives the progress of the last long work on one plan (`ProgressView`), which the page asks for while it runs and when a panel is opened again (`evaluation_assess_progress`, `plan_fill_progress`); each step of a filling or an assessment is written into it (`objects/session/progress/progress_ai/step.rs`: `record_step`); `chat/chats.rs` passes each piece of an answer to the page while it comes, and stops it at once on "Stop". One long work of the AI runs at a time (a filling, an assessment or a question of a chat); the evaluator saves no results while it runs.

| Panel | `logic/` | `views/` commands |
|---|---|---|
| `program_start` | none: the first start and the login are the rules of the user (`objects/user/user_data/user_data_rules.rs`) | `installation_create`, `login_get`, `login_log_in`, `login_log_out` |
| `evaluation_plan` | none: the plan, its work units and the standards are objects (`objects/evaluation/`), "Fill with AI" in `evaluation_plan_ai.rs` | `standard_*`, `plan_*`, `work_unit_*`, `plan_fill_preview`, `plan_fill_parameters`, `plan_fill_dependencies`, `plan_fill_progress`, `plan_fill_stop` |
| `agents` | none: the agent and the knowledge are objects (`objects/ai/agent/`, `objects/ai/rag/`) | `agent_*`, `knowledge_*`, `agent_journal_list`, `agent_journal_get` |
| `evaluation` | `inputs.rs` (every place of input with the work units that need it); the evaluation begun, its inputs given and the results of the evaluator are in `objects/evaluation/` | `evaluation_list`, `evaluation_create`, `evaluation_get`, `evaluation_set_agent`, `evaluation_document_give`, `evaluation_proof_give`, `evaluation_assess_preview`, `evaluation_assess`, `evaluation_assess_progress`, `evaluation_assess_stop`, `evaluation_unit_list`, `evaluation_unit_get`, `evaluation_unit_save` |
| `chat` | none: the chat is an object (`objects/ai/chat/`) | `chat_list`, `chat_create`, `chat_get`, `chat_rename`, `chat_ask` (the answer streamed into the page), `chat_stop` |
| `settings` | `integrations.rs` (an integration checked, its key, the question that checks it; to be brought to the objects, `todo.md`) | `integration_*` |

A command is listed in `tauri::generate_handler!` (`main.rs`), `build.rs` and `capabilities/default.json`; its function on the page is in `ui/src/lib/api/<panel>.ts`, named as the command.

### `ui/`

`App.svelte` (the first start, the login, or the frame with the side bar), `views/<panel>/` (the screens of each panel; which screen is shown stays in the page), `components/` (used in more than one place: `recovery_code`, `save_status`), `lib/api/` (one function per command, `errors.ts`, `progress.ts`: the type of the progress the panels ask for), `styles/` (the sheets, one dark theme).

## 3. Conventions

| Topic | Rule |
|---|---|
| Names | The files of an object carry its name and their role (`agent_store.rs`); the files inside a folder of a role carry no prefix (`work_unit_ai/assessment.rs`). Functions are verbs; the functions of a store are `create_`, `read_`, `update_` with the name of their model. No module is imported under another name. |
| Comments | At every function: what it does, `Input:`, `Output:`, `Called by:` and `Calls:` with the file of each function, a comment over every step that is not plain (`CLAUDE.md`, "Comments"). The head of every file says what it holds; a store draws its tables. |
| Rules of the code | `CLAUDE.md`, "The Power of 10" and its rules for Rust. |
| Tests | In the file they test, its last item. Help in `src/test_support.rs` and `src/test_support/` (`database.rs`: a database with its data; `pdf.rs`: a small PDF; `ai_service.rs`: a fake AI service on 127.0.0.1 with real TLS, either protocol; `evaluation.rs`: an evaluation begun on a plan, its ST given). |
| Checks | `cargo test`, `cargo clippy --all-targets` (clean but `dead_code` during the rebuild), `npm run check`, `cargo fmt --check`. |
