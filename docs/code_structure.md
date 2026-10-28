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
  db/                create, read and update of the models; the two databases; the encryption
  security/          the building blocks of protection: random numbers, no memory dumps
  pdf/               reading PDFs with Poppler (pdfinfo, pdftotext): another program, in one place
io-master/ui/        the page (Svelte 5), a separate project: one line until the views come back
```

### 1.1 The direction of the dependencies

```text
        db/
         │
         ▼
  model/, security/, pdf/     (nothing of the program)
```

A folder uses only itself and the folders below it, never a folder above it: no cycle. **The rule is checked by a test**, `src/architecture_tests.rs`, on the source of every folder (the tests aside). Two decisions of 2026-10-28 for the folders that come next: the functions of the making of a plan (the order of the work, the cross references, `blocked`) get a folder of their own, above `db/`; the check of an OSCAL document against the schema of NIST is a file of its own, not a part of `db/`.

## 2. The folders

### 2.1 `model/`: the data

One structure per kind of data kept in the database, its fields each commented, **no functions** (user, 2026-10-28); the objects named by their ids. The models, their links and their rules: `data_model.md`. The models are agreed with the user and not changed without them.

### 2.2 `db/`: the databases

- **Create, read and update, no delete** (user, 2026-10-28). Exceptions, each said in its file: the journal of the AI and the installation are created and read only; a work unit leaves a plan only when the plan is written without it (`evaluation/evaluation_plan.rs`), the only removal.
- **The layout mirrors `model/`**: one file per file of a model, holding its tables (`SCHEMA`), the reading of its rows and its functions `create_<model>`, `read_<model>` (and `read_<model>s` for a list), `update_<model>`. A list of a model is a table of its own, one row per element with its position (`lists.rs`).
- **Two databases**: `users/<id>.v2.db`, encrypted by SQLCipher with the key of the user (`database.rs`), and `system.v2.db`, not encrypted, holding only the installation and its user (`system_database.rs`). Their tables are made when they are missing; the layout changes freely until version 1 (CLAUDE.md).
- **SQL lives only here**: the connection of a database is visible only inside `db/`.
- A new object is given with `id` 0 and gets its id from the database. **What a model names must hold together**, checked at every write: a work unit and its plan (its standard, documents, dependencies, verdict, quotes), an API key and the address it was given for.
- Columns are read by their names; statements with more than one parameter name them; one transaction per operation; a value the program never writes is `DbError::Corrupt`, never replaced in silence; every read of a list is bounded by its limit (rule 2).
- `encryption/` holds all code of encryption, moved unchanged from the earlier program and audited by the user later.

### 2.3 `security/` and `pdf/`

`security/`: `random.rs` (random bytes from the kernel) and `process.rs` (no memory dumps of the process). `pdf/poppler.rs`: the number of pages and the text of a range of pages of a PDF, the PDF given on the standard input, never through a shell.

### 2.4 `ui/`: the page

A separate project (Svelte 5, TypeScript strict), built into `ui/dist` and embedded in the program. Today one line (`App.svelte`); the styles of the earlier page are kept in `ui/src/styles/` for the views that come back. The page calls no command yet: `build.rs` and `capabilities/default.json` list none.

## 3. Conventions

| Topic | Rule |
|---|---|
| Names | No file repeats the name of its folder. Functions are verbs; the functions of `db/` are `create_`, `read_`, `update_` with the name of their model. |
| Modules | **`main.rs` is the map of the program**: every folder and its modules are declared there, in the order of the layers from the bottom up, each group with a comment; there is no `mod.rs`. A folder not used by the program yet carries `#[allow(dead_code, reason = ...)]` there until it is. |
| Rules of the code | CLAUDE.md, "The rules for Rust" (The Power of 10 adapted). A condition that should never happen is checked with `ensure!(condition, error)` (`checks.rs`), which returns the error; never `assert!` outside the tests. |
| Comments | At every function: what it does, where it is called from, which functions it calls. The head of every file says what it holds; the tables of a file of `db/` are drawn there. |
| Tests | In the file whose functions they test, as its last item: `#[cfg(test)] mod tests { use super::*; ... }`. Help for the tests in `src/test_support.rs` (temporary folders) and `src/test_support/` (`database.rs`: a new database, a plan, a work unit, an integration, an agent; `pdf.rs`: a small PDF); the test of the whole program in `src/architecture_tests.rs`. |
| Formatting | `rustfmt.toml` (120 characters); `cargo fmt --check` must be clean before a commit. |
| Checks | `cargo test` (with the test of the architecture), `cargo clippy --all-targets`, `npm run check` (svelte-check), all clean. |
| Serialization | Only at the boundary to the page, when the views come back; the models carry no serde attributes. |
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
    instructions.rs              The instructions of the program for a task of the AI: the fixed part
    integration.rs               An integration with an external AI service: where the program sends
    journal.rs                   The journal of the AI: every question the program asked an AI service
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
db/
  database.rs                  The database of the user, made anew for the models of `model/`
  error.rs                     The error of every function of `db/`: why a database could not be
  lists.rs                     The lists of a model kept in a table of their own: a list of ids (the
  system_database.rs           The system database, made anew for the models of `model/` (2026-10-28):
  ai/
    agent.rs                     The AI agents (`Agent`), in the table `ai_agents` of the database of the
    api_key.rs                   The API keys of the integrations (`ApiKey`), in the table
    chat.rs                      The chats (`Chat`) and their messages (`Message`), in the tables
    instructions.rs              The instructions of the tasks of the AI (`Instructions`), in the table
    integration.rs               The integrations with AI services (`Integration`), in the table
    journal.rs                   The journal of the AI (`JournalEntry`), in the table `ai_journal` of the
    rag.rs                       The knowledge for the AI (RAG): the knowledge bases (`KnowledgeBase`)
  encryption/
    database_key.rs              The key that encrypts a database of the program.
    generate_key.rs              Generating the key that encrypts the database.
    keys.rs                      The keys of the databases of the evaluators
    recovery.rs                  The second copy of the key of a database, wrapped with the recovery
    recovery_code.rs             The recovery code of a forgotten password (step 49 of docs/history.md,
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
security/
  process.rs                   The protection of the memory of the process (step 47 of docs/history.md):
  random.rs                    Random bytes from the kernel.
pdf/
  poppler.rs                   Reading the text of a PDF, page by page, with the tools of
test_support/
  database.rs                  Help for the tests of `db/`: a new database of the user in a temporary
  pdf.rs                       Help for the tests that run the tools of Poppler (`pdf/poppler.rs`): a
```
