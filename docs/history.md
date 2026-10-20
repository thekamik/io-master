# IO Master: the steps already built

The steps of `todo.md` that are built, moved here on 2026-10-17 (step 40) so that the plan stays short. Each keeps its decisions, its tasks and its state as they were when it was built; **the paths of the code they name are those of their time** (before step 40 the code had other folders: `evaluation/`, `test_projects/`, `standards/`, `installation/` at the top, `ai/chat/`, `db/ai/memory/`, ...; today's layout is in `code_structure.md`). The number of a step is its name, as in `todo.md`.

---

## Step 0: Put the current state in order

- [x] Commit what exists today to `v0.3`: the new folder layout (`cli_ui/`, `web_ui/`, `db/`, `keyring/`, `permission/`), `keyring.rs`, `database_key.rs`, `db/generate_key.rs`, `docs/`.
- [x] Remove the console interface: `cli_ui/`, the `--cli` parameter, its tests.
- [x] Remove empty files that no step needs yet, or keep them with a comment saying which step fills them.

**Done when:** the working tree is clean, `v0.3` on GitHub matches it, and the program builds without the console.

---

## Step 1: Web server skeleton

**Decisions**
- [x] Web framework: `axum` on `tokio`.
- [x] No TLS for the web UI: plain HTTP on 127.0.0.1 only. TLS 1.3 only where data leaves the computer (master, AI, git).
- [x] How the server is started: flags. `--port N` (default 8080); `--address IP` always means the TLS server (step 4); `--data-dir DIR` (default `~/.local/share/io-master`).
- [x] The role is chosen at the first start, in the web interface, together with the first user, unlocked by a one-time setup code printed in the terminal (`architecture.md`, section 3.2).
- [x] Pages: plain HTML and CSS built into the program, no separate frontend build.

**Tasks**
- [x] First start: the setup page (setup code, first user, password twice, role, confirmation); afterwards the page is gone and the first user is logged in. **TODO: until steps 2–3 the state is kept in `installation.json` in the data directory; the password is checked on the form but not kept.**
- [x] A damaged installation file stops the program.
- [x] Flags `--port`, `--address` and `--data-dir`; `--address` is recognized and refused with an explanation until step 4.
- [x] HTTP server listening only on `127.0.0.1`.
- [x] Protection of a local server: only the `Host` names of the program are answered (DNS rebinding); requests that change something need the `Origin` of the program (cross-site requests).
- [x] Layout: navigation on the left, the page on the right.
- [x] Pages of the workstation: Chat (placeholder), Scan and permissions (placeholder), Settings (placeholder).
- [x] Page of the master: password reset panel (placeholder). Each role refuses the pages of the other.
- [x] Red bar on every page while no password reset is configured (always, for now).
- [x] Mock-up login: a login page and a session cookie (`HttpOnly`, `SameSite=Strict`); only users of the installation are accepted, the password is not checked yet. **TODO: real login in step 5.**
- [x] Security headers: Content-Security-Policy, no framing, no MIME sniffing, no referrer.
- [x] `main.rs` reads the configuration and starts the server; Ctrl+C stops it cleanly.
- [x] The random bytes of the kernel moved to `random/random.rs`, shared by the database key and the session tokens.

**Done when:** the browser opens the pages on `http://127.0.0.1:PORT/`; tests cover the configuration, routes, role blocking, the Host and Origin checks and the headers.

---

## Step 2: Secrets in the TPM

> **Changed after step 3:** the TPM is an **optional, further factor**, not a requirement, because the program must also run on machines without a TPM (many virtual machines). By default the program uses passwords only. The TPM modules built here stay and are switched on as a factor in step 8.

**Needs:** packages `libtss2-dev` and `swtpm` (installed). Until the beta version the program uses only the emulator; access to the real TPM (group `tss`) is needed from step 8.

**Decisions**
- [x] Library: `tss-esapi`.
- [x] The secret of the device: 32 random bytes sealed under the primary key of the TPM (derived by the TPM on every use, never stored). The sealed envelope is a file in the data directory, readable only by the user; it is useless without the TPM of this computer.
- [x] A marker in the non-volatile memory of the TPM (one small index, written once at installation) holds the identifier of the installation; it tells the states of `architecture.md`, section 3.2 and refuses a second installation.
- [x] No binding to the state of the machine (PCR) for now.
- [x] The owner password of the TPM is assumed empty; the program never clears the TPM or changes its passwords.
- [x] Until the beta version the program uses only the emulator `swtpm`, which it starts itself in the data directory; the real TPM comes in step 8. The tests start their own emulators.

**Tasks**
- [x] The TPM emulator (`tpm/emulator.rs`): started and stopped by the program, its state in `<data directory>/tpm-emulator/`; an emulator left running after a crash is stopped at the next start.
- [x] Access to the TPM (`tpm/tpm.rs`): encrypted sessions, sealing and unsealing, the marker in the non-volatile memory.
- [x] The secret of the device (`tpm/device_secret.rs`, `architecture.md` section 5.2) created and sealed at the first start; unsealed at every start to prove the installation.
- [x] The states of `architecture.md`, section 3.2 (`installation/lifecycle.rs`): a marker without a record, a record without the marker, a record of another installation, or an interrupted first start stops the program.
- [x] One installation per TPM: a second installation is refused. With the emulator every data directory has its own emulated TPM, so this protects a computer only with the real TPM (step 8).
- [x] `io-master --uninstall` (asks for a typed confirmation): removes the marker from the TPM, the sealed secret, the record and the emulator state; works on a damaged installation too.
- [x] Tests against `swtpm`, which never touch the real TPM.
- [x] Remove the Secret Service keyring (`keyring/`); the type of the database key moves to `db/database_key.rs`.

**Done when:** the secret survives a restart, cannot be read without the TPM, and the tests pass on `swtpm`.

---

## Step 3: Databases, accounts and the management of users

**Needs:** SQLCipher (installed; bundled or system library decided below).

**Decisions**
- [x] Database: SQLite encrypted with SQLCipher (`architecture.md`, section 5.3).
- [x] Two levels: the system database (key sealed in the TPM only) and the databases of the evaluators (password + secret of the device).
- [x] SQLCipher bundled into the program (`rusqlite`, feature `bundled-sqlcipher`), on the OpenSSL of the system.
- [x] One database per evaluator (`users/<id>.db`) next to the system database (`system.db`).
- [x] Argon2id with 64 MiB and 3 passes (OWASP); the key of a database is wrapped with AES-256-GCM under a key derived (HKDF-SHA256) from Argon2id(password) and the secret of the device, and bound to the account name.
- [x] Password rules: 12 to 1024 characters, nothing else (NIST).

**Tasks**
- [x] Encrypted databases (`db/db.rs`): files readable only by the user; every new database holds its schema version at once, so even an empty one opens only with its key.
- [x] System database (`db/system_db.rs`): key derived from the secret of the device, opened at program start without a password (`architecture.md`, section 5.3). It is the record of the installation; `installation.json` is gone.
- [x] Creating an account (`accounts/accounts.rs`): password rules, a random key for its own database, wrapped with the password and the secret of the device (`db/keys.rs`).
- [x] Login unlocks the key of the database of the evaluator; the session keeps it and logout wipes it. A wrong name and a wrong password get the same answer. This replaces the mock-up login of step 1.
- [x] Change of password (Settings): needs the current password, re-wraps only the key of the database.
- [x] Management of users (Settings): on a workstation a logged-in user adds a further user (equal rights); a master keeps its one user.
- [x] Uninstall also removes the databases.
- [ ] Settings kept in the database of the evaluator: moved to step 5, the first setting is the choice of columns.
- [ ] Changing the password on a master: comes with the master (step 11).

**Done when:** without the password the data cannot be read, also with the TPM present; tests cover creating, opening, changing the password, a wrong password, and adding a user.

---

## Step 5: Scan page

Done before step 4, at the request of the user.

**Decisions**
- [x] The page is built by the server (HTML forms, no JavaScript); a JSON API and `web_ui/serializer/` come when a page needs live data.
- [x] The settings of the scanner are on the scan page itself, kept per evaluator in their encrypted database.
- [x] No CSRF token: the check of `Origin` and the `SameSite=Strict` cookie already refuse forms of other websites (step 1).

**Tasks**
- [x] The scan filters by several kinds of port at once, or all (`devices::PortFilter`).
- [x] The properties of a port that can be shown (`devices/port_property.rs`), each with a stable key for storing the choice; default columns: port, type, device, serial number, access.
- [x] The settings of the scanner (`devices/scan_settings.rs`) in the settings table of the evaluator's database (`db/user_db.rs`); forgiving when reading.
- [x] Scan page: kinds of port (with "All kinds") and columns as boxes, "Save and scan", "Scan again", the table of ports with the chosen columns.

**Done when:** the Arduino appears in the table; the chosen settings survive a new login and a restart of the program, and each evaluator has their own; tests cover the filter, the properties, the settings and the page.

---

## Step 7: AI chat with Anthropic

The first AI function: a chat with a Claude model of Anthropic, with the API key of the evaluator. Other providers (Gemini) and the use of AI on the data of devices come later.

**Decisions**
- [x] HTTP client: `reqwest` with `rustls`, TLS 1.3 only, certificates verified, redirects never followed, as in v0.2, so the key never reaches another address.
- [x] Settings of the chat on the chat page itself, folded like those of the scanner (step 5), kept per evaluator in their database: the model, chosen from a list of the current Claude models (Opus, Sonnet, Haiku), with a default.
- [x] Replies are shown while they are written (streamed): friendlier for the user. This needs a small script on the chat page, served from the program itself (the Content-Security-Policy allows no other), and server-sent events from the server.
- [x] The key and the chosen model are checked when they are stored, as in v0.2: one short request; a failure is explained in plain words (key rejected, unknown model, no connection) and the key is not stored.
- [x] What the page shows after the check: only that the model is configured correctly and can be used in the chat or to support the evaluation; nothing of the key itself, not even its last characters.

**Tasks: the API key (Settings)**
- [x] A section "AI" in Settings: entering, replacing and deleting the Anthropic API key of the logged-in evaluator. The field is a password field; the key is **never shown back** in a page, a log or an error message.
- [x] The key is stored only in the evaluator's database, which is encrypted with a key opened by their password (and by every further factor, steps 8 and 9): **at least the password protects it**. It is never written to a file or to the system database.
- [x] Transport of the key from the browser: only to the local interface on 127.0.0.1 (later over TLS 1.3, step 4); the form is protected like every form (Origin check, `SameSite=Strict`).
- [x] In memory: the key is held in a type that wipes it and never prints it (as `DatabaseKey`), read from the database only for a request and dropped right after it.

**Tasks: the chat**
- [x] AI module (`ai/`): sending a conversation to the Anthropic Messages API, reading the reply and the token counts; errors told apart (key rejected, limit reached, no connection, refusal, overloaded service).
- [x] Chat page: list of the evaluator's chats, new chat, continue, delete; every message and reply kept in the evaluator's database with its time and model.
- [x] A reply is shown while it is written; it can be stopped while it is being written.
- [x] Tests against a fake Anthropic service, without a real key and without network.

**As built**
- `ai/`: `api_key.rs` (the key: wiped, never printed), `model.rs` (Opus 5.5 default, Sonnet 5.5, Haiku 5.5), `anthropic.rs` (the client: check, streamed reply, stop), `sse.rs` and `reply.rs` (reading the stream), `ai_error.rs` (kinds of failure), `ai_settings.rs` (key and model in the database), `chats.rs` (history).
- Database of the evaluator: new tables `secrets`, `chats`, `messages` (`db/user_db.rs`).
- Pages: the AI section of Settings (`web_ui/settings.rs`), the chat page (`web_ui/chat_page.rs`) with its script `web_ui/static/chat.js`.
- With Opus the request asks Anthropic for its server-side fallback (`fallbacks: "default"`): a request declined by mistake by the safety checks is answered by another Claude model; the reply names that model. The settings of the chat say so.
- The thinking of the model is not shown and not kept; only the text of the replies is kept and sent back.
- A refused reply is kept only as the fact (its partial text is dropped, as Anthropic advises); a stopped or failed reply keeps its text and is marked.
- Tests: a fake Anthropic service over real TLS on 127.0.0.1 (`ai/test_support.rs`); the client refuses TLS 1.2, a foreign certificate and redirects.

**Agreed for the AI (kept for the later AI steps)**
- One provider for now: Anthropic, Messages API over raw HTTPS (Rust has no official SDK). Other providers (Gemini) come later, behind the same kinds of result and error (`ai/ai_error.rs`).
- Models: only the current model of each family, plain API names without a date (`claude-opus-5-5` default, `claude-sonnet-5-5`, `claude-haiku-5-5`). A stored model this version no longer offers falls back to the default.
- The key and the model are checked with one short request before they are stored; a key or model that fails is never stored. The pages say only whether a key is stored and that the model is configured correctly, never anything of the key.
- The key lives only in the table `secrets` of the evaluator's encrypted database; removed secrets and chats are overwritten in the file (`secure_delete`).
- Connection: HTTPS only, TLS 1.3 or newer, certificate verified against the system's trusted certificates, no redirects, no proxy of the environment. Timeouts: 15 s to connect, 120 s of silence during a reply, 60 s for a check.
- Replies stream to the browser as server-sent events; the script of the page is served by the program (CSP `'self'`). The text of a reply is shown as plain text, never as HTML.
- "Stop", or a browser that leaves, stops the reply; the text written so far is kept and marked "stopped". A failed reply keeps its text and the explanation; a cut reply (largest length) is marked.
- Refusals: a refused reply keeps only the fact, its partial text is dropped (as Anthropic advises). With Opus the server-side fallback is asked for (`fallbacks: "default"`, beta `server-side-fallback-2026-07-01`); the chat settings tell the evaluator; Sonnet and Haiku without it.
- The thinking of the model is left to its defaults (adaptive), not shown and not kept; only the text of earlier replies is sent back. Two messages of the evaluator in a row (after a failed reply) are joined into one.
- Largest reply 64000 tokens; largest message of the evaluator 100000 characters.

**Open for later AI steps**
- [ ] Show replies with formatting (Markdown) through a safe renderer that allows no HTML.
- [ ] Ask Anthropic whether Sonnet 5.5 supports the server-side fallback; switch it on if it does.
- [ ] A proxy of the laboratory, if one is needed: only with TLS kept end to end (CONNECT) and an explicit setting, never from the environment.
- [ ] Keep the thinking blocks of earlier replies and send them back unchanged, if the quality of long conversations needs it (see "preserved thinking" in the documentation of Anthropic).
- [ ] Token counts are stored per reply but not shown yet; a view of the cost per chat.
- [ ] Documents of the evaluation as input to the model (instead of very long messages), and AI support of the evaluation itself (`v0.2_functionality.md`, section 3.7).

**Done when:** an evaluator stores an API key in Settings, the key cannot be read from the files without the password, a conversation with a real Claude model works from the browser, and the tests pass without network.

---

## Step 14: The user interface: web or native (decision and prototype)

Raised by the user on 2026-10-15. The goal of the tool is the AI-assisted Common Criteria evaluation, and the analysis is done on graphs:

1. the norm (CC, CEM) and the documentation of the manufacturer are parsed into pieces ("tiles": elements of the norm, passages of the documents);
2. the evaluator connects the tiles into a pipeline of dependencies (a node graph: which evidence answers which element, which element depends on which);
3. the AI model analyses the dependencies and proposes an assessment, with its reasons (principle P1: the evaluator decides).

This needs a rich interface: a node editor with zoom and many nodes, views of long documents, charts. The user expects the web interface to make this hard and proposes a native interface.

**Requirement added by the user (2026-10-15): the original PDF documents are shown well, with the passages used by the AI highlighted**, so that the evaluator can check what the AI did against the source (principle P3, traceability). Parsed fragments alone are not enough.

Candidates after the first discussion: **Tauri 2** (Rust core, the interface in a web view without a network port; pdf.js for the PDF and its highlights, a node editor such as Svelte Flow or React Flow) and **egui** (Rust only; `egui-snarl` for the graph, PDFium through `pdfium-render` for the pages, highlights drawn over them). The prototype has to show the PDF with a highlighted passage, not only the graph.

**Decisions**
- [ ] Native or web; if native, which toolkit. Options and their trade-offs are proposed by Claude in the concept (conversation of 2026-10-15).
- [ ] What happens to the parts that exist only because of the local web server: the Host and Origin checks, the session cookie, the one-time setup code, `--address` and the TLS server of step 4 (a remote interface contradicts "the evaluator works at the computer").
- [ ] What stays unchanged: the core (`ai/`, `db/`, `accounts/`, `devices/`, `installation/`, `tpm/`), which already knows nothing of the web (CLAUDE.md, "core logic is separate from presentation").
- [ ] The network interface of the master (step 11) is separate from the user interface and is not decided here.

**Tasks**
- [ ] Concept approved by the user.
- [ ] A small prototype of the chosen interface: a window with a node graph of a few tiles that can be connected, and one long text; tested by the user before the decision is final.
- [ ] Update `architecture.md` (section 4 and every mention of the web interface), `CLAUDE.md` (main design rule) and this list (steps 1, 4, 5, 7 and 12 that assume the web).

**Decided (2026-10-16):** the user chose **Tauri 2 + Svelte 5** and started the branch `v0.4` for it; the whole interface of v0.3 was ported first, the graph and PDF prototype follows.

**As built (the port)**
- `src/tauri_ui/`: (the window is opened by `tauri_startup` in `main.rs`, moved there from `startup.rs` during the review, 2026-10-16), `state.rs` (installation, session with the key of the database, the reply being written; all in Rust, nothing secret in the page), `commands.rs` (the 16 commands the page may call), `*_actions.rs` (the work, testable without a window), `serializer/` (the only place where data becomes JSON), `error.rs`, `user_data.rs` (every command on the data of the evaluator checks the login first), `blocking.rs`.
- `ai/conversation.rs`: preparing a message of the chat, moved out of the web page into the core.
- The page: `ui/` (Svelte 5 + TypeScript, built by Vite into `ui/dist`), `ui/src/lib/api.ts` (all calls to Rust in one file).
- Security: no network port; the Content-Security-Policy of `tauri.conf.json` (own scripts and styles only); the window may call only the listed commands (`build.rs` makes the permissions, `capabilities/default.json` grants them); text is always shown as text (Svelte escapes it, `{@html}` is not used).
- Left out on purpose: the one-time setup code (it protected the network port; the window has none), adding users (one user per Linux account, step 13).
- `src/web_ui/` was removed by the user (2026-10-16); the web interface of v0.3 is in the branch `v0.3`.

**Building** (once: `npm ci` in `io-master/`):

```text
npm run check     the types of the page (svelte-check)
npm run build     the page into ui/dist (needed before cargo build)
cargo build       the program, with the page built in
cargo test        the tests of Rust
```

**Open**
- [ ] The prototype of the graph of tiles and of the PDF with a highlighted passage (the reason for the choice).
- [x] `--port`, `--address` and the parser of the command line removed (2026-10-16); the program refuses any parameter. Step 4 reduced to the TLS server of the reset service; `architecture.md` section 4 rewritten.
- [x] `src/web_ui/` removed (2026-10-16). Left over: the module block commented out in `main.rs`; the dependencies only the web interface used; core functions only it called (adding users, the list of user names), which step 13 removes anyway.
- [ ] Tests of the page itself (end to end through the window); so far tested by hand on a virtual display.

**Done when:** the user has decided, the prototype was tried, and the documentation describes the chosen interface.

---

## Evaluation schemes (steps 16 to 22)

> **Replaced by step 41 (2026-10-18).** The model of the evaluation schemes (tiles and links) was replaced by NIST OSCAL and removed with its tables and data in step 41.1 (no migration; `oscal.md`, `todo.md`). What was kept: the loading of the PDFs of a standard and their text by pages, the viewer with the highlighted passages, the check of a quote on its page, and the work of the AI part by part.

Raised by the user on 2026-10-16, with the highest priority: the program does not go further (not even to the review, step 12) until these steps are done. Decisions of the user of the same day are marked (user).

**The idea.** An evaluator works against a **standard** (a norm): ISO/IEC 15408 with the CEM, but also any other (for example ETSI EN 303 645, IEC 62443-4-2, EN 18031, FIPS 140-3). Nothing is written for Common Criteria; the functions are general. From the documents of a standard, with the support of AI, an **evaluation scheme** is made: the plan of the evaluation (the plan of the tests), drawn as **tiles** joined by **links**. A head of the laboratory can make a scheme and give it to the evaluators (export and import, step 22).

```text
documents of a standard (PDF)  ──load──►  text by pages  ──AI proposes, the user corrects and draws──►  scheme
                                                                                                        │ export / import
                                                                       later (step 23): evaluation ◄────┘
```

**Now: making schemes (steps 16 to 22).** The tool **Create evaluation scheme** loads the documents of a standard, analyses them with AI, and lets the user draw and correct the scheme. **Later: carrying a scheme out (step 23).** The tool **Evaluation** (in the sidebar on the left) works through a scheme for one product with the help of AI; its results and verdicts belong there, and the final decisions stay with the evaluator (user). It is designed when steps 16 to 22 are done.

**Three kinds of tile** (user):

| Kind | What it is | When it is met |
|---|---|---|
| **Input** | what the evaluation needs: documentation of the manufacturer, a sample, access, the result of another requirement, an element described in another document of the standard | when it is provided (in the evaluation, step 23) |
| **Requirement** | what the standard imposes: a criterion, an activity, a test | when **all** its inputs are met |
| **Output** | the result in a standard form for the report: the tests carried out, the set of evidence, the references to the standard | made from the requirements it reports (in the evaluation) |

**Links** (proposed): an input *is needed by* a requirement; a requirement *is reported in* an output; an input may be *provided by* another requirement (a test that needs another test done first). Where a tile sits in the standard (clause, identifier such as ADV_FSP.4) is a field of the tile, not a kind of its own.

**Standards in several documents** (user): a standard often comes in parts (CC Part 1 to 5 and the CEM). A scheme may use several documents. When a requirement needs something described in a document that is not loaded, the analysis makes an **input to be completed later**, naming that document and the place in it.

**Sources.** Every tile keeps where it comes from: document, page and the exact quoted text, so that every proposal of the AI is checked in the highlighted page (the reason for the desktop window, step 14). A quote that is not in the text of its page is marked.

**Principles** (`architecture.md`, Part 1, P1 to P5): the AI proposes, the user decides (no tile enters a scheme without the user); everything is traceable to a passage; the standard is data, never code.

**Decisions (user, 2026-10-16)**
- [x] Three fixed kinds of tile: input, requirement, output (above).
- [x] Standards are loaded from PDF; the analysis at the import is done with AI, which proposes the scheme as tiles; standards in several documents (above).
- [x] The text of a PDF is read with `pdftotext` of Poppler (a system package, called as a separate program), page by page.
- [x] The viewer: pdf.js of Mozilla (Apache 2.0), built into the window, no network.
- [x] The drawing: Svelte Flow (`@xyflow/svelte`, MIT).
- [x] Standards, schemes and their graphs are kept encrypted, in the database of the evaluator.
- [x] **The program does not limit where the text of a standard goes.** Sending parts of a licensed standard to an AI service is the decision of the laboratory; the program sends it over the connection the user chooses, without a warning or a block. The documentation describes a safe configuration (a local model) as a recommendation.
- [x] Verdicts belong to the evaluation (step 23), not to the scheme.
- [x] Schemes are exported and imported, so that a head of the laboratory can give them to the evaluators (step 22).

- [x] The laboratory is trusted (user): a certified laboratory is controlled by others, so the program does not check its honesty: no signatures of schemes. An exported scheme may carry the PDFs of its documents, if the laboratory wants (an option of the export); the SHA-256 of a document only finds the right copy for the quotes.
- [x] **The form of an output** (proposed by Claude, accepted by the user): six sections fixed by the program, the same in every scheme, so that reports look alike:
  1. *Requirements reported*: identifier, title, place in the standard, the quoted sources.
  2. *Inputs*: what each requirement needed, and what was provided.
  3. *Activities and tests carried out*: method, steps, tools and their configuration, date, who.
  4. *Evidence*: what, where it is kept, its checksum.
  5. *Findings*: observations, deviations, open points.
  6. *Verdict and rationale*: per requirement, given by the evaluator.
  7. *Vulnerabilities and notes*: the vulnerabilities that may follow from the solution used (added by the user on 2026-10-17, one of the four results of a requirement in the evaluation).

  The scheme gives each output its title, the sections that apply (all by default) and the instructions for the report (what the laboratory expects written there). Sections 2 to 7 are filled by the evaluation (step 23).
- [x] All steps 16 to 22 are to be built one after another (user, 2026-10-16); each is tested on its own before the next starts.

**State (2026-10-16): steps 16 to 22 built and tested; waiting for the user's test.**
- 268 tests pass; `npm run check` clean; `cargo clippy` only the known dead-code warnings.
- Tested in the window (private Xvfb display): loading a PDF through the dialog of the system, the viewer with a highlighted hit, the search, a new scheme, the analysis with a fake local model (7 proposals, 6 links, an invented quote marked "not on the page"), deleting and accepting proposals, a refused link (input → output), a link drawn by hand, Undo, a quote selected in the viewer, export with the PDF, import of the same file ("(imported)").
- **As built**
  - Core `src/evaluation/`: `quote.rs` (one normal form of text, with the places of its bytes for snippets), `scheme.rs` (kinds, links, rules, the six sections of an output), `documents.rs`, `pdf_text.rs` (Poppler, the PDF on standard input, in parts of 20 pages with progress and stop), `schemes.rs` (storage; `replace_content` for Undo and import), `checks.rs`, `exchange.rs` (JSON v1, optional PDFs in base64), `analysis.rs` (parts of the text, the prompt, reading the answer, applying the proposals; instructions editable, setting `evaluation.analysis_instructions`).
  - Window layer: `tauri_ui/standard_actions.rs`, `tauri_ui/scheme_actions.rs` (Undo stack of 50 changes per scheme in `UiState`, one Undo for a whole analysis), `serializer/standard_view.rs`, `serializer/scheme_view.rs`; 27 commands; the dialogs of the system only from Rust (`tauri-plugin-dialog` 2.8.3); new dependency `base64` 0.22.1.
  - Page: `Schemes.svelte`, `SchemeEditor.svelte` (Svelte Flow 1.7.0), `TileNode.svelte`, `TilePanel.svelte`, `PdfViewer.svelte` + `lib/pdf.ts` (pdf.js 6.3.289, loaded only when the viewer opens; its maps of characters and fonts copied into the build by `scripts/copy-pdfjs.mjs`), `AnalysisPanel.svelte`, `Evaluation.svelte` (placeholder of step 23).
  - `vite.config.ts` patches `d3-color/src/define.js` (used by Svelte Flow) so that it works with the frozen prototypes of the window (`freezePrototype` stays on); the patch refuses to apply to a changed file.
  - `ui/src/main.ts` shows an error of the start in the window, with its stack, instead of an empty window.
- **Open:** the user's test with real standards and a real model; the size of the editor on small windows; the evaluation itself (step 23).
- Not committed.

---

## Step 16: Evaluation schemes: the data model

The core types and their storage, without a window; everything later builds on them.

**Tasks**
- [x] Types (`evaluation/`): document of a standard (title, part, version, SHA-256 of the file, the text by pages); scheme (name, version, the documents it uses, author, state); tile (kind: input / requirement / output; identifier, title, description, place in the standard, sources: document + page + quote; state: proposed by AI / accepted; for an input of another document: that document and place, "to be completed"); link (from, to, kind); the position of a tile on the canvas.
- [x] Rules: links only between tiles of one scheme and only of the allowed kinds (input → requirement, requirement → output, requirement → input as its provider); no cycle; a quote must occur in the text of its page.
- [x] Storage in the encrypted database of the evaluator (new tables), with tests.

**Done when:** schemes with tiles and links are stored, read and checked by the rules; all tests pass.

## Step 17: Load a standard: documents and their text (tool Create evaluation scheme)

**Needs:** the package `poppler-utils` (`pdftotext`).

**Tasks**
- [x] The tool "Create evaluation scheme" in the sidebar: the loaded documents and the schemes.
- [x] Load a PDF: its text read page by page with `pdftotext`, the file kept encrypted; title, part and version entered by the user (proposed from the file).
- [x] Long work reports its progress and can be stopped (a standard has hundreds of pages).
- [x] Search in the text of the documents.

**Done when:** the parts of a real standard are loaded, their pages and text are shown and searchable.

## Step 18: The viewer of a standard with highlighted passages

**Tasks**
- [x] The PDF shown in the window (pdf.js, built in, no network), page by page.
- [x] A source (document, page, quote) highlighted in its page; from a tile to its source in one click.
- [x] Selecting text in the viewer gives a quote for a tile.

**Done when:** every source of a tile can be seen highlighted in the original page.

## Step 19: The drawing module: the scheme as tiles on a canvas

**Tasks**
- [x] The scheme drawn on a canvas (Svelte Flow): tiles as cards coloured by kind, links as arrows; moving, zooming, arranging automatically.
- [x] Tiles added by hand, edited (also those proposed by AI), deleted; links drawn and removed; the rules of step 16 checked at once.
- [x] Filters and search (kind, state, place in the standard, document), and a list view next to the canvas.
- [x] Undo of the last changes.

**Done when:** a scheme for one clause of a standard can be drawn completely by hand, and a proposed one corrected.

## Step 20: AI analysis at the import: a proposed scheme with verified quotes

**Tasks**
- [x] The analysis of the loaded documents (or of a chosen part of them) over the active AI connection: the model proposes tiles and links in a fixed JSON form, each with the quotes it is based on.
- [x] Every quote is checked against the text of its page; a proposal with a quote that is not in the document is marked and is not accepted without a correction.
- [x] Inputs described in a document that is not loaded become inputs to be completed later, naming that document and place.
- [x] Long documents are analysed in parts that fit the context of the model (local models have small ones); the parts are joined (the same requirement found twice is one tile); progress, stop, repeat of one part.
- [x] The proposals appear on the canvas as proposed tiles; the user accepts, corrects or rejects them, one by one or by group.
- [x] The instructions to the model are text the user can read and change (as the connections of step 15).
- [x] The documentation describes a safe configuration (a local model for licensed standards) as a recommendation.

**Done when:** a part of a real standard is analysed by a local model and by Anthropic, the proposals are checked against the highlighted passages, and the accepted ones form the scheme.

## Step 21: Checks of a scheme and the form of the outputs

**Tasks**
- [x] Checks: requirements without inputs, inputs needed by nothing, requirements in no output, inputs still to be completed from other documents, cycles, quotes not found.
- [x] The form of an output: what the report of a requirement will hold (the tests carried out, the evidence, the references to the standard), as the evaluation (step 23) will fill it.
- [x] A scheme is marked ready only when the checks pass or the user accepts the remaining warnings.

**Done when:** the checks point to the gaps of a scheme, and its outputs describe the report the evaluation will make.

## Step 22: Export and import of schemes

**Tasks**
- [x] Export of a scheme as a file (JSON with a version of the format): tiles, links, positions, sources; the documents named by title, part, version and SHA-256, and, if the user chooses so, the PDFs themselves.
- [x] Import: the file checked (format, rules of step 16); documents carried in the file are loaded; the others are found among the loaded documents by SHA-256, or the user loads them; the quotes are checked against them.
- [x] Versions: an imported scheme keeps its author and version; it is stored next to the earlier one, with "(imported)" in its name when the name is taken; the user deletes the one no longer needed.

**Done when:** a scheme made on one installation is imported on another with its own copy of the standard, and all its quotes are found.

## Test projects and chapters (steps 24 to 28)

> **Replaced by step 41 (2026-10-18).** The model of the test projects and their schemes per chapter was replaced by NIST OSCAL and removed with its tables and data in step 41.1 (no migration; `oscal.md`, `todo.md`). What was kept: the loading of the PDFs of a standard and their text by pages, the viewer with the highlighted passages, the check of a quote on its page, and the work of the AI part by part.

Raised by the user on 2026-10-16 while testing the analysis of a real standard. Decisions of the user of that day:

- [x] **The tool only maps the standard.** A project of tests records the requirements of a standard, their inputs, outputs, links and rules. Providing documents to inputs, carrying out tests and making reports belong to the tool **Evaluation** (step 23), not here.
- [x] **AI creates, the person corrects and approves.** The program is the layer between the AI and the person: the AI proposes the structure and the content; the user corrects by hand and accepts.
- [x] **A requirement may be the condition of another requirement** (step 24): the result of the analysis showed links requirement → requirement, which are refused today.
- [x] **One large scheme is split** (step 25): a large standard in one graph slows the window down (the animations stutter) and is hard to read. A **test project** holds the documents of the standard and a flat list of **schemes**, one per chapter of the standard; the window shows the list, the user picks a chapter and sees its graph. Nothing more (no tree, no filters by chapter) at this stage.
- [x] **Links between schemes** (step 26): in scheme X an input may be that a requirement of scheme Y is met; requirements can be moved between the schemes of a project.
- [x] **Names in English:** a project and a scheme each have their own name.
- [x] **No migration of the data of steps 16 to 22:** the user deletes the database of the test installation and starts anew; every function is tried again.
- [x] **Reports in the LaTeX template of the laboratory** (step 28, later): the results of an evaluation are written into the LaTeX template the laboratory provides, automatically by default. Until then a result is shown in the window.

```text
Test project            the documents of the standard, name, version, author, description, "ready",
 |                      export / import (one file = one project), the instructions of the analysis
 +-- Scheme (chapter)   number "9.1", name, place in the list
 |    +-- tiles         inputs, requirements, outputs: the graph of today
 +-- Links              belong to the project: they may join tiles of different schemes
```

## Step 24: A requirement as the condition of another requirement

**Tasks**
- [x] `scheme.rs`: a fourth meaning of a link, `requirement -> requirement` = *precedes* ("must be met before"); the message of a refused link and the description updated. The check of cycles already refuses A -> B -> A and A -> A.
- [x] `checks.rs`: a requirement whose only predecessor is another requirement is not "without inputs".
- [x] `analysis.rs`: the instructions to the AI name the new link.
- [x] Window: `api.ts` (the meaning `precedes`), `TilePanel.svelte` (its sentence), a line of its own style in the editor.
- [x] The link requirement -> input (*provides*) stays: it names a product of a requirement (a test report) used elsewhere; *precedes* is only the order of conditions.
- [x] A link the AI proposes twice (or one already drawn) is skipped without a message.

## Step 25: Test projects: a project holds the schemes, one per chapter

**Tasks**
- [x] Data model and storage: a project (the fields of today's scheme: name, version, description, author, documents, ready) and its schemes (number, name, order); tiles belong to a scheme; links belong to the project.
- [x] The checks per scheme (computed on the whole project); the project shows the sum of the findings. "Undo" is one stack per project (a change may touch two schemes); deleting a scheme forgets it.
- [x] Export and import of a whole project (a new version of the format).
- [x] Window: the list of projects; in a project the list of its schemes (by number); a scheme opens its graph.

## Step 26: Links between the schemes of a project

**Tasks**
- [x] A link may join tiles of two schemes of one project; the check of cycles works on the links of the whole project.
- [x] In the graph of scheme X, a tile of another scheme linked to a tile of X is shown as a read-only shadow with the name of its scheme; a click opens that scheme.
- [x] Moving a tile to another scheme of the project (in the panel of the tile); its links stay. A link to a tile of another scheme is made in the panel of the tile, or by dragging to a shadow.

## Step 27: AI: the table of contents as the list of schemes, the analysis per chapter

**Tasks**
- [x] The AI reads the table of contents of the standard and proposes the schemes: number, name, range of pages; the user corrects and accepts them like proposed tiles.
- [x] The analysis of a scheme sends only the pages of its chapter; the requirements already known in the whole project (identifier and title only) are listed, so that the AI links across chapters; a requirement named again is joined to the known one, not copied.
- [x] "Analyse chapters with AI": several schemes one after another, each on its own pages; one "Undo".
- [x] The pages of a chapter: its heading looked for in the text after the table of contents; where not found, the printed page moved by the offset of the found ones; the user corrects them in the scheme.

**State (2026-10-16): steps 24 to 27 built, waiting for the user's test.**
- 289 Rust tests pass; `npm run check` clean; `cargo clippy` only the known dead-code warnings.
- Tested in the window (private Xvfb display, a fake model): a PDF with a table of contents, a new project, the table of contents read (two proposed schemes, their pages found), accepted, both chapters analysed in one run (a link from a requirement of 10.1 to one of 14.1, the repeated link skipped quietly), the shadow tile and a click on it, the links of a tile with "Open", a tile moved to another scheme and "Undo" of the move.
- **As built**
  - Core `src/evaluation/`: `scheme.rs` (`Project`, `Scheme` = chapter with `PageSpan`, `Tile.scheme`, `ProposalState` for tiles and schemes, the link *precedes*), `projects.rs` (projects, their documents, `data` of a whole project), `schemes.rs` (schemes, tiles, links of a project, `move_to_scheme`, `replace_content` per project; the sources of many tiles read in one query), `contents.rs` (the table of contents: prompt, answer, `locate`, `spans`, `apply`), `analysis.rs` (prompt with the chapter and the requirements of the other chapters), `exchange.rs` (format `io-master-project`).
  - Database: tables `projects`, `project_documents`, `schemes` (with pages, state, position), `tiles`, `tile_sources`, `links` (of the project). **No migration:** a database of steps 16 to 22 is deleted by the user.
  - Window layer: `tauri_ui/project_actions.rs`, `tauri_ui/scheme_actions.rs`, `tauri_ui/model_run.rs` (one question to the model, shared), `serializer/project_view.rs`, `serializer/scheme_view.rs` (shadows); 55 commands; `tauri_startup` in `main.rs`.
  - Page: `Schemes.svelte` (the list of projects), `ProjectEditor.svelte` (the list of schemes, hideable; settings, export, ready), `SchemeEditor.svelte` (one scheme), `ShadowNode.svelte`, `ContentsPanel.svelte`, `ChaptersPanel.svelte`, `TilePanel.svelte` (links to other schemes, moving), `AnalysisPanel.svelte` (the pages of the scheme by default).
- Not committed.

## Step 37: The database layer by areas and CRUD

Raised by the user on 2026-10-17 during the review: the code of the database had spread over many files and escaped the overview of the user; as it carries the security of the data and must be audited, this is a critical error, fixed before further functions (CLAUDE.md, "Database code").

**Decisions (user, 2026-10-17)**
- [x] All SQL in `src/db/`, one subfolder per area of data; in each `schema.rs`, `rows.rs` and the files of CRUD (`create.rs`, `read.rs`, `update.rs`, `delete.rs`).
- [x] The names of the areas: `system/`, `user/`, `ai/`, `standards/`, `test_projects/`, `evaluations/` (Claude's; the user preferred them to their own sketch).
- [x] All code of encryption in `db/encryption/`, **moved as it is**: the user audits it and redesigns it later (a fault in the design is suspected). Only a test that pins the parameters of SQLCipher in effect is added; production code unchanged.
- [ ] Later (user): the encryption reworked; in production the ciphers work on secrets.

**As built (2026-10-17)**
- `db/error.rs` (`DbError`); `db/encryption/`: `sqlcipher.rs` (the only place a database file is opened or created, encrypted or plain; its head lists the parameters in effect: SQLCipher 4.14 community, OpenSSL of the system, AES-256-CBC, HMAC-SHA512 per page, pages of 4096 bytes, the key as raw bytes, `cipher_memory_security` off), `keys.rs`, `database_key.rs`, `generate_key.rs`; test `the_parameters_of_the_encryption_are_those_described`.
- `db/system/` (system.db, not encrypted: `database.rs` the file and the lock, `create/read/update/delete.rs` as methods of `SystemDb`), `db/user/` (`database.rs` opens the file and creates the tables of all areas; settings and secrets; new `remove_setting`), `db/ai/`, `db/standards/`, `db/test_projects/`, `db/evaluations/` (the journal: written only by `create.rs`, read by `read.rs`; its tests of the triggers in `create_tests.rs`).
- The modules of the domain (`ai/`, `evaluation/`, `product_evaluation/`) keep their API and their rules and call `db/`; the callers (window, tests) are unchanged.
- **The compiler keeps the rule:** `UserDb::connection` is `pub(in crate::db)`, `SystemDb::connection` `pub(super)`; SQL outside `db/` does not compile. Two tests that damage a database on purpose use `connection_for_tests` (compiled only in tests).
- Changed in behaviour: none. `replace_content` checks the rules of the links before writing (on the old numbers, the same graph) instead of during the write; the text of the error `DocumentError::InUse` names evaluations too.
- 312 tests pass; clippy only the known dead-code warnings.
- Not committed.

## Step 38: The AI layer: `ai/api/` in the form of REST, `ai/chat/`, roles of the AI

Raised by the user on 2026-10-17, before their review: the code of `ai/` that talks to the AI service is mixed with other code and has no single entry; it is to be an intermediate layer between the provider and the rest of the program, with its functions in the form of REST, collected in one place so that it can be audited.

**Decisions (user, 2026-10-17)**
- [x] `ai/api/`: the communication with the provider and its mapping onto Rust functions, which the rest of the code calls; the functions in the form of REST (resources and operations named like the methods of REST). **Not a server:** no port is opened (`architecture.md`, section 4).
- [x] `ai/chat/`: the functions of the chat, the conversations and their memory.
- [x] **The configuration of the connections is stored in the database, so its management belongs to `db/`** (`db/ai/`), not to `ai/`.
- [x] **Memory of a conversation** (shortening the history, summaries and other operations on text) may be extended, but **always controlled by parameters**: the quotes of a standard (and other text that must stay word for word) are never shortened.
- [x] **The AI acts in several roles**, and the interface takes them into account: analysis of a text and pointing to fragments; summarising parts of a text; consultation (the chat); formal analysis.
- [x] The list of todo.md is a heap, not a stack (user): this step is done now, before the review and step 31.

**Decisions (user, 2026-10-17, second round)**
- [x] **The rule: whatever happens in the database is in `db/`, in a subfolder of its own**, so that nothing is mixed. `db/ai/` gets subfolders as needed: `connection/` (the connections, the active one, the API key bound to the address: the management of `ai/connections.rs` with its rules of the stored data), `memory/` (the chats and their messages), `presets/` (the presets and the "AI help" of the fields; later the user may add and edit presets), and the others the roles and the journal need. The management of the connections and the memory of the chat are never mixed.
- [x] The description of a connection and its rules of security (`connection.rs`, `api_key.rs`) go to `ai/api/`: every request is made from it.
- [x] The instructions of the roles and the connections given to them are kept in the database (`db/ai/`); by default a role uses the active connection; a role may have a connection of its own.
- [x] **A journal of every question to the AI, for every role, now** (in the database, `db/ai/`).

**Concept (Claude, approved by the user 2026-10-17)**
- [x] A **role** is fixed by the program (the form of its answer, whether its quotes are checked, what the memory may do with its text); a **task** has its role and its own instructions. Tasks now: `chapter`, `contents` (text analysis), `chat` (consultation), `check` (no role). Later: phase 1 (text analysis), phase 2 (formal analysis), AI notes and drafts of outputs (summary).
- [x] The journal of the AI: one entry per question (time, task, role, connection, model, the body of the request, never a header, the answer, how it ended, tokens), never changed or removed (two triggers). How long entries are kept: later.
- [x] The instructions move from `settings` to `ai_instructions` without migration (they go back to the default ones).
- [x] **Approved by the user (2026-10-17):** the journal of the AI keeps the text of a chat after the chat is deleted, and the text of a reply the model refused (the chat drops it); everything stays traceable. How long entries are kept is decided later.
- [ ] Later: the window for giving a role its own connection (the core and the database have it); the journal of the AI shown in the window; operations of the memory of the chat (by parameters).

**As built (2026-10-17)**
- `ai/api/`: `roles.rs` (`Role`, `AnswerForm`, `MemoryRule`, `Model`, `ModelError`; `list`, `connection`, `set_connection`, `model`), `tasks.rs` (`Task`; `instructions`, `set_instructions`), `completions.rs` (`create`: pieces to a handler, "Stop" by a `watch`; `record`; `counting`, `outcome`, `single` for the long works; re-exports `Reply`, `ReplyEnd`, `StopReason`, `Speaker`, `Turn`, `AiError`), `checks.rs` (`target`, `create`, `record`), `connection.rs`, `api_key.rs`, `test_support.rs`; `provider/` (`client`, `request`, `response`, `sse`, `json_path`, `ai_error`) declared `pub(in crate::ai::api)`.
- `ai/chat/`: `chats.rs`, `conversation.rs` (the role *consultation*), `memory.rs` (`turns`: the whole history, as before).
- `db/ai/`: `connection/` (the management of `ai/connections.rs`: `create::connection`, `update::connection`, `update::active`, `update::api_key`, `read::{list, get, active, api_key}`, `delete::{connection, api_key}`), `memory/` (chats, messages), `roles/` (`ai_roles`, `ON DELETE SET NULL`), `instructions/` (`ai_instructions`), `journal/` (`ai_journal`, triggers), `presets/` (`read.rs` the built-in presets, `field_help.rs`).
- Window layer: `tauri_ui/model_run.rs` removed; the chat, the analysis of chapters, the table of contents and the check of a connection ask through `ai/api/` and write the journal (a failure to write it is told, not hidden).
- Changed in behaviour: every question goes into the journal of the AI; the chat and the analysis use the connection of their role (by default the active one, as before).
- 325 tests pass; clippy only the known dead-code warnings. Not tried in the window.
- Not committed.

## Step 39: The domain folders: CRUD of the objects in memory, `db/` for the lasting write

Raised by the user on 2026-10-17: `evaluation/` is disordered; its functions are to be split into folders and follow CRUD.

**Decisions (user, 2026-10-17)**
- [x] **`db/` is the communication with the database: the lasting write.** The CRUD of a domain (`evaluation/` and the others) **manages the objects in memory (RAM)**: when the user has a project open for editing and changes it, the objects in memory are handled formally (create, read, update, delete), and the lasting write is done by `db/`.

- [x] **Write-through** (variant A): every change of an object in memory is written to the database at once, in one transaction; the database gives the numbers of new objects. **The versions of an open project are kept in memory**, so that the user can undo changes.
- [x] An evaluation writes at once in any case: a result never exists without its entry of the journal.
- [x] Names: `standards/` (the documents of standards), `test_projects/` (the tool Create evaluation scheme), `evaluation/` (the tool Evaluation, today `product_evaluation/`); tiles and links in folders of their own.
- [x] `analysis/`: one file per task of the AI. It takes from the object the data, the instructions and the role of the model, and carries the task out through the internal structure of communication with the AI (`ai/api/`).
- [x] **The goal (user):** the functions and the responsibilities of the parts of the code are separated; the pipeline is very readable, and at every moment one can tell where the information the program works on is.

**Layout (agreed 2026-10-17)**

```text
standards/       model.rs, create/read/update/delete.rs, pdf_text.rs, quote.rs (no object in memory: a document is loaded and read)
test_projects/   model.rs, rules.rs, workspace.rs (the open projects and their versions for "Undo"),
                 projects/ schemes/ tiles/ links/ (CRUD of the open project in memory, written through by db/),
                 checks.rs, exchange/, analysis/ (one file per task of the AI)
evaluation/      the same pattern: the open evaluation in memory, results/, journal/
```

**As built (2026-10-17)**
- `standards/`: `model.rs` (types, rules of the fields), `create.rs` (`add`), `read.rs` (`list`, `get`, `by_sha256`, `page`, `pdf`, `search`), `update.rs`, `delete.rs`, `pdf_text.rs`, `quote.rs`. No object in memory: a document is loaded and read.
- `test_projects/`: `model.rs` (types and `ProjectData`), `rules.rs` (`SchemeError`, `StoreError`, `check_link`, the checks of the fields), `workspace.rs` (`Workspace`, `OpenProject`: `data`, `change` = one version per action of the user, `remember` = one version before a long work, `undo` with the numbers kept, `VERSIONS` = 50), `projects/`, `schemes/`, `tiles/`, `links/` (CRUD on the open project: the rules on memory, the write through `db/test_projects/`, then memory), `content.rs` (the content of an imported project), `checks.rs`, `exchange/` (`format.rs`, `export.rs`, `import.rs`), `analysis/` (`parts.rs`; `chapter.rs` and `contents.rs`, each with `question` from the open project and the instructions, and `answer` applied to it; asked over `ai/api/` in the role of the task).
- `evaluation/` (was `product_evaluation/`): `model.rs`, `workspace.rs` (`OpenEvaluation`: the frozen structure, the results, the proposals; no "Undo", every step is in the journal), `projects/` (`create.rs`: the import and the copy; `read.rs`; `delete.rs`), `results/` (`create.rs` propose, `read.rs`, `update.rs` accept/correct/write, `delete.rs` reject), `journal/read.rs`.
- `db/test_projects/update.rs`: `restore_content` writes a version back with the numbers it had; when a number was given to another project meanwhile, `undo` writes it with new numbers, reads the project back and forgets the older versions (memory and the database always agree).
- Window layer: `UiState` holds the `Workspace` of the test projects (its undo stacks removed); `user_data.rs` has `with_workspace` and `with_project`; `project_actions.rs` and `scheme_actions.rs` work on the open project; the analysis is the pipeline question → `ai/api/completions.rs` → journal of the AI → answer. The window of an evaluation (and its `Workspace` in `UiState`) comes with step 31.
- Tests: `test_support::single` in `test_projects/` and `evaluation/` run the CRUD as single calls and check after every call that memory and the database agree; `workspace_tests.rs` covers the versions and "Undo". 331 tests pass; clippy only the known dead-code warnings. Not tried in the window.
- Not committed.

## Step 15: Configurable AI connections (local models)

Raised by the user on 2026-10-16. The program is meant to work with a **local model** of the laboratory (Ollama, llama.cpp, vLLM, LM Studio or any other HTTP service); Anthropic is used only for the tests. So the connection to the AI is no longer written for one provider: the evaluator sets it up completely in the tab **AI** of Settings, and the program has **no code and no form for any particular provider**. Anthropic is configured in the same form as a local model, so that the tests exercise exactly the interface used in production.

**Decisions (approved 2026-10-16)**
- [x] Several named connections, kept in the encrypted database of the evaluator; one of them is active and used by the chat.
- [x] A connection holds: name, endpoint (the full URL), model name, optional API key, optional CA certificate (PEM), the request (headers and a JSON body template) and how the answer is read (stream or one JSON, paths to the text, the stop reason, an error, the model, the token counts).
- [x] No code for any provider. A new connection starts from one neutral template (the de facto standard chat API of local servers, `/v1/chat/completions`); everything can be changed.
- [x] Presets (decided by the user later the same day): a tab "Presets" in the form fills the fields with the settings of Local Gemma (Ollama), Anthropic, OpenAI and Google, as if they were typed; nothing is stored until the evaluator checks the fields and saves. A preset is only data (`ai/presets.rs`).
- [x] "AI help" at every field (decided by the user, instead of presets made by a chatbot): an instruction for a chatbot of the evaluator's choice that explains the field, its rules and an example, with the fields that matter for it as they are now and a place to describe the service. It is shown and copied; nothing comes back into the program on its own. No value of a header (except content-type and accept), no key and no certificate goes into it (`ai/field_help.rs`, command `ai_field_instruction`).
- [x] The form of one message is a template too (`{"role": "{{role}}", "content": "{{text}}"}` by default), so that services with another form of messages (the "parts" of Gemini) are described the same way.
- [x] Saving is always allowed; the check is a separate button ("Check connection"), because a local model may be switched off while it is set up. The check shows the request as it was sent (the key masked), the raw answer (shortened, the key masked) and what the paths found.
- [x] The old settings (the Anthropic key and the chosen Claude model) are removed without migration; the user enters the configuration again.
- [x] Done now, before the review reaches `ai/`.

**Rules of the connection (security)**
- `https://`: TLS 1.3 or newer, the certificate verified against the trusted certificates of the system, or **only** against the CA certificate given with the connection (an internal authority of the laboratory).
- `http://` only to the loopback (`127.0.0.0/8`, `::1`, written as an IP address): such traffic does not leave the computer. `http://` to any other address is refused when the connection is saved.
- No redirects, no proxy of the environment.
- The API key is used only where the placeholder `{{api_key}}` stands in a **header**; it is refused in the URL and in the body, so that it never lands in a log of a server. It is never shown back, and it is masked in the report of a check.
- The key is bound to the address: when the scheme, host or port of a connection with a stored key changes, the key is removed and has to be entered again.
- The placeholders of the body (`"{{model}}"`, `"{{messages}}"`, `"{{system}}"`) stand for whole JSON values and are replaced as values, never by joining text: a message can never break the JSON or add fields to the request.

**Tasks**
- [x] Core (`ai/`): the connection and its rules, storage, the request from the template, reading the answer by paths, the client; the old Anthropic code and the list of Claude models removed.
- [x] Tests against a fake service: a streamed and a whole answer in two different formats, errors, TLS 1.2 and a foreign certificate refused, a CA certificate of the connection, plain HTTP on 127.0.0.1, the key only in its header, the placeholders as values, the key removed with a changed address, "Stop".
- [x] The tab AI of Settings (list, form, key, check); the AI section of Settings and the settings fold of the chat removed; the chat names the active connection.
- [x] `docs/architecture.md`: the rule "plain HTTP only on the loopback" (section 4).
- [ ] The user's test with a local model and with Anthropic.

**As built**
- `ai/connection.rs` (the fields as entered and their rules, the template of a new connection), `ai/json_path.rs` (paths such as `choices[0].delta.content`, `content[*].text`), `ai/request.rs` (the request from the template; the turns joined so that the speakers alternate), `ai/response.rs` (reading a stream or one JSON with the paths), `ai/client.rs` (the HTTP client per connection, the reply, the check with its report), `ai/connections.rs` (storage, the active connection, the key bound to its address), `ai/ai_error.rs` (kinds by HTTP status, the explanation from the error path or the beginning of the answer).
- Removed: `ai/anthropic.rs`, `ai/reply.rs`, `ai/model.rs`, `ai/ai_settings.rs`. The old secret `anthropic.api_key` and setting `ai.model` are deleted when a database is opened (`db/user_db.rs`, to be removed before the release, step 10).
- Database of the evaluator: new table `ai_connections`; keys in `secrets` as `ai.connection.<id>.api_key`.
- Window: page `ui/src/pages/AiConfig.svelte`, commands `ai_config`, `save_ai_connection`, `delete_ai_connection`, `activate_ai_connection`, `store_ai_key`, `delete_ai_key`, `check_ai_connection` (`tauri_ui/ai_config_actions.rs`, `serializer/ai_config_view.rs`). An error of a field names the field, and the page marks it.
- Timeouts: 15 s to connect; a stream may be silent 300 s (a local model may think long), a whole answer may take 900 s; a check 120 s. Largest answer 8 MiB.
- `serde_json` keeps the fields of a template in the order written (`preserve_order`).
- Changed 2026-10-16 (the user): the AI connections are settings, not a tool. Settings have tabs **User** (password), **AI** (the connections, `AiConfig.svelte`) and **Program** (version, role, folder of the data, protection); the sidebar holds Chat, Scan and permissions, Settings.
- New look of the window (the user left the design to Claude): one set of colour and size tokens in `ui/src/style.css` with a dark variant that follows the system, a dark sidebar with simple SVG icons drawn in `Shell.svelte` (no icon library, no images), cards for the sections, primary and danger buttons, message bubbles in the chat, the login and setup as a card. No web font and no library: the CSP allows only the files of the program.
- Message template: field `message_template` (placeholders `{{role}}`, `{{text}}`, whole JSON texts); a table made before it gets the column when the database is opened (`db/user_db.rs`, to be removed before the release, step 10).
- Presets (`ai/presets.rs`), values as of 2026-10-16: Local Gemma (`http://127.0.0.1:11434/v1/chat/completions`, `gemma4`), Anthropic (Messages API, `claude-opus-5-5`, with the server-side fallback), Anthropic Haiku (the same, `claude-haiku-5-5`, without the fallback, which Haiku does not have; added during the review for tests), OpenAI (Chat Completions, `gpt-6-astra`, token counts with `stream_options`), Google (Gemini API `streamGenerateContent?alt=sse`, `gemini-4-argon`, the model in the URL, `systemInstruction` and "parts"). Each is tested against the fake service in the form of its stream.
- The thinking of a model is not shown: only what the text path finds is kept. With the server-side fallback of Anthropic the header and the body field are simply part of the template.

**Done when:** a local model and Anthropic both work in the chat through connections set up only on the tab AI of Settings; all tests pass.

---

## Step 29: Evaluation: the evaluation project, the results and the journal

Concept approved by the user on 2026-10-17: the import from both sources (a ready test project of the installation, and the file of an exported project); only the accepted part is copied; a seventh section of an output, *Vulnerabilities and notes*; the module `product_evaluation/`. The states of the inputs come with step 32, the fragments with step 33.

- [x] The import: a test project → an evaluation project (a frozen copy of its accepted schemes, tiles, sources and links; the proposals left out and counted); the rules of `scheme.rs` checked again; the fields of the evaluation (name, product, version, manufacturer, evaluator) and its origin as text.
- [x] The four results of every requirement (findings; analysis; assessment and verdict proposition; vulnerabilities and notes) as text; the proposals of the AI kept apart, marked AI, made results only by the evaluator (accept, correct, reject; or written alone); the origin of every result.
- [x] The journal, append-only, kept so by the database (two triggers); every step written in the same transaction as its change.
- [x] The seventh section of an output, *Vulnerabilities and notes* (`evaluation/scheme.rs`).
- [ ] The user's review and approval: not done; the step was removed first (below).

**As built (2026-10-17)**
- `src/product_evaluation/`: `model.rs` (types, `ResultField`, `ResultOrigin`, `JournalAction`, `EvaluationError`, checks), `import.rs` (`from_project`: only a project marked ready; `from_exchange`: the documents found or loaded by the caller, the quotes checked again), `projects.rs` (`create` in one transaction with the first entry of the journal, `list`, `get`, `data`, `tile`, `delete` with `secure_delete`), `results.rs` (`propose`, `accept`, `correct`, `reject`, `write`, `of`, `proposals`), `journal.rs` (`append`, `of`), `test_support.rs`; 21 new tests (310 in all).
- Database: tables `evaluations`, `evaluation_documents`, `evaluation_schemes`, `evaluation_tiles`, `evaluation_tile_sources`, `evaluation_links`, `evaluation_results`, `evaluation_proposals`, `evaluation_journal` with the triggers `evaluation_journal_unchanged` and `evaluation_journal_kept` (`db/user_db.rs`). A document of a standard used by an evaluation cannot be deleted (`evaluation/documents.rs`).
- Not used by the window yet (step 31): the compiler warns of unused code until then.
- Committed in `e7711ee` (2026-10-17).

> **Removed in step 41.1 (2026-10-18)** together with the test projects it copied (commit `f91cba8`); its decisions (the four results of a requirement, the proposals of the AI kept apart until the evaluator accepts them, the journal that is only added to) are carried onto the OSCAL assessment results in step 41.5 (`todo.md`, step 23).

---

## Branches until 2026-10-20

Until 2026-10-20 the stages of the work were kept in branches named as versions. The user removed them on 2026-10-20 (the project is at an early stage, so no versions are kept yet; `CLAUDE.md`, "Branches"): `main` holds the tested state and `test_branch` the work. Every commit of the removed branches is in the history of `main`; their last commits:

| Branch | Last commit | What it held |
|---|---|---|
| `v0.2`, `V0.2` | `6cc4091` | version 0.2, the program in C (`v0.2_functionality.md`) |
| `v1.1` | `8ec09cf` | Bluetooth in the program in C, untested (before `6cc4091`) |
| `v0.3` | `acedc8b` | the Rust program with the web interface (steps 0 to 7) |
| `v0.4` | `fbc3c47` | the desktop window (step 14) and the evaluation schemes, before step 40 |
| `v0.5` | `f5e0e5c` | step 40 (the layers) and step 41.1 to 41.3 (NIST OSCAL), the CEM catalog |

Older entries of this file name these branches as they were then.
