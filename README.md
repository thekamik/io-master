# IO Master

**A desktop tool for security evaluation laboratories: it plans the evaluation of a product against standards kept as NIST OSCAL catalogs, and brings AI agents with their own knowledge to each work unit, while every decision stays with the evaluator.**

IO Master is installed on the computer of an evaluator, in their own Linux account, and used through one desktop window. It knows no particular standard: a standard is an **OSCAL 1.2.3 catalog**, checked against the JSON schema of NIST (Common Criteria with the CEM and ETSI EN 303 645 first).

**Status: beta, `V0.3` (2026-11-04).** Every function of the program is in place: the plan, the evaluation with the AI, the report. It is a test version, not a release: before version 1 it must pass the full process of tests and verification (`docs/todo.md`), and the data of the tests is not kept between versions until then. The work goes to `test_branch`; `main` and `V0.3` (the stable beta) are brought to its state when it is approved; `V0.1` and `V0.2` are closed snapshots of earlier stages. The number of the program stays 0.1.0.

## What the beta does

- **First start and login.** One Linux account, one installation, one user. The data is in a database encrypted with SQLCipher, its key opened by the password (Argon2id); a recovery code, shown once, is a second key: it sets a new password when the password is forgotten.
- **Account.** The change of the password and a new recovery code, each with the present password; each makes a new code and the old one stops working.
- **Evaluation plan.** Standards imported as OSCAL catalogs; plans that name their standards and the places of the documents of the product; work units made from the controls, with their dependencies, documents, proofs of the laboratory, guidance and the agent that will assess them; the order of the work and the cycles of dependencies found (Kahn's algorithm). **Fill with AI:** the agent of the planner fills the parameters, then the dependencies, of every work unit.
- **Evaluation.** An evaluation begins on a copy of a plan, with the identification of the product, the client and the evaluator, and the agent of the evaluation. The documents of the product (PDF or text) and the proofs of the laboratory are given once and then frozen; the assessment does not start while one that a work unit needs is missing. **Assess with AI:** every work unit in the order of the work, autonomously (it may run overnight), with a preview of the questions and "Stop"; each question carries the work unit, the results of its dependencies, its documents (whole, or the passages that fit it best) and its proofs. The program checks every quote word for word in its document and warns of each dependency that did not pass; the verdict is the model's. The list of the work units with filters and pages; a work unit with its results, which the evaluator may correct (then the AI never writes them again), and the last question and answer of the AI.
- **Reports.** The report of an evaluation written by a template: any text format (Markdown, HTML, LaTeX) with markers of the fields of the models (`{{work_unit.findings}}`) and blocks for the lists of an evaluation (the work units in the order of the work, with a filter of the verdict); shown in the window and exported into a file. Templates are made from the default one or imported from a file, changed in an editor that puts a marker at the cursor, and written back into a file.
- **Agents.** An agent is a layer of knowledge over the model of an integration: its role (evaluator, planner or chat), its instructions, shared knowledge bases (`.txt`, `.md`, `.pdf`, searched on this computer), a glossary, its limits, the templates of every text of its questions, and its own journal of every question and answer.
- **Chat.** A conversation with an agent chosen by the user, its knowledge searched for each question, the answer streamed into the window and stopped at once on "Stop"; a long chat leaves its oldest messages out of the question. Chats are kept, never removed.
- **AI settings.** Integrations with AI services (the chat completions of OpenAI, which local servers offer, the messages of Anthropic, or a custom format: the JSON of a service's own, written by the user with placeholders): HTTPS with TLS 1.3, or plain HTTP only on this computer; the API key only in its header, never shown back.

## Requirements

Debian 13 (trixie), the target system and the only one tested, with a graphical session (XFCE on X11 is tested).

| Needed for | Debian 13 (`apt`) |
|---|---|
| running | `poppler-utils bubblewrap` |
| building | `build-essential pkg-config libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev libdbus-1-dev nodejs npm` |
| tests in the window | `xvfb xclip imagemagick x11-utils libxtst6 python3` |

Rust (stable, edition 2024) from rustup (https://rustup.rs), then `rustup default stable`; not the `rustc` and `cargo` packages of the system beside it. Node.js 20.19 or newer.

## Building and testing

```bash
git clone https://github.com/thekamik/io-master.git
cd io-master/io-master
npm ci                 # once: the tools of the page
npm run check          # svelte-check of the page
npm run build          # the page, built into the program
cargo build
cargo test
./target/debug/io-master
```

The program is started without parameters; everything is set in its window.

## Architecture

The program is a desktop window (Tauri 2) over a Rust core. The page (`io-master/ui/`, Svelte 5) holds only presentation and calls a fixed list of commands; no network port is opened. The Rust code (`io-master/src/`) is parted into three groups, and the dependencies go one way:

```text
panels/  ──►  objects/  ──►  infrastructure/
the window    the models and    what belongs to no object: the database files and their
              the functions     encryption, random bytes, PDFs read with Poppler, the clock
              on them
```

- **`objects/`**: one folder per model, holding the model and every function that works on it. The files are named by the object and their role:

  | File | Holds | May use |
  |---|---|---|
  | `user_data_model.rs` | the structure, fields only | nothing |
  | `user_data_store.rs` | its tables, `create_`, `read_`, `update_` (the only place with SQL) | the models, `infrastructure/storage/` |
  | `user_data_rules.rs` | the rules of its data | the stores, the models, `infrastructure/` |
  | `user_data_ai.rs` | the work of the AI on it | the rules, the stores, the models |

  A role that grows becomes a folder of the same name (`user_keys_rules/password.rs`). An object has only the roles it needs.
- **`infrastructure/`**: knows no object (`storage/`, `security/`, `pdf/`, `clock.rs`, `data_folder.rs`).
- **`panels/`**: `views/<panel>/` the commands the page calls (the only code that knows Tauri and serde), `logic/<panel>/` only what a panel shows.

**Where a new function goes:** with the model it works on, in the file of its role. A function is never a universal helper shared by several models.

The modules follow Rust 2018: every folder has a file of the same name beside it (`objects.rs` for `objects/`) that declares its modules; `io-master/src/main.rs` declares the groups and starts the window. The rules of the code ("The Power of 10", adapted to Rust) are in `CLAUDE.md`. More: `docs/architecture.md` (what and why), `docs/data_model.md` (the models), `docs/code_structure.md` (where everything is), `docs/architecture_map.md` (each model from the database to the window).

## Dependencies

Few, well known, pinned; each explained where it is declared (`io-master/Cargo.toml`, `io-master/package.json`).

| Rust crate | Version | Why |
|---|---|---|
| `tauri` | 2.12.4 | the desktop window with a web view (WebKitGTK), commands without a network port |
| `tauri-plugin-dialog` | 2.8.3 | the dialogs of the system to choose files, called only from Rust |
| `tokio` | 1.53.3 | the asynchronous runtime under the window and the requests to the AI |
| `reqwest` | 0.13.5 | the HTTPS client of the AI integrations (rustls, TLS 1.3, no proxy) |
| `rusqlite` | 0.40.2 | SQLite with SQLCipher built in: every database encrypted with AES-256 |
| `argon2` | 0.6.0 | the password turned into a key (Argon2id) |
| `aes-gcm` | 0.11.1 | the key of the database wrapped with the password and the recovery code |
| `hkdf`, `sha2` | 0.13.0, 0.11.1 | keys derived from one secret; the check sum of the recovery code |
| `zeroize` | 1.9.1 | secrets overwritten in memory when no longer needed |
| `rustix` | 1.1.5 | random bytes from the kernel and no memory dumps, without unsafe code |
| `serde_json` | 1.0.154 | JSON read and written by hand: the AI, the OSCAL catalogs |
| `serde` | 1.0 | the views of the page as JSON, at the boundary to the window only |
| `jsonschema` | 0.58.5 | every OSCAL catalog checked against the JSON schema of NIST, built in |
| `rcgen`, `tokio-rustls` (tests) | 0.14.10, 0.26.6 | a fake AI service with real TLS 1.3 on 127.0.0.1 |
| `tauri-build` (build) | 2.7.3 | the permissions of the commands, made at build time |

| npm package | Version | Why |
|---|---|---|
| `svelte` | 5.57.2 | the page |
| `@tauri-apps/api` | 2.12.4 | the commands called from the page |
| `vite`, `@sveltejs/vite-plugin-svelte` | 8.3.4, 7.3.1 | the page built into `ui/dist` |
| `typescript`, `svelte-check`, `@tsconfig/svelte` | 5.9.3, 4.7.6, 5.0.8 | TypeScript strict, checked before every commit |
| `@tauri-apps/cli` | 2.12.3 | the tools of Tauri |

The system packages are in **Requirements** above; Poppler (`pdfinfo`, `pdftotext`) is run as another program to read PDFs, each call in a sandbox of `bubblewrap` (`bwrap`: no network, no home folder, the system read-only), since the PDFs come from outside the laboratory (`io-master/src/infrastructure/pdf/poppler.rs`).

## The repository

```text
io-master/          the program (Rust, Tauri 2 + Svelte 5)
  src/              infrastructure/, objects/, panels/
  ui/               the page of the window, a separate project
  oscal/            the JSON schemas of OSCAL 1.2.3 and example documents of NIST
  report_templates/ the default template of a report (Markdown), a new template's text
docs/
  architecture.md   what the program is and why: the user, the keys, the AI, the standards
  data_model.md     the models, their links and their rules
  code_structure.md where everything is in the code, and the rules that keep it in order
  architecture_map.md each model: where it is stored, processed, reached by a command and shown
  todo.md           the open work
  handover.md       the state of the work, for a new working session
testing_documents/  documents for trying the program by hand (the CEM:2022 as an OSCAL catalog, the script that made it)
review.md           the state of the code review
```

## License

GNU General Public License, version 2 (`LICENSE`).
