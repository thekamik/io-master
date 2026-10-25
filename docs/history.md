# IO Master: the steps already built

The steps of `todo.md` that are built, moved here on 2026-10-17 (step 40) and on 2026-10-24 (steps 40 to 52, with the steps dropped in step 48 and those replaced by steps 51 and 52) so that the plan stays short. Each keeps its decisions, its tasks and its state as they were when it was built; **the paths of the code they name are those of their time** (before step 40 the code had other folders: `evaluation/`, `test_projects/`, `standards/`, `installation/` at the top, `ai/chat/`, `db/ai/memory/`, ...; today's layout is in `code_structure.md`). The number of a step is its name, as in `todo.md`.

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

**Removed 2026-10-24 (step 48 of `todo.md`):** the program no longer works with devices. The last state of the scan (`hardware_tests/`, `views/devices/`, `db/scan_settings/`, the page `ui/src/views/devices/`) is in commit `bdde117`.

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

## Step 40: The cleanup of the architecture (2026-10-17)

Built on the analysis of the code of 2026-10-17 and the guidelines of the user. **The structure and the rules that came out of it are in `code_structure.md`**; the guidelines are in `CLAUDE.md`, section "The architecture".

**Decisions of the user**
- [x] The folders `model/`, `db/`, `ai/`, `hardware_tests/`, `security/`, `business/`, `tauri_ui/`; the business folder named `business/`.
- [x] The operations of a resource are functions named after REST verbs (then `projects::create`; today `catalogs::create`), as in `ai/api/`.
- [x] The questions to the AI and the reading of its answers are in `ai/tasks/`.
- [x] The README keeps both aims, written anew for the project as it is now.
- [x] No approval of the review moves with the code: every file is reviewed from the start.
- [x] The modified code went to the branch `v0.5` (since 2026-10-20: `main` and `test_branch`).

**Tasks** (one commit each, all tests passing after every one)
- [x] 40.2 `rustfmt.toml` (120 characters) and one run of `cargo fmt`.
- [x] 40.3 The module tree out of `main.rs`: every folder declares its modules in the file of its name.
- [x] 40.4 The AI connections saved in one transaction; a key given out only for the address it is bound to.
- [x] 40.5 `model/`: the types and their rules; one `check_text`, one `now`; the normal form (`normalized`) the only place where text is trimmed.
- [x] 40.6 `db/`: migrations by version, unique names in the database, busy timeout; columns by name, named parameters, rows as types, `DbError::Corrupt` instead of guessed values; one reading of tiles; the journal of an evaluation in its own folder; one style of functions; `db/ai/chats/`.
- [x] 40.7 `security/` (random numbers, the TPM) and `hardware_tests/` (the scan).
- [x] 40.8 `ai/`: `ai/tasks/` (questions and answers), `ai/api/connections.rs`, `ai/presets/`; no dependency of `ai/` on the business.
- [x] 40.9 `business/`: the session (the database opened once at the login), the long works with `Progress` and `Cancel`, every question to the AI journaled by `ai/api/` itself, the resources with REST verbs, `db_<area>::` instead of aliases, errors with enums of fields and objects.
- [x] 40.10 `tauri_ui/`: `commands.rs` (the whole surface) and `commands/` per resource; all error conversions in `error.rs`; the window no longer reaches `db/`; a test of the four lists of commands.
- [x] 40.11 A test of the direction of the dependencies (`src/architecture_tests.rs`).
- [x] 40.12 The page: `pages/` and `components/`, `lib/api/` per resource, the style in parts (`styles/`).
- [x] 40.1, 40.13 The documentation: README, `architecture.md`, `code_structure.md`, this plan and `history.md`, `handover.md`, `CLAUDE.md`, `review.md` reset.
- [x] The unused dependencies of the web interface removed (axum, futures-util, tower, http-body-util).

**Done when:** the user has reviewed the new layout.

---

## Step 41: The evaluation plan on the model of NIST OSCAL (2026-10-18)

**Decided by the user (2026-10-18):**
- [x] The "test project" (the map of a standard, the tool "Create evaluation scheme") is renamed **evaluation plan**, in the code, the database, the window and the documents.
- [x] The program adopts **NIST OSCAL** (Open Security Controls Assessment Language), the machine-readable format of NIST for requirements and their assessment, so that the requirements of a standard and the plan of their evaluation follow a published standard instead of a model of our own, which may have missed normative matters.

**Second round (user, 2026-10-18):** two functions, *evaluation plan* (make or load the OSCAL scheme of a standard, show it as a graph, hand it over) and *evaluation* (evidence and checks with the AI, the report), plus the integration of the tools of the functional tests; the data of standards stored in the NIST standard, our own additions (AI instructions, prompts, the work of the evaluation) kept apart as the **evaluation support** and only linked to it; catalog, mapping, profile, assessment plan and assessment results now, SSP and component definition with step 30; the earlier data deleted, no migration; tests on NIST files, and schemes of our own for Common Criteria and ETSI EN 303 645. **The concept is in `docs/oscal.md`**, approved by the user on 2026-10-18 (with the answers of its section 8).

**Parts** (one commit each):
- [x] 41.1 The rename to *evaluation plan*; the old model of the test projects and the evaluations removed with its tables (no migration).
- [x] 41.2 The foundation: the JSON schemas of OSCAL 1.2.3 in the repository (`oscal/`), the types of `model/oscal/` with the check against the schemas, `db/oscal/`; import, export and deletion of documents in the window (part 1: types and schemas; part 2: storage and window).
- [x] 41.3 Own catalogs made in the program: part 1 the catalogs and the evaluation support (quotes, proposals); part 2 the AI proposes the groups (from a table of contents) and the controls (from the pages of a group), the evaluator accepts, corrects or rejects; part 3 the window: the editor of a catalog (tree, object, its quotes in the viewer, its fields as JSON of OSCAL; the AI with progress and Stop; the proposals; the instructions of the two tasks). Tested in the window with a fake model.
- [ ] 41.4 Profile and assessment plan (the evaluation plan), shown as a graph; the CEM and the Protection Profiles as documents of OSCAL that a plan can take.
- [ ] 41.5 Assessment results (the evaluation), then steps 31 to 36 on them.

---

## Step 42: The views (2026-10-21)

The user found that `business/` and the commands of `tauri_ui/` did the same thing in two layers: one action of the page went through three Rust functions of the same name and two closures, and the data flow could not be followed. The layout and its rules: `code_structure.md`, section 2.9.

**Decisions of the user**
- [x] Like the views of Django: every function that joins the program with the page belongs to an element of the window; `business/` and the commands of `tauri_ui/` are joined into `src/views/<view>/<element>/`.
- [x] The page (`ui/`) and the Rust program are separate projects; their files are never mixed. The page has the same names (`ui/src/views/<view>/`).
- [x] Only what matters goes to Rust (the AI, processing, storing); presentation stays in the page; only final results are sent.
- [x] When an element changes data another element shows, it tells the view, and the other element reads the data again with its own command.
- [x] `mod.rs` in the folder of a view and of an element; commands named `<element>_<action>`.
- [x] The pilot is Scan.
- [x] New code follows the rules for Rust (CLAUDE.md, "The rules for Rust").

**Tasks**
- [x] Scan: `views/scan/` (`ports_table/`, `scanner_settings/`), the settings of the scanner as plain data in `hardware_tests/devices/scan_settings.rs`, stored by `db/scan_settings/`; `business/hardware/`, `tauri_ui/commands/scan.rs` and `serializer/scan_view.rs` removed; the page in `ui/src/views/scan/`.
- [x] `ensure!` (`checks.rs`); the tests of the commands through the IPC of Tauri (`views/test_support.rs`); the test of the command lists for the views (`command_lists_tests.rs`); `views/` in the test of the architecture.
- [x] Start (`setup_form`, `login_form`) and Shell (`program`, `user_menu`); the installation, the account and the session moved unchanged to `installation/`. The setup, the login and the logout give no data back: the page reads the state of the program again (`program_get`).
- [x] Chat (`connection_line`, `chat_list`, `conversation`); one turn of a chat (`business/chats/`) moved into the conversation (`turn.rs`, `stored.rs`); the line names the connection of the role *consultation*, as the chat uses it.
- [x] Settings (`password_form`, `connection_list`, `connection_form`, `api_key`, `connection_check`); the tab Program shows the state of the program the frame has read. The key has an element of its own: the form never carries it, and the key is read again after a save (a changed address removes it).
- [x] Evaluation plan (`oscal_documents`, `new_catalog`, `standard_documents`, `document_search`, `pdf_viewer`): the reading of PDFs in the new folder `pdf/` (without a closure: the loading reads part by part), the normal form of a quote in `model/standards/quote.rs`, the error of the stored documents of OSCAL in `db/oscal/error.rs`, the small standard of the tests in `test_support/standards.rs`. Every element reads the data it shows (the viewer and the new catalog their documents, a hit the title of its document). The command `document_page`, which the page never called, removed.
- [x] The editor of a catalog, the last view (`catalog_tree`, `object_form`, `new_object`, `object_quotes`, `proposals`, `ai_builder`, `ai_instructions`): the changes of a catalog in memory in `model/oscal/catalog/` (`edit.rs`, `tree.rs`), the new UUIDs and times in `db/oscal/ids.rs`, a new catalog in `views/plans/new_catalog/made.rs`, the parts of the pages for the AI in `ai_builder/parts.rs`. The heavy work: the AI is an `async fn` command that awaits `ai/api/`, the PDFs are read with `spawn_blocking` (`code_structure.md`, section 2.9). The page in `ui/src/views/catalog_editor/`.
- [x] `business/`, `tauri_ui/commands/`, `tauri_ui/session.rs` and `tauri_ui/blocking.rs` removed; `tauri_ui/serializer/` keeps only the error of a command (`error_view.rs`); `ui/src/components/` removed, `ui/src/pages/` keeps the two pages that call no command yet (Evaluation, Documentation); the exceptions of `architecture_tests.rs` for `tauri_ui/` are only the errors of the folders below.

**Done when** every command of the window lives in the folder of its element and `business/` is gone.

---

## Step 43: One vocabulary, views and components, `tauri_ui/` renamed (2026-10-23)

**State: built 2026-10-23, in the phases 43.1 to 43.9 (below).**

The user found, after step 42: `tauri_ui/` is left although the architecture was agreed in the style of a web server; the names of the code and of the window differ from the words the user prompts in; and the views are mixed with the elements several of them use (the viewer of a standard lives in the view Evaluation plan and is used by the editor of a catalog as well).

**Decisions of the user**
- [x] One vocabulary for Rust, the page, the navigation and the documents (`CLAUDE.md`, "Terms"): `start`, `shell`, `evaluation_plan`, `evaluation_lab`, `devices`, `settings`, `chat`.
- [x] A view is a whole page (the high-level display); a component is a small element used more than once (several times in one view or in several views), kept in `components/` and imported by the views (`CLAUDE.md`, "Terms"). Example to come: the *requirement*, with its functions of the AI.
- [x] `tauri_ui/` is renamed **`app/`** (user, 2026-10-23): `state.rs`, `error.rs`; the list of the commands moved out of `main.rs` into `src/routes.rs` (as `urls.py` of Django), above `views/`. It holds the infrastructure every view uses, as the session and the error handling of a web framework; not `server/`, a name kept for the network server of the reset service (steps 4 and 11).
- [x] The vocabulary, the tasks of the folders and the flow of data in the views are in `architecture.md`, sections 10 to 12 (user, 2026-10-23).
- [x] The Manufacturer documentation (step 30) is a part of `evaluation_lab` (user, 2026-10-23): during an evaluation the laboratory delivers the evidence there (quotes of the Security Target, ...). The placeholder page goes into the view.
- [ ] **The tiles of the evidence (open, not part of this step):** the user has not thought them through yet (2026-10-23). The present idea (`architecture.md`, section 10.2): requirement ← input ← proof, the AI in the input checking whether a proof meets the criteria to be that input of the requirement. Settled in the concept of step 41.5.
- [x] **How the window loads** (user, 2026-10-23): the page is loaded once; a view asks for no data of its own; every element and component reads its own data with its own command and refreshes only itself (`architecture.md`, section 12). This is the way of step 42, kept.

**The map** (old → new; the commands keep their names `<element>_<action>`, only their paths change)

| Old | New | Why |
|---|---|---|
| `src/tauri_ui/` | `src/app/`; the list of the commands of `main.rs` to `src/routes.rs` | infrastructure, not an interface |
| `src/views/plans/` | `src/views/evaluation_plan/` | vocabulary |
| `src/views/catalog_editor/` | `src/views/evaluation_plan/catalog_editor/` | opened from the evaluation plan (`Plans.svelte`), a part of it |
| `src/views/plans/pdf_viewer/` | `src/components/pdf_viewer/` | used by the evaluation plan and the editor of a catalog |
| `src/views/catalog_editor/part_rows.rs` | `src/components/part_rows/` | used twice: the object chosen and the proposals |
| `src/views/scan/` | `src/views/devices/` | vocabulary; its elements keep their names |
| `ui/src/views/plans/`, `Plans.svelte` | `ui/src/views/evaluation_plan/`, `EvaluationPlan.svelte` | vocabulary |
| `ui/src/views/catalog_editor/` | `ui/src/views/evaluation_plan/catalog_editor/` | as in Rust |
| `ui/src/views/plans/PdfViewer.svelte`, `lib/pdf.ts` | `ui/src/components/PdfViewer.svelte` (with its pdf.js code) | component |
| `ui/src/views/catalog_editor/PartRows.svelte` | `ui/src/components/PartRows.svelte` | component |
| `ui/src/views/scan/`, `Scan.svelte` | `ui/src/views/devices/`, `Devices.svelte` | vocabulary |
| `ui/src/pages/Evaluation.svelte`, `Documentation.svelte` | `ui/src/views/evaluation_lab/EvaluationLab.svelte` (one placeholder) | vocabulary; the documentation of the manufacturer is a part of the evaluation lab; `pages/` removed |
| `ui/src/lib/api/<old view>/` | `ui/src/lib/api/<new view>/`, `ui/src/lib/api/components/` | as the views |
| navigation: "Scan and permissions", "Evaluation", "Manufacturer documentation" | "Devices", "Evaluation lab" | vocabulary |

`chat/`, `settings/`, `start/` and `shell/` keep their names. What two elements of the editor of a catalog share without being shown (`checked_quote.rs`, `oscal_object.rs`) stays in its folder: it is not a component.

**The rules checked by `src/architecture_tests.rs`**
- `components/` uses the folders below (`model/`, `db/`, `ai/`, ...) like a view; it never uses `views/`.
- A view uses `components/`; it never uses another view (a part of a view inside its folder, as the editor of a catalog, is not another view).

**Phases** (user, 2026-10-23: short series, so that a session never runs out of its context in the middle of a change and the work always returns to a stable state). Each phase is one commit, pushed: it builds, every test passes, clippy and `npm run check` are as before, and the documentation of what it changed is in the same commit. A session does one phase, or two small ones; the next session starts at the first phase not ticked.
- [x] 43.1 `src/tauri_ui/` renamed `src/app/` (the folder only; its types renamed in 43.2); every `use`, the test of the architecture, the comments, `code_structure.md`.
- [x] 43.2 The list of the commands moved out of `main.rs` into **`src/routes.rs`** (as `urls.py`; user, 2026-10-23: not `app/routes.rs`, which would name `views/` from below them, a cycle); the window and the tests of the commands use the one list (the copy in `views/test_support.rs` removed); `command_lists_tests.rs` reads it there. The types `UiState`, `UiError` renamed `AppState`, `AppError` (also `AppError` and `isAppError` of the page).
- [x] 43.3 `scan` renamed `devices`: Rust (`views/devices/`), the page (`ui/src/views/devices/`, `lib/api/devices/`), the navigation "Devices".
- [x] 43.4 `plans` renamed `evaluation_plan`: Rust, the page, `lib/api/`, the navigation.
- [x] 43.5 `catalog_editor` moved into `evaluation_plan/catalog_editor/`: Rust, the page, `lib/api/`.
- [x] 43.6 `components/` with its first component `pdf_viewer` (Rust and page); the rules of the components in the test of the architecture (a component never uses a view, a view never another view).
- [x] 43.7 `part_rows` moved into `components/` (Rust and page).
- [x] 43.8 `evaluation_lab`: one placeholder view instead of the pages Evaluation and Manufacturer documentation; `ui/src/pages/` removed; the navigation. A test that the ids of the pages (`views/shell/program/view.rs`) and the page `Shell.svelte` agree (an icon, a subtitle, a view for each), as the commands are checked (found in 43.4: the subtitle of Devices was lost in 43.3 and no check saw it).
- [x] 43.9 The end: the notes "was ..." removed from `architecture.md` (sections 10 to 12) and `CLAUDE.md` ("Terms"), `handover.md`, the state of this step.

**Done when** every name of the code is the one of the vocabulary, every element used more than once is in `components/`, and the test of the architecture checks it.

---

## Step 44: The tests in the file they test (2026-10-23)

**State: built 2026-10-23, in the phases 44.1 to 44.6 (below).**

The user (2026-10-23): the unit tests move from their own files (`<file>_tests.rs`, 85 files, about 6000 lines) into the file whose functions they test; fewer files, and a function and its tests read together. It replaces the rule of `CLAUDE.md` ("Way of working") and of `code_structure.md` (section 3, Tests).

**The rule (proposed)**
- The tests of a file are its **last item**: `#[cfg(test)] mod tests { use super::*; ... }`, after all the code of the program, so that the file is read from the top as the program and ends with its tests. The head of the module says what the tests cover.
- What stays in files of its own: the help of the tests shared by several files (`test_support.rs`), and the tests of the whole program, which test no one file (`architecture_tests.rs`, `command_lists_tests.rs`).
- `// ok kk` is unchanged: it marks a function of the program; the tests below it may still be changed.

**What the checks need**
- `architecture_tests.rs` reads a file only up to its test module, and checks that the module is the last item (after the line `mod tests {` only indented lines, empty lines and the closing `}`), so that no code of the program can hide below it. When step 44 is done, a file named `*_tests.rs` is no longer skipped by its name (until then the folders not moved yet keep theirs).
- The other checks (clippy, rustfmt, the test of the command lists) see the same code as before.

**Phases** (one commit each, pushed; every test passing after it; the move is mechanical: the content of `<file>_tests.rs` is put into `mod tests { }` at the end of `<file>.rs`, rustfmt indents it, the `#[path]` lines are removed)
- [x] 44.1 The rule in `CLAUDE.md` and `code_structure.md`; `architecture_tests.rs` as above; the small folders: `checks`, `model/`, `pdf/`, `security/`, `app/`, `components/`, `installation/`.
- [x] 44.2 `db/` (13 files). The tests of a whole area of CRUD were split by the function each checks (user, 2026-10-23): every file of an area ends with the tests of its own functions; the shared data of the tests of `db/` is in `src/test_support/db.rs`.
- [x] 44.3 `ai/` (15 files; each tested one file).
- [x] 44.4 `hardware_tests/` (3 files).
- [x] 44.5 `views/`: `start`, `shell`, `chat`, `settings`, `devices`.
- [x] 44.6 `views/evaluation_plan/` with its editor of a catalog; `handover.md`, the state of this step. The acceptance of a proposal, tested so far inside the tests of the AI of the editor, has its own tests in `proposals/accept.rs`; the helpers of the tests of the AI (`ready`, `collector`) are in `catalog_editor/test_support.rs`; the test of the architecture no longer skips a file by its name, and refuses a new `*_tests.rs`.

**Left open (2026-10-23):** `db/user/delete.rs::remove_setting` has no test yet (the split of 44.2 only moved the assertions that existed).

**Done when** no `<file>_tests.rs` is left besides the tests of the whole program, and the test of the architecture checks that every test module is the last item of its file.

---

## Step 45: The tree of the modules in `main.rs` (2026-10-23)

**State: built 2026-10-23, in the phases 45.1 to 45.4 (below).**

27 files of the layers hold no code, only declarations of modules (`db.rs`, `db/ai.rs`, `db/ai/chats.rs`, `security.rs`, `security/tpm.rs`, ...). The user: they are moved into `main.rs`, but not as a heap: in the order of the layers (`model/` up to `views/`), each group with a comment that says which part of the project it is and what it does, and each module with its line; `main.rs` becomes the map of the program. Rust allows it: `mod security { pub mod random; pub mod tpm { pub mod chip; } }` in `main.rs` finds `src/security/random.rs` and `src/security/tpm/chip.rs`.

**Decisions of the user**
- [x] The declarations of the layers go into `main.rs`, grouped and commented; the files that only declared them are removed.
- [x] The rule "`main.rs` lists only the top folders, so a new file never changes it" (`code_structure.md`, section 3) is replaced: every file of the program is named in `main.rs`, so the map is always complete.
- [x] The comments of the removed files: a line that names a folder goes to its group in `main.rs`; a longer description of the architecture goes to `docs/` (Markdown, with schemes), never to the head of another file (`CLAUDE.md`, "Where the comments are").
- [x] The `mod.rs` of the views and of the components (35 files) are not part of this step: step 46.

**Phases** (one commit each, pushed, every test passing; `code_structure.md`, sections 3 and 4, with each):
- [x] 45.1 The map in `main.rs`: every folder in the order of the layers from the bottom up, each group commented; `model/` written as its tree, `model.rs` removed (its list of files named a file that no longer exists, `analysis.rs`: the descriptions in files without code go stale).
- [x] 45.2 `security/` and `db/` (15 files of declarations removed; `db/scan_settings.rs` stays, it holds the names of its settings). The comment of each area of `db/` says in which database its data lives; its `schema.rs` says how it is protected.
- [x] 45.3 `ai/` (4 files of declarations removed; `ai/presets.rs` stays, it holds the presets). The provider stays reachable only from `ai/api/` (`pub(in crate::ai::api)`), seen to refuse a use from a view; the list of `ai/tasks.rs`, which named files that no longer exist, is gone, and `CLAUDE.md` names the real tasks.
- [x] 45.4 `hardware_tests/`, `pdf/`, `installation/`, `app/` (6 files of declarations removed; the two files that are not compiled, `hardware_tests/permissions/permission.rs` and `installation/uninstall.rs`, are named in the comments of their groups); the exception of `hardware_tests.rs` in `architecture_tests.rs` (`the_tests_of_a_file_are_never_in_a_file_of_their_own`) removed with the file; the state of this step.

---

## Step 46: The entry of a view (2026-10-23)

**State: built 2026-10-23, in the phases 46.1 to 46.7 (below).**

When the folder of a view is opened, one must see at once where the view begins: the file the page calls to open it, and the functions the page calls to refresh its elements (user, 2026-10-23). The functions are right already; the fault is readability.

**Decisions of the user**
- [x] **One entry file per view, named after the view** (`views/chat/chat.rs`): all the commands of the view; first those the page calls when the view opens, then the refreshes and the actions of its elements. Every function carries the comment of `CLAUDE.md` ("Where the comments are"): what it does, where the page calls it (which component), which functions it calls. The `commands.rs` of the elements are gone.
- [x] The folders of the elements stay, with their data and logic only (`view.rs`, `turn.rs`, `stored.rs`), around the entry file that uses them.
- [x] The 35 `mod.rs` of the views and the components are gone: their declarations go into the map of `main.rs`, their descriptions of the flow of data into `docs/architecture.md`, section 12. The editor of a catalog, a screen of its own, has its entry `catalog_editor.rs`; a component has the file of its name (`pdf_viewer.rs`).
- [x] The page follows: one file of commands per view (`ui/src/lib/api/chat.ts`) instead of one per element.
- [x] No new command "open the view": the entry file shows at its top the functions the page calls when the view opens.
- [x] The tests of the command lists check the new layout: a command is in the entry file of its view (or component), its name begins with the name of an element of that view.
- The entry file has the name of its folder, so its module has the name of its parent (`views::devices::devices`); clippy's `module_inception` is allowed at those declarations in `main.rs`, with the reason.

**Phases** (one commit each, pushed, every test passing; the documents with each)
- [x] 46.1 `devices`, the pilot; `views.rs` written in the map of `main.rs`; the test of the command lists for entry files.
- [x] 46.2 `start` and `shell`. An element with no data of its own (the login form) has no folder in the program; the test of the names takes the elements also from the components of the page (`LoginForm.svelte` is `login_form`).
- [x] 46.3 `chat`.
- [x] 46.4 `settings` (its two helpers, `api_key_view` and `no_such_connection`, at the end of the entry, commented as the commands).
- [x] 46.5 `evaluation_plan`. What the commands of its elements did beside the commands went into the elements: the dialogs of the system into `oscal_documents/dialog.rs` and `standard_documents/dialog.rs`, the reading of a file and the safe name of a file into `oscal_documents/file.rs`, the limit of a search into `document_search/search.rs`.
- [x] 46.6 the editor of a catalog (its helper `instructions_task` at the end of the entry; the description of the pipeline of the AI, in `ai_builder/mod.rs` so far, is in `architecture.md`, section 12).
- [x] 46.7 the components (`pdf_viewer` with its entry `pdf_viewer.rs`; `part_rows`, without commands, only its `view.rs`); `components.rs` in the map; the test of the command lists no longer reads a `commands.rs`.

---

## Step 47: The view `users`: the account, the session and the encryption in one place (2026-10-23)

**State: built 2026-10-23, in the phases 47.1 to 47.3 (below).**

The user (2026-10-23): the login is joined with the encryption of the database (the password, Argon2id, the unlocking key, the key of the database, the open database), so every function that touches it must be in one place, where the management of the keys and the encryption and decryption can be followed easily. The view `start` is renamed `users`, and its entry holds the setup (the installation and its user), the login, the change of the password, the logout and the closing of the program, and later every other function of the account. To keep the window and the code readable, the settings are split: **the settings of the user** (the account and the encryption: a page of their own, "User", in the navigation) and **the settings of the program** (the AI connections, what the installation is: the view `settings`).

**Decisions of the user**
- [x] `start` renamed `users`; `views/users/users.rs` is the entry of everything that touches the key of the database.
- [x] The settings of the user are a page of their own in the navigation ("User"); the view `settings` keeps the tabs AI and Program.
- [x] A new function: log out and close the program (`user_menu_close`), beside "Log out" in the user menu.
- The commands keep their names `<element>_<action>`; the user menu is shown in the frame of the window (`Shell.svelte`), which places it as it places the views.

**Phases** (one commit each, pushed, every test passing; the documents with each)
- [x] 47.1 `users`: the setup and the login (from `start`), the logout (from `shell`), the change of the password (from `settings`), each commented with what happens to the key; the page `ui/src/views/users/` (the forms, the user menu, the page "User"); `settings` without the tab User; the vocabulary.
- [x] 47.2 `user_menu_close`: the session ended (the key wiped, the database closed) and the program closed; its button in the user menu. **The cross of the window ends the session the same way** (user, 2026-10-23): "Log out", "Log out and close" and the cross go through one function, not through the end of the process alone. With it, the memory of SQLCipher (user, 2026-10-23: only the minimum of decrypted data in RAM; SQLCipher already decrypts only the pages a query reads, so the concept stands): `cipher_memory_security` on (the memory it frees wiped, its keys locked in RAM), the temporary files of SQLite in memory only and a small, explicit cache, each checked by a test; the core dumps of the process off; the requirements of the laboratory (encrypted swap, no hibernation) in `architecture.md`, section 9.
- [x] 47.3 `architecture.md`, section 12: the way of the key in every operation of `users`.

---

## Step 48: The evaluation with AI as the only aim (2026-10-24)

**State: decided 2026-10-24; 48.1 and 48.3 built; 48.2 (the documents of the framework) partly done with them.**

The user (2026-10-24): the project tried to catch too many magpies by the tail. **IO Master is a framework for the evaluation with the help of AI**: it works with the documentation of the client and the inputs of the evaluator, and its output is the report. The name stays: IO is the input and the output, and the flow of the data between them is to be done masterfully.

**Decisions of the user**
- [x] **No work with devices.** The scan of the serial ports, the view Devices, `hardware_tests/` and `db/scan_settings/` are removed; step 6 (port permissions), UART and Bluetooth are dropped. The results of tests on a device are inputs of the evaluator.
- [x] **P2 removed** ("no command reaches a device without an action of the evaluator"); its number stays so that P3 to P5 keep theirs. That the AI only proposes is P1.
- [x] **The name IO Master stays.**
- [x] **No role master and no password reset through a master** (steps 4 and 11 dropped): it is architecturally overcomplicated. The evaluator exports backups instead (section 5.5 of `architecture.md`), encrypted with **OpenPGP** keys, a standard of the laboratories.
- [x] **No TPM factor and no passkeys** (steps 8 and 9 dropped): OpenPGP makes them unnecessary.
- [x] **No git in the program** (`architecture.md`, section 6): the laboratory manages its documents itself; not every laboratory uses git, and the configurations would hold the work up.
- [x] **The export to LaTeX stays** (step 28): the results go straight into the report template of the laboratory.
- [x] ~~The protection is built on GnuPG~~ (user, 2026-10-24, morning). **Withdrawn the same day** (user, 2026-10-24): an open source program must be easy to use, and OpenPGP keys have no recovery of a passphrase either, so the **simple login with a password stays, as it was designed and built** (the password, Argon2id, the key of the database wrapped with AES-256-GCM, `db/encryption/keys.rs`). What follows from it (user, 2026-10-24):
  - **No role master and no server mode**: a strong overcomplication. A recovery of a forgotten password only if a safe and proven mechanism exists for a program installed on a workstation (Claude proposes one); otherwise no recovery.
  - **The password alone at first**; an optional second factor of the login may be added later.
  - **No keyring of GNOME and no secret of the application** (step 13 dropped): it added security at the cost of chaos.
  - **One user per installation.**
  - **Backups may be encrypted with an OpenPGP key, optionally**: without a key the backup is not encrypted, and the window says so and recommends a key; never required.

**Phases** (one commit each, pushed, every test passing; the documents with each)
- [x] 48.1 The devices removed: `hardware_tests/`, `views/devices/`, `db/scan_settings/` (the version 4 of the database of the evaluator removes its two settings), the three commands in their four lists, the page (`ui/src/views/devices/`, `lib/api/devices.ts`, `styles/09-devices.css`, the navigation), the feature `fs` of `rustix`; the documents.
- [x] 48.4 A fresh start of the databases (user, 2026-10-24: the user removes the old installation and tests from the beginning): the migrations 2 to 4 of the development removed, the history of `db/user/migrations.rs` starts again at version 1 and a database of another version is refused; the names of the AI connections unique whatever their case in the table itself (`COLLATE NOCASE`, was an index made by a migration); the default of `message_template` (needed only by the migration) removed; the note on the old columns of `system.db` removed.
- [ ] 48.2 The documents describe the framework (README, `architecture.md` and `CLAUDE.md` brought up to date with 48.1 and 48.3; left: a read of the whole for the framework, `oscal.md`, `history.md`): `README.md`, `architecture.md` (sections 1, 3, 5, 6, 8), `CLAUDE.md`; the steps dropped marked so in this file; `v0.2_functionality.md`.
- [x] 48.3 The role master, more users, the TPM and the constants of the keyring removed; the password stays (decided 2026-10-24): `Role` and the choice of the role at the setup, the page of the master, `accounts::add`, `security/tpm/` and `tss-esapi`, the factor of the device in `db/encryption/keys.rs` (the keys wrapped before open unchanged: the derivation without a device is the same), the columns `role` and `key_device` no longer written (an older `system.db` keeps working), `base64` (used by nothing); one warning instead of two; `architecture.md` sections 3, 5, 6, 8, 9. **The concept on GnuPG below is withdrawn** (kept for the record: the analysis of the flow of GnuPG may serve the backup).

### The concept of 48.3 on GnuPG (proposed by Claude 2026-10-24; withdrawn the same day, the user chose the password)

Found in the code: the keyring was never built (step 13): only two unused constants in `main.rs`. `accounts::add` (more users) is dead code. The TPM factor is half built (`device_secret` always `None`). GnuPG 2.4.7 and `pinentry-gnome3` are on Debian 13.

```text
SETUP
  gpg --list-secret-keys ──► the keys of the evaluator (fingerprint, UID, on a card or not)
  the evaluator chooses their key [+ optionally the recovery key of the laboratory]
  32 random bytes = the key of the database (RAM)
    ──► gpg --encrypt -r <evaluator> [-r <recovery>] ──► an OpenPGP message ──► system.db
    ──► SQLCipher makes users/<id>.db with the key; the key wiped

OPENING (instead of the login)
  the button "Open" ──► gpg --decrypt (the message on its standard input)
    gpg-agent ──► pinentry asks for the passphrase of the key / the PIN of the card
                  (the window of the program never sees the secret)
    the key of the database on its standard output (RAM) ──► SQLCipher opens the database ──► the key wiped
```

- **`security/openpgp/gpg.rs`**: the only place that starts `gpg` (as `pdf/` with Poppler; uses nothing of the program): the secret keys, the public keys, encrypt for recipients, decrypt. A fixed path `/usr/bin/gpg`, never `PATH`; its output read in the machine format (`--with-colons`, `--status-fd`) up to a constant number of bytes; the decrypted key in `Zeroizing`. The tests use a temporary home of GnuPG (`--homedir`) with a key without passphrase made in the test; `~/.gnupg` is never touched.
- **`db/encryption/keys.rs`** rewritten: `WrappedKey` = the OpenPGP message and the fingerprints of its recipients. Argon2id, HKDF, AES-GCM and the rules of the password go.
- **`views/users/users.rs`** stays the one place of the key: `setup_form_keys` (the list of keys), `setup_form_install(name, key, recovery)`, `login_form_open` instead of `login_form_log_in`; the change of the password is replaced by the change of the recipients (a new key after a rotation, a recovery key added).
- **Removed:** `Role` and the choice of the role, the panel of the master in `Shell.svelte`; `accounts::add`; `security/tpm/` whole, `device_secret`, `tss-esapi`, `swtpm`, `libtss2-dev`; `PasswordForm`, `password_form_change`, `argon2`, `aes-gcm`, `hkdf`; the constants of the keyring, section 5.6; `base64` (used by nothing any more).
- **The warnings at the top of the window:** instead of "no password reset" and "no factor", one: "No recovery key: losing your OpenPGP key means losing the data", while the key of the database has one recipient.
- **Phases:** 48.3a the master, more users and the TPM removed (only deletion; the password still works); 48.3b `security/openpgp/` with its tests; 48.3c the setup and the opening with OpenPGP instead of the password (`keys.rs`, the new layout of `system.db`, the forms of the page, the dependencies of cryptography removed); 48.3d the change of the recipients on the page User; then 48.2, the documents of the framework. The backup, encrypted for the same recipients, is a step after 48.
- **Risks accepted knowingly:** gpg-agent keeps the passphrase for a while (meanwhile other programs of the account may use the key; a card with touch lessens it); a key without passphrase cannot be forbidden by the program (a requirement of the laboratory; the list shows which key is on a card); a graphical pinentry is needed (README, requirements).

**Decisions for the user** (Claude's proposal after each):
- [ ] The recovery key of the laboratory: required, recommended or not offered? *Proposed: optional, with a lasting warning while it is missing.*
- [ ] The keyring of GNOME: dropped entirely? *Proposed: yes; the private key of OpenPGP is outside the files of the installation anyway, so it does the work of the secret of the application, and the trouble with PAM on XFCE goes.*
- [ ] The name of the user: from the UID of the key? *Proposed: yes by default, editable.*
- [ ] The installations of the tests: migrated? *Proposed: no (as in step 41): the program recognises an old `system.db` and asks to remove `~/.local/share/io-master` and install anew.*
- [ ] An OpenPGP card (YubiKey, Nitrokey) to test with? Otherwise the tests use keys in software.

---

## Step 49: The recovery code of a forgotten password (2026-10-24)

**State: built 2026-10-24**; the concept approved by the user the same day ("a sensible mechanism, and probably the only sensible option"). The design: `architecture.md`, sections 5.4 and 12; tried in the window on a private display (the setup, the recovery with a code typed in small letters with spaces, a new code with a wrong and the right password).

The key of the database is wrapped a second time, with a random recovery code (160 bits, groups of characters without 0, O, 1, I, with a check sum), the mechanism of BitLocker, FileVault and LUKS: no server.

**Decision of the user (2026-10-24):** the first recovery code is made **automatically when the user is created** and shown right after the setup, before the program opens, as programs of this kind do; it is not something the evaluator has to find in the settings.

**The management of the codes (user, 2026-10-24):**
1. The code appears on the screen after the user is created, as in BitLocker.
2. A new code appears after every change of the password, and the old one stops working (the recovery of a forgotten password with the code is a change of the password as well).
3. The page User offers "Make a new recovery code", which needs the password of the user.

Everything of it is in `views/users/users.rs`, with the password: one place for the keys of the account.

**Tasks**
- [x] `db/encryption/recovery.rs` (a file of its own beside `keys.rs`): the wrapping with the recovery code (HKDF with its own purpose, AES-256-GCM); `model/recovery_code.rs`: the code written and read as text with its check sum (alphabet without 0, O, 1, I; 32 + 3 characters in 7 groups of 5; case, spaces and dashes aside).
- [x] `system.db`: the second copy of the key in columns of its own (`recovery_*`, NOT NULL: no migration, the fresh start of 48.4).
- [x] `views/users/users.rs`: `setup_form_install` and `password_form_change` give the new code; `login_form_recover` ("I do not remember my password"); `recovery_code_form_replace` (needs the password). The page: the component `recovery_code` (shown once, a random group typed to go on), `RecoveryCodeForm` on the page User.
- [x] The warning bar of the window removed: every account has its code from the setup on (no installation before step 49 is left, 48.4).

---

## Step 50: The conversion of PDFs to OSCAL as a view of its own (2026-10-24)

**State: decided by the user 2026-10-24; 50.1 built the same day; 50.2 next, concept first.**

The user (2026-10-24): the conversion of the PDFs of a standard to OSCAL is today a part of the view Evaluation plan (`views/evaluation_plan/`: the documents of standards, a new catalog, the editor of a catalog with the AI). It becomes **a view of its own, whose only aim is the conversion of PDFs to OSCAL, supported by the AI**. The view Evaluation plan keeps the documents of OSCAL and the plan itself (step 41.4).

**The new role of the view Evaluation plan (user, 2026-10-24):** it works only on documents of OSCAL: it shows a document graphically and turns it into the **flow of data of the evaluation**, which the view Evaluation lab then uses. Concept later; the converter comes first.

**The answers taken (user, 2026-10-24: "first the converter"):** the view is named `converter` (the user's word; "Converter" in the navigation, after Chat); the view Evaluation plan shows only the list of the documents of OSCAL (no editor, no view of a catalog for now); the converter makes catalogs now, its name open to other models of OSCAL later; a guided flow step by step (50.2) is discussed after the move.

**Phases**
- [x] 50.1 The move, the logic unchanged (with a list of the catalogs of its own in the converter, `catalog_list/` and `catalog_list_get`, since the editor no longer opens from the view Evaluation plan and a view never uses another; the styles of the tables restored, lost in 48.1 with the sheet of the view Devices): `standard_documents/`, `document_search/`, `new_catalog/`, `catalog_editor/` from `views/evaluation_plan/` to `views/converter/` (Rust and the page), the entry `converter.rs` with their commands (names unchanged), `evaluation_plan.rs` keeps the documents of OSCAL; the navigation; the documents.
- [x] 50.2 The conversion step by step (concept approved 2026-10-24, built the same day, below). **The user (2026-10-24): the conversion is best led step by step, interactively** (variant a: the PDFs, a new catalog, the AI proposes the structure, then the controls group by group, the review; the evaluator confirms every stage).
  **The concept (approved 2026-10-24):** a catalog opens in a wizard of five stages, remembered so that a conversion can be left and resumed: (1) sources (title, version, PDFs; the dialog of a new catalog), (2) table of contents (**the AI finds its pages in the first pages of the document**, user 2026-10-24; the evaluator confirms them in the viewer), (3) groups (proposed by the AI from the table of contents, reviewed; **the program finds where every group starts**, by its heading in the text of the pages, never by the numbers of the AI), (4) controls group by group (the pages of each group filled in, corrected if needed, the AI asked, the proposals reviewed, "this group is done"), (5) review (quotes not found, groups without controls, proposals undecided; "Finish"). The editor of today stays as the tab "Manual editing". The state is ours (`db/conversion/`), linked to the catalog by its UUID. **The view Evaluation plan shows only finished catalogs** (user, 2026-10-24: the evaluator sees only what the planner approved); an imported catalog has no conversion and is finished.
  - [x] 50.2a/b The state of a conversion (`model/conversion.rs`, `db/conversion/`), the wizard element (`catalog_editor/conversion_wizard/`: `progress.rs`, `locate.rs`, `contents.rs`) with its six commands, the task `catalog_contents` of the AI, the start of a conversion with a new catalog, its deletion with the catalog, the plan without unfinished catalogs.
  - [x] 50.2c The page: `ConversionWizard.svelte` (the stages, the groups as cards, the log of the AI), the tabs Conversion and Manual editing of the editor, `build_progress.ts` (the events of the AI, shared with `AiBuilder.svelte`).
  - [x] 50.2d Tried in the window on a private display with a small PDF of four pages and a fake AI service: the AI found the table of contents (page 2), proposed two groups, the program found their pages (3 and 4), the controls proposed and accepted group by group, the review, "Finish", the catalog shown in the Evaluation plan; the conversion resumed after a restart of the program. `architecture.md`, section 12.

---

## Step 51: The evaluation plan: the evaluation project (2026-10-24)

**State: described by the user 2026-10-24 and settled by their answers the same day (below); built 2026-10-24 (51.1 to 51.5). Next: step 52 (the Evaluation lab: the import of a work package and the work of the evaluator).** It gives the step 41.4 (profile and assessment plan as a graph) its form.

**The user's description (2026-10-24), in order:**
1. The **author of the plan** browses the standard (in OSCAL, from the converter or imported) and searches it for requirements.
2. **An option of the AI** reviews the document and proposes a list of requirements, which the author accepts (P1).
3. Every requirement accepted is moved into the **evaluation project**: an element of the view Evaluation plan, and what is **exported to the evaluator**.
4. The evaluation project holds the list of requirements, **shown graphically** (the drawings of the schemes of steps 16 to 27, Svelte Flow, move here).
5. A requirement chosen in the plan shows a **scheme of the inputs the evaluator has to deliver**.
6. When the author adds a requirement, **the AI proposes its scheme**, the inputs that represent it; the author accepts or corrects it.

**The user's answers (2026-10-24), which settle the model:**
- **The project is a list of tasks for the evaluator**, as the laboratories work: the evaluator gets a list of work units to carry out. The planner chooses elements of the standards (of several catalogs) as tasks; **every task is one requirement and has a scheme of its own**, made by the planner with the help of the AI. **No graph of a whole standard**: it would cloud the view and hinder the work.
- **The scheme of a task has three kinds of blocks: input → requirement → output.** The evaluator imports the file of the project, sees the list, opens a task and sees the scheme the planner designed.
  - **Input:** what the evaluator has to deliver to meet the requirement. Evidence has many forms (the Security Target as a PDF, the output of a program, even a conversation), so **an input has a chatbot** that takes the evidence in any form and, connected to the requirement, helps the evaluator judge whether the evidence is processed far enough to be assessed.
  - **Requirement:** the interaction of the evaluator with an agent of the AI: the evaluator consults it whether the inputs meet the requirement. **One task, one requirement: the context of the agent stays small and it works better.**
  - **Output:** the configuration of where the evaluator exports the result of the evaluation.
- **The AI only proposes; the planner approves** (P1). The AI has to understand the text and find elements such as the work units; the structure of OSCAL helps the machine.
- **51.1, the answers of the user (2026-10-24):**
  - **A task is a whole requirement** (a control of a catalog), never a single work unit inside it: requirements are assigned to the evaluators, and the structure stays simple. One requirement, one task.
  - **The master project** holds all the tasks of a standard (of its catalogs) with their schemes; it is made once, when a new version of the standard comes, and **used again and again**. From it the planner **delegates tasks into smaller packages, which are handed to the employees** (the evaluators); this organises the time of the planner.
  - The tasks keep the order in which they were added and are moved by hand; the catalogs of a project are chosen when it is made and may be changed later (a catalog no task uses may be removed).
  - **Names (approved 2026-10-24):** the master project is `evaluation_project` in the code ("Master project" in the window); a package delegated from it is a `work_package` (pakiet pracy). A package only points to tasks of its master, so a scheme is made once; its export (51.5) takes a picture of the state at that moment. **The packages and their export are the stage 51.5.**
  - **The concept of 51.1 (approved 2026-10-24):** our data (`db/evaluation_project/`: the projects, their catalogs, their tasks with their order; a task points to a control of a catalog by UUID and id, so the text of the requirement always comes from the catalog); the view Evaluation plan in two tabs, Projects and OSCAL documents; the screen of a project: the list of tasks beside a browser of the standards (the tree of a catalog, a search, the requirement with its parts and quotes, "Add as a task"); the tree of a catalog becomes a component (`components/catalog_tree/`), used by the converter and the plan; a project uses only finished catalogs; a catalog a project uses cannot be deleted. Phases:
    - [x] 51.1a the model (`model/evaluation_project.rs`) and the database (`db/evaluation_project/`);
    - [x] 51.1b the tree of a catalog as a component (`components/catalog_tree/`, the command `catalog_tree_get` moved there);
    - [x] 51.1c the twelve commands of the view (`project_list_*`, `new_project_*`, `project_form_*`, `task_list_*`, `standard_browser_*`), `Control::label`; a catalog used by a project cannot be deleted;
    - [x] 51.1e **Requirements of one's own** (user, 2026-10-24): a project is not based on the standard alone; the planner may add a requirement of their own, outside the standard, as a task. **Concept approved (2026-10-24), with one shared catalog** (the user: "one shared OSCAL file"): the own requirements of the laboratory live in one catalog of OSCAL, "Requirements of the laboratory", made by the program at the first one and known by its UUID; an own requirement is a control (**its identifier written by the planner, whatever they choose: the program imposes no naming** (user, 2026-10-24), a title, a statement), checked against the schema of NIST; adding one makes it a task of the project and adds the catalog to the project; it stays in the catalog for other projects; it is changed for all, and deleted only while no task points to it. **Built 2026-10-24:** `model/own_requirement.rs`, `db/own_requirements/` (the UUID of the catalog as a setting), the element `own_requirement/` with `own_requirement_add`, `_save`, `_delete`; `Catalog::empty` shared with the converter; the dialog `OwnRequirement.svelte` and "Change"/"Delete" in the browser; tried in the window.
    - [x] 51.1d the page (the tabs Master projects and OSCAL documents; `ProjectList`, `NewProject`, `EvaluationProject` with `ProjectForm`, `TaskList`, `StandardBrowser`), tried in the window on a private display with the example catalog of NIST (a project made, requirements chosen in the tree and in the hits of a search, added as tasks, reordered); `architecture.md` sections 4, 10, 12.
- **51.2, the scheme of a task, as the user sees it (2026-10-24):**
  - Opening a task shows **only the block of the requirement**, inherited from OSCAL. The planner **adds input blocks and connects them to the requirement by hand**, or asks the AI to propose the scheme (51.3).
  - **The document of the standard stays visible** in the screen the whole time, so the planner can check the scheme against the text of the requirement.
  - **Outputs: as many as needed, added and connected by the planner.** Example of the user: one output writes a text file, another fills a LaTeX template; in Common Criteria the results go to at least two documents of different structure and similar content, the **Observation Report** (only when the requirement is not met) and the **pETR**. So **an output has its rules**: its kind (a text file, a LaTeX template), its target, **its condition (always, only when the requirement is not met, only when it is met)** and the instructions with which its AI fills it.
  - The fields of the input (name, expected evidence, instructions for its agent) and of the requirement (instructions for its agent; its text from the catalog) are approved.
  - **Phases of 51.2:**
    - [x] 51.2a the model (`model/task_scheme.rs`: the kinds, the formats and conditions of an output, the rules of the connections and the limits) and the tables `task_blocks`, `task_connections` in `db/evaluation_project/` (a new task gets its block of the requirement; a task and a project go with their schemes);
    - [x] 51.2b the seven commands `task_scheme_*` (get, add a block, save its fields, move it, remove it, connect, disconnect);
    - [x] 51.2c the page: the screen of a task with the drawing (Svelte Flow; `TaskScheme`, `BlockNode`, `SchemeFit`), the form of the block chosen (`BlockForm`), the standard always beside (the parts and, for a quote, the PDF in the embedded `PdfViewer`);
    - [x] 51.2d tried in the window on a private display (2026-10-24): a task opens with only its requirement; inputs and outputs added, connected, a forbidden connection refused, the fields saved, a block moved and found in its place after the task is opened again, a connection and a block removed; `architecture.md`, section 12.
- **51.3, the template of the outputs, the four results of the requirement, the AI proposes (concept approved by the user 2026-10-24):**
  - **The template of the outputs of a project ("Standard outputs")** (user, 2026-10-24): at the start of an evaluation the laboratory decides which documents every requirement goes to (in Common Criteria usually the pETR and the Observation Report), so the project holds a template of outputs (name, format, condition, target, instructions), edited with all four operations (create, read, update, delete). **It is only the default shape of the outputs of every task:** a new task gets a copy of its outputs as ordinary blocks, connected from the requirement, which the planner may change, remove or add to in that task. A change of the template applies to the tasks added after it; the tasks already made keep their outputs (they may have been changed by hand).
  - **The block of the requirement** (user, 2026-10-24) only verifies the inputs and decides whether the requirement is met; its AI also sums up. The block shows the evaluator **the four standard results** (*Findings*, *Analysis*, *Assessment and verdict proposition*, *Vulnerabilities and notes*), which are delivered to the outputs, where the AI of each output adapts them to its template. In the plan they are empty places, filled in the Evaluation lab (step 52). Its instructions say what to check and how to decide.
  - **The AI proposes** (user, 2026-10-24): "Propose with AI" on the screen of a task. The AI reads the whole requirement from its catalog (never shortened), the scope of the project and the blocks already there, and proposes **inputs** (drawn dashed, each accepted or rejected, or all at once) and **corrections of the blocks already there** (of the inputs and of the instructions of the requirement), each with its reason, shown in the form of the block and accepted or rejected; nothing is overwritten without the planner. A new question removes the proposals left undecided. **The AI proposes no outputs**: the template gives them. One question of the role *formal analysis*, through `ai/api/` into the journal; progress as events, a "Stop".
  - **Phases of 51.3:**
    - [x] 51.3a the template of the outputs: the model, the table, its CRUD, the element `standard_outputs/` with its commands, a new task gets its copy; the page (a card in the form of the project). **Built 2026-10-24:** `StandardOutput`, `NewScheme`, `new_scheme` in `model/task_scheme.rs`; the table `standard_outputs`; `standard_outputs_get`, `_add`, `_save`, `_delete`; `scheme::first`; the page `StandardOutputs.svelte` with `OutputFields.svelte` (the fields of an output, shared with `BlockForm`); tried in the window (pETR and the Observation Report in the template, a new task drawn with both). The fallback that gave an old task its block of the requirement when it was read was removed (the data of the tests is temporary);
    - [x] 51.3b the block of the requirement with the four results (`ResultText` in `model/task_scheme.rs`, sent with the scheme; the tile shows the verdict, proposed by the AI and decided by the evaluator, P1, and the four results as empty places; tried in the window);
    - [x] 51.3c the task of the AI, the proposals and the corrections (the model, the database, the commands). **Built 2026-10-24:** `Block::proposal`, `Correction`, `check_correction`, `next_input_place` in `model/task_scheme.rs`; the column `proposal` and the table `block_corrections`; `ai/tasks/task_scheme.rs` (the task `task_scheme`: the blocks named by short ids, never by UUIDs); `task_scheme/propose.rs`; the eight commands `task_scheme_propose`, `_stop`, `_accept`, `_reject`, `_accept_all`, `_reject_all`, `_instructions_get`, `_instructions_save`; tested with a fake AI. **One change from the concept:** the task is asked in the role *text analysis*, not *formal analysis*, since its answer is one JSON object read by the task (the form of *text analysis*); *formal analysis* keeps the four text results of the assessment (step 52). **The layout of the database changed:** the database of the tests made before must be deleted;
    - [x] 51.3d the page of the proposals: "Propose with AI", "Stop", "Instructions…" (`SchemeInstructions.svelte`), "Accept all" / "Reject all" in `TaskScheme`; a proposed block dashed and marked in `BlockNode`; the proposal or the correction with its reason, "Accept" and "Reject" in `BlockForm`;
    - [x] 51.3e tried in the window on a private display with a fake AI service (2026-10-24): a new installation, a connection, the NIST catalog imported, a project with pETR in its template, a task drawn with it; the AI proposed two inputs (dashed, connected) and a correction of the requirement; the correction accepted, one input rejected, the rest accepted at once; the instructions opened. `architecture.md`, section 12.
- **51.4 and 51.5 (the user, 2026-10-24: "you may implement all the functions of step 51"; the concept below written by Claude and carried out without a stop between the stages, the user reviews it afterwards):**
  - **51.4a, the AI proposes the tasks** (point 2 of the description of the user): on the screen of a project, "Propose tasks with AI" for one catalog of the project. The AI reads the scope of the project and the requirements of the catalog that are no task yet (identifier, title, statement), in parts of a bounded size, and proposes the requirements to evaluate, each with its reason. The proposals wait in a list above the tasks: "Accept" makes the task (with the outputs of the template), "Reject" drops it; "Accept all", "Reject all". A new question for a catalog replaces its undecided proposals.
  - **51.4b, the AI finds the related elements of a task** (the work units of the CEM for a requirement of CC Part 3, the test cases of a test specification, ...): on the screen of a task, "Find with AI" reads the requirement and the requirements of the other catalogs of the project, in parts, and proposes the related ones with their reasons; the planner accepts or rejects each; an accepted one is shown beside the standard and travels with the task to the evaluator, and can be removed.
  - **One shared way to the AI in the view** (`components/ai_work/`): one kind of events (part, writing, proposed, failed, end), one "Stop", and the instructions of the three tasks of the AI of the view (`task_scheme`, `requirement_choice`, `related_elements`) behind one pair of commands. The texts of the requirements given to the AI for a choice are cut into parts by `ai/tasks/candidates.rs`, shared by the two new tasks.
  - **51.5a, the work packages** (`work_package`): a folded card on the screen of a project; a package has a name, the evaluator it goes to, a note and its tasks (chosen from the tasks of the master project; **a task is in one package at most**, so that nothing is delegated twice); created, changed, deleted; the list of tasks shows the package of each task.
  - **51.5b, the export of a package** (format b, user 2026-10-24): one JSON file chosen in the dialog of the system: our format (the project, the package, the four results, every task with its requirement, its scheme of **accepted** blocks only and its accepted related elements, linked to the catalogs by their UUIDs and ids) and, as **attachments**, the documents of OSCAL of every catalog it points to, whole and unchanged. A picture of the state at the moment of the export; the import is the work of the Evaluation lab (step 52).
  - **Phases:** [x] 51.4a (`ai_work/`, `ai/tasks/candidates.rs` and `requirement_choice.rs`, the table `task_proposals`, the element `task_proposals/` with six commands, `TaskProposals.svelte`; tested with a fake AI), [x] 51.4b (`ai/tasks/related_elements.rs`, the table `task_links`, the element `task_links/` with four commands, `TaskLinks.svelte` beside the standard; tested with a fake AI), [x] 51.5a (`model/work_package.rs`, the tables `work_packages` and `package_tasks`, the element `work_packages/` with four commands, the package of each task in the list, `WorkPackages.svelte`), [x] 51.5b (`work_packages/export.rs`, `work_packages_export`, the dialog of a place shared with the export of a document of OSCAL), [x] tried in the window on a private display with a fake AI service (2026-10-24): a second catalog (a copy of the NIST catalog standing for a methodology) added to the project; the AI proposed two tasks, one accepted (made with the output of the template), one rejected; on the screen of a task the AI found a related element in the other catalog, accepted; a work package made for "Anna" and exported, the file checked (the accepted blocks only, the related element, both catalogs attached). `architecture.md`, section 12.
- **The evaluator sees a scheme exactly as the planner made it**: several inputs of the planner are several inputs for the evaluator; the same blocks, connections and layout.
- **A requirement may have many inputs** (a fragment of the Security Target, the output of a program, a fragment of the documentation, ...).
- **What the evaluator delivers to an input is kept exactly as delivered, the raw data** (the file as it was, the output of the program, the conversation), **and beside it, as an addition, the interpretation of the AI** (user, 2026-10-24). The interpretation never replaces the raw data: the assessment can always go back to what was delivered (P3, P5).
- **The format of the export (user, 2026-10-24: variant b):** the standard (its catalogs of OSCAL) travels as an **attachment**; **our format holds the list of the requirements (the evaluation tasks) with the information of their schemes**, linked to the catalogs by their ids and UUIDs.
- **Names:** `evaluation_task` (zadanie ewaluacji) in the code and the documents; it does not clash with the tasks of the AI (`ai/api/tasks`).
- **The file is not locked by a signature of its author**; a project is changed only in the view Evaluation plan and carried out only in the view Evaluation lab.
- **The result of a task:** the four texts decided on 2026-10-17 (*Findings*, *Analysis*, *Assessment and verdict proposition*, *Vulnerabilities and notes*) are the default format; laboratories have templates of their own, so **the output block has an instance of the AI too**, which adapts the default format to what the laboratory requires.

---

## Step 52: The Evaluation lab: a work package carried out (2026-10-24)

**State: concept approved by the user 2026-10-24 ("we may implement the whole of point 52"); built the same day, by stages without a stop between them.**

```text
file of a package (.iomaster.json) ──import──► an evaluation (its tasks) ──► a task: its scheme, read only
   input: evidence (kept raw) + the chat of the input + fragments (quotes checked in the evidence, accepted)
   requirement: the agent of the AI on the accepted fragments ──► four results + a verdict proposed ──► the evaluator decides
   outputs: their condition against the verdict ──► the AI adapts the four results to the template (text / LaTeX) ──► files
```

**The answers of the user (2026-10-24):**
1. At the import the evaluator gives **the product (TOE) and its version** (both optional).
2. **Three verdicts: met, not met, inconclusive.** The verdict is the evaluator's (P1); while it is inconclusive or not set, the outputs with a condition wait, the outputs "always" do not.
3. **The evidence: a PDF, a text file, a pasted text** (the notes of an interview, the output of a tool). Other formats (images, ...) are a reminder below.
4. **Nothing returns to the planner: the result of the work of the evaluator is the report**, data exported out of the program (the files of the outputs).
5. The whole step at once.

**Stages:**
- [x] **52.1 The import and the list of evaluations.** "Import a work package…": the file chosen in the dialog of the system, its format, version and size checked, the documents of OSCAL attached checked against the schema of NIST (a catalog stored already stays as it is); an **evaluation** is made: a frozen copy of the tasks, their schemes and related elements (the evaluator does not change a scheme); a package is imported once. The list shows every evaluation with its progress.
- [x] **52.2 The screen of an evaluation.** The tasks with their state (to do, in progress, done); a task opens its scheme read only, beside it the requirement as the standard writes it and its related elements. The drawing of a scheme becomes a component of the page, used by both views.
- [x] **52.3 The inputs.** The evaluator delivers evidence: a file (PDF, text) or a pasted text, kept exactly as delivered (the text of a PDF read page by page for the AI); **the chat of the input** (the instructions of the plan, the evidence) helps to judge whether the evidence is ready; the AI proposes **fragments**: exact quotes the program finds in the text of the evidence, with a separate *AI note*; the evaluator accepts them or adds his own (decisions of 2026-10-17).
- [x] **52.4 The requirement.** Its agent of the AI (role *formal analysis*) reads the requirement, its instructions and **only the accepted fragments**, and proposes the four results and a verdict, with its reason; the evaluator corrects and accepts them and sets the verdict.
- [x] **52.5 The outputs and the report.** An output whose condition holds for the verdict is drafted by its AI from the four results (its target, format and instructions), corrected and accepted by the evaluator. **The report**: the accepted texts of every output, task by task, written into one file per output (`.txt` or `.tex`) in a folder the evaluator chooses.
- [x] **52.6** Tried in the window on a private display with a fake AI service (2026-10-24): the package "Roles" exported in step 51 imported for the product "Card reader"; a text pasted as evidence, two fragments proposed (one found in the evidence and accepted, one not found and rejected), a question to the chat of the input, the agent proposed "met" with the four results (accepted), the pETR drafted and accepted, the report exported (`pETR.tex`, the header and the section of the task). The drawing of a block became the component `BlockNode` of the page; the way to the AI with its events became the component `components/ai_work/`.

**Reminders:** other forms of evidence (images, office documents, archives) later; the import of a new version of a package into an evaluation begun.

---

## Step 53: Zero warnings (2026-10-24 and 25)

**Done 2026-10-24** ("nieużywane funkcje usuwamy", user): the unused code of earlier steps removed, rustc and `cargo clippy --all-targets` give no warning (commit `f5fbfc6`). Removed: the forms of the answer and the rules of memory of the roles (`model/ai.rs`), the fields and objects no rule names (`model/text.rs`), the connection of its own for a role (`db/ai/roles/`: every role uses the active connection), the unused removal of the installation (`lifecycle::uninstall`, `user_db::remove_all`, written anew in step 54). Kept for the tests only, under `#[cfg(test)]`: `db/system/read.rs::user_names` and `db/ai/journal/read.rs::last` (until the page of step 58).

**eslint on the page dropped** (user, 2026-10-25): TypeScript strict and `svelte-check` are the checks of the page; the page is judged by its testers. No automatic tests of the page either, for the same reason.

## Step 55: Backups (dropped 2026-10-25)

Was: backups of the work, optionally encrypted with OpenPGP (`architecture.md`, section 5.5 until then). **Dropped by the user (2026-10-25):** the program keeps nothing that matters to the laboratory besides the projects and the API key; the key is typed again after a reinstallation, and the export of the work (the work package of a master project, the report of an evaluation) makes the work independent of the program, which is the backup.

**Also decided on 2026-10-25:** no viewer of the evidence with the quote of a fragment highlighted (the evaluator has Ctrl+F; what matters is that the AI does not distort the quotes, which the program checks); no data for a demonstration (the standards are licensed; the real test is the colleagues of the laboratory); `cargo-audit` and `cargo-deny` moved to the release (step 10).

## The steps dropped in step 48 (2026-10-24)

IO Master became a framework for the evaluation with AI only (step 48): no devices, no master, no TPM, no passkeys, no keyring. The steps below were planned for the earlier concept and dropped before they were built (steps 8 and 13 partly: their half-built code was removed in 48.3). They are kept as they were written, for the record.

### Step 4: The TLS server of the reset service (changed 2026-10-16)

**Dropped 2026-10-24 (step 48 of this file)**: the program has no master, no TPM, no passkeys and no keyring. Kept below as it was.

**Changed on 2026-10-16:** the program is a desktop program started without parameters (step 14). The TLS server for a remote access to the interface (`--address`) is **dropped**: there is no remote access to the interface, the evaluator works at the computer. `--port`, `--address` and the parser of the command line were removed from `main.rs`. What remains for this step is the network part of the password reset: done right before the master (step 11) and together with it.

**Decisions**
- [x] Network functions are started from the window, not from the command line: the master starts the server of the reset API, the workstation uses its client.
- [ ] Whether the TLS settings of the master can be changed after they are saved (for example to replace an expiring certificate).
- [ ] TLS library: `rustls` with TLS 1.3 only (proposed; the client side already uses it, `ai/anthropic.rs`).
- [ ] How the workstation trusts the certificate of the master (a certificate of the laboratory, or the certificate of the master pinned when the connection is set up).

**Tasks**
- [ ] Master: the certificate and private key of the reset server, checked (they belong together, the certificate is valid) and stored; the server started and stopped in the window.
- [ ] Workstation: the client of the reset API, set up in Settings.
- [ ] Tests: a TLS 1.2 client is refused; a wrong certificate of the master is refused.

**Done when:** a master serves the reset API over TLS 1.3 only, started from its window, and a workstation reaches it.

### Step 6: Port permissions

**Dropped 2026-10-24 (step 48): the program no longer works with devices.** Kept below as it was.

**Decisions**
- [ ] How access to a port is granted from the browser: (a) a button that runs `pkexec` (system password dialog), or (b) the command shown for the user to copy and run. Proposed: (b) first.
- [ ] Which advice is shown: membership in the group of the port (usually `dialout`) as the lasting solution, `chmod` until reconnect as the quick one.

**Tasks**
- [ ] Access of the current user shown for every port (read, write), with the owner, group and permissions.
- [ ] For a port without access: the explanation and the command(s), per the decision above.
- [ ] Fill `permission/permission.rs` with this logic, independent of the UI.

**Done when:** a port without access is explained on the page with a working command; tests cover the advice for each case.

### Step 13: Start: keyring, secret of the application, one user per account

**Dropped 2026-10-24 (step 48 of this file)**: the program has no master, no TPM, no passkeys and no keyring. Kept below as it was.

Agreed with the user on 2026-10-15 during the review (step 12); the concept is in `architecture.md`, sections 3, 3.2, 5.1, 5.2, 5.6 and 9. It replaces the first start told by the system database alone, the data directory chosen with `--data-dir`, and the removal with `--uninstall`.

**Decisions**
- [x] One Linux account holds one installation with **one user of the program**, on a workstation and on a master. Users cannot be added.
- [x] The keyring of the Linux account (GNOME Keyring, Secret Service) holds two entries: the **location** of the installation and the **secret of the application**. All other settings stay in the databases.
- [x] The secret of the application (32 random bytes, made with the databases) is a further secret of the unlocking key (a "pepper"): `HKDF(Argon2id(password) || secret of the application [|| secret of the TPM])`.
- [x] Every damage of a keyring entry or of the installation is a **critical error**: the program does not start. Both entries missing: the start page offers a **new installation** (only in an empty or new folder) or the **restoring** of an existing one (location + recovery code).
- [x] The secret is also the **recovery code**: the alphabet without 0/O/1/I, groups of five characters, a check sum, about 52 characters. Shown again in Settings only after the password is entered again; can be **replaced** (the key of the database is encrypted again).
- [x] The check of the installation at the start lives in `installation/verify.rs` (`verify_installation`, returns the state, never ends the program); the start of the web interface in `web_ui/startup.rs` (`web_startup`, chooses the HTTP or the TLS server by `--address`); `main` only calls them. The checking of the location of the database lives in `db/`.
- [x] The command line keeps only `--port` and `--address`. The removal of the installation moves to the web interface (after login and confirmation).
- [x] Requirement of the system: a desktop session with a keyring through the Secret Service; no start over SSH or without a desktop (wanted: the evaluator works at the computer). Supported desktops: **GNOME and XFCE** (agreed 2026-10-15; the user works on XFCE), both with GNOME Keyring.

**Open**
- [ ] How the location is chosen in the window at the installation (the user has an idea; settled when the interface is reviewed).
- [x] The names of the keyring entries: `KEYRING_DATA_DIR_ENTRY = "io-master.data-dir"` and `KEYRING_APP_SECRET_ENTRY = "io-master.app-secret"` (now in `main.rs`; they move to `installation/verify.rs` with the code that uses them).
- [ ] The red bar "No TPM is configured...": its text, now that the secret of the application protects copied files.
- [ ] The removal in the window: what it asks for (password, typed confirmation).
- [ ] The library for the keyring (for example the crate `keyring` with the Secret Service backend), explained when it is added.
- [ ] Test the keyring on GNOME and on XFCE; describe the setup of XFCE (PAM module `libpam-gnome-keyring`, LightDM, "Launch GNOME services on startup") in the installation guide; a clear message at the start when no Secret Service is available.

**Tasks**
- [ ] `db/`: the check of the location (absolute path, a folder, the owner is the user, permissions `0700`, the system database present and readable).
- [ ] `installation/verify.rs`: reading the two keyring entries, the states of `architecture.md` section 3.2, the critical errors as an enum (the skeleton with `InstallationState` exists since the review of `main.rs`). Tests without the real keyring (a replaceable store).
- [ ] `db/keys.rs`: the secret of the application in the unlocking key.
- [ ] One user: remove adding users (Settings, `accounts::add`, its tests); the system database keeps one account.
- [ ] The start page: new installation, or restoring with location + recovery code.
- [ ] Settings: show the recovery code (after the password), replace the secret.
- [ ] **Removal of the installation in the window** (a function to build, not forgotten): after login, password and a typed confirmation; removes the databases, the two keyring entries and, with the TPM factor, the marker in the TPM. The old console version is kept as a reminder in `installation/uninstall.rs` (not compiled, marked "do not use"); `lifecycle::uninstall` and `user_db::remove_all` are kept for it.
- [x] `--data-dir` and `--uninstall` removed from `main.rs` and `main_tests.rs` (review of `main.rs`, 2026-10-15).
- [ ] Documentation: `architecture.md` is already updated; `CLAUDE.md` and the comments of the code follow.

**Done when:** a new installation records its location and secret in the keyring, a start with a damaged entry or installation refuses to start with a clear reason, a lost keyring is restored with the recovery code, the secret can be shown and replaced, and the tests pass without a real keyring.

### Step 8: The TPM as an optional factor

**Dropped 2026-10-24 (step 48 of this file)**: the program has no master, no TPM, no passkeys and no keyring. Kept below as it was.

Comes after the functions of the workstation (steps 4–7). The program authenticates with passwords only by default (`architecture.md`, section 5). The TPM, built in step 2, becomes a further factor that can be switched on where a TPM exists, for the whole installation.

**Decisions**
- [ ] Where the factor is switched on: at the first start, in Settings afterwards, or both.
- [ ] Switching it on later: every account must log in once to have its key wrapped again with the new factor; what happens to accounts that have not.
- [ ] Whether switching it off is allowed, and what it asks for.

**Tasks**
- [ ] The setting in the system database; the program starts the TPM only when the factor is on.
- [ ] The secret of the device added to the unlocking keys (`db/keys.rs` already takes it as a factor; `with_device_secret` is stored per account).
- [ ] With the factor on: the system database encrypted with a key from the secret of the device, the marker of the installation compared at every start, one installation per computer.
- [ ] `--uninstall` also removes the marker from the TPM.
- [ ] Tests on the emulator `swtpm`.
- [ ] The red bar "No TPM is configured" (shown on every page since step 3) disappears once the factor is on.

**Done when:** the factor can be switched on, an account then needs both the password and the TPM, and the program still works fully without it.

### Step 9: Passkeys as an optional factor

**Dropped 2026-10-24 (step 48 of this file)**: the program has no master, no TPM, no passkeys and no keyring. Kept below as it was.

A further factor next to the TPM, chosen by each user for their own account (`architecture.md`, section 5.1). The TPM binds the data to the computer, a passkey to the person; either, both or neither can be used.

**Decisions**
- [ ] Library: `webauthn-rs` (proposed); check its support of the PRF extension.
- [ ] What a browser or key without PRF gets: a passkey only for logging in (does not protect the files), or no passkey at all.
- [ ] How many passkeys an account may have (proposed: several, so that a spare key in a drawer prevents the loss of the data).

**Tasks**
- [ ] Registering and removing passkeys in the Settings of an account.
- [ ] Login with password and passkey; the secret from the PRF extension becomes part of the unlocking key (`db/keys.rs`), the key of the database is wrapped once per registered passkey.
- [ ] Tests with a software authenticator.

**Done when:** an account with a passkey opens only with the password and the passkey, and losing one of two registered passkeys loses no data.

### Step 11: Master and password reset (last stage)

**Dropped 2026-10-24 (step 48 of this file)**: the program has no master, no TPM, no passkeys and no keyring. Kept below as it was.

The workstation is the core of the program and is completed first (steps 0–10). The master comes last.

**Needs:** a second machine for the master; a virtual machine with its own virtual TPM (`swtpm`) is enough. Master and workstation never run on the same computer (`architecture.md`, section 3.1).

**Decisions**
- [ ] How a workstation and the master recognize each other (mutual TLS with certificates exchanged at enrollment, proposed).
- [x] The administrator of the master is the user created at its first start (step 1).

**Tasks**
- [ ] Master: API that receives reset requests, a panel where the administrator approves or rejects them, shares sealed in its TPM.
- [ ] Workstation: settings page to configure the reset service; creates the recovery copy of the key of the database and hands the master its share.
- [ ] Reset flow: request, approval, share handed out, new password set on the workstation.
- [ ] The orange bar disappears once a reset service is configured.
- [ ] Settings of a workstation: connect it to a master after installation, change the master, or disconnect it.
- [ ] Every request and decision recorded on the master.

**Done when:** a forgotten password is reset with the master on a second machine; neither side alone can decrypt the data (shown by tests).

---

## The steps replaced by steps 51 and 52 (2026-10-24)

The evaluation was planned on 2026-10-17 as the steps 23 and 29 to 36, and on 2026-10-18 on OSCAL assessment results (step 41.5). It was built instead as the master projects of the Evaluation plan (step 51: a scheme of inputs, requirement and outputs per task, work packages) and the Evaluation lab (step 52: the evidence, the fragments, the assessment on the accepted fragments only, the outputs and the report). The decisions of the user of 2026-10-17 below (one requirement at a time, the two phases, the four text results, nothing of the AI without the evaluator) are those step 52 carries out; the report in the LaTeX template (step 28) is the outputs of step 52, and the documentation of the manufacturer (step 30) enters as the evidence of the inputs. The plans are kept as they were written.

### Step 23: Evaluation with AI: carrying out an evaluation plan (steps 29 to 36)

Planned with the user on 2026-10-17. Step 23 is the name of the whole; it is built in the steps 29 to 36 below, each with its own concept, tests and approval. Decisions of the user are marked (user).

**Changed by step 41 (2026-10-18).** The steps below were planned on the *test project* of steps 16 to 27 (schemes of tiles: inputs, requirements, outputs). That model was replaced by **NIST OSCAL** and removed in step 41.1 (`oscal.md`): an evaluation now carries out an **OSCAL assessment plan** and records its work as **OSCAL assessment results** (observations with their evidence, findings per objective, risks, the log of the assessment), built in step 41.5. The decisions of the user below still hold, mapped onto OSCAL (`oscal.md`, section 5); the tasks of steps 31 to 36 are designed again on the assessment results in the concept of each step.

**The idea.** An **evaluation** carries out one evaluation plan for one product (the TOE). The plan says *what* has to be done (the requirements chosen in its profile, the tasks and activities of its assessment plan); the evaluation records *how it was done*. The AI proposes, the evaluator decides (P1); every proposal carries verified quotes; everything is traceable (P3).

```text
assessment plan (OSCAL) ──handed over──► assessment results (OSCAL) of one product
                                           ├─ evidence:      observations (examine, interview, test) with their
                                           │                 relevant evidence (documentation of step 30, files, tests)
                                           ├─ requirements:  phase 1: fragments of the documentation, accepted by the evaluator
                                           │                 phase 2: the AI proposes, on the accepted fragments only, the four
                                           │                 results: findings, analysis, assessment and verdict proposition,
                                           │                 vulnerabilities and notes; the evaluator accepts or writes them
                                           ├─ findings:      per objective, their status set by the evaluator; risks
                                           ├─ report:        drafted by the AI, approved by the evaluator (step 28: LaTeX)
                                           └─ log:           every question to the AI, every acceptance and every change (P3)
```

**Decisions (user, 2026-10-17)**
- [x] **One requirement at a time.** The window of the evaluation goes plan → one requirement, which the evaluator works on; not the whole graph at once, so that the work stays readable.
- [x] **The documentation of the product is analysed in two phases.** Phase 1 finds the fragments of the documentation that matter for the requirement; phase 2 assesses the requirement on those fragments only, not on the whole document.
- [x] **A fragment = a verified quote + a separate AI note.** The quote is exact text of the document, checked on its page by the program (as `quote.rs` does for the standard). A summary of the context or any explanation of the model goes into a separate field, shown as *AI note*, never as a quote.
- [x] **Only accepted fragments take part in the evaluation.** A methodological requirement (user): a fragment proposed by the AI is used in phase 2 only after the evaluator has accepted it; a workflow driven by the AI alone would not stand an audit. The evaluator may also add a fragment by hand (selected in the viewer).
- [x] **The documentation of the manufacturer is analysed in a tool of its own** (step 30, designed later); the evaluation reads its results (in OSCAL: the system security plan and the component definition, `oscal.md`).

**Decisions (user, 2026-10-17, second round)**
- [x] **An evaluation is a separate process that starts when a plan is handed over.** Then: the test project was copied into an *evaluation project*; since step 41: the assessment plan is handed over and the evaluation records its own assessment results, so that later changes of the plan do not touch it.
- [x] **No fixed verdicts.** The program supports many standards, so it imposes no list of verdicts. Every requirement gets four **results of the evaluation**, each a text field filled by the AI or the evaluator according to the standard:
  1. *Findings*;
  2. *Analysis*;
  3. *Assessment and verdict proposition*;
  4. *Vulnerabilities and notes*: the vulnerabilities that may follow from the solution used.
- [x] **The AI writes text proposals, marked as AI; the evaluator must accept them.** No text of the AI becomes a result without that acceptance.
- [x] **The AI service is the matter of the laboratory**, also for the documents of the manufacturer (often under NDA): the program imposes no provider and does not block (as for the standard).
- [x] **The journal comes at once** (from step 29).
- [x] The evaluation is built next (user, 2026-10-17).

### Step 28: Reports in the LaTeX template of the laboratory (later)

Designed with or after step 23. The laboratory provides a LaTeX template; the results of an evaluation (its OSCAL assessment results) are written into it, automatically by default.

### Step 29: Evaluation: the evaluation project, the results and the journal

Built on 2026-10-17 on the test projects and removed with them in step 41.1; its record is in `history.md`. Its decisions (the four results, the proposals kept apart, the journal that is only added to) are carried onto the OSCAL assessment results in step 41.5.

### Step 30: Tool "Manufacturer documentation" (later)

Designed later (user, 2026-10-17); a part of the view Evaluation lab since step 43 (user, 2026-10-23), whose placeholder (`ui/src/views/evaluation_lab/`) names it. The documents of the manufacturer (Security Target, guidance, design) are loaded and analysed in a way proper to them; in OSCAL they become the system security plan and the component definition (`oscal.md`, section 1), which an assessment plan imports and an evaluation reads. Steps 32 to 34 need it.

### Step 31: The window of the evaluation

- [ ] The list of evaluations; a new evaluation from an evaluation plan (an assessment plan of OSCAL).
- [ ] Plan → one requirement: what it needs, its fragments, assessments, findings; the progress of the whole evaluation.

### Step 32: Inputs provided

- [ ] The evaluator marks what a requirement needs as provided, open or not applicable: by the documentation of step 30, an evidence file (with SHA-256), or a note; in OSCAL the subjects and the relevant evidence of the results. A need *provided by* another requirement is met when that requirement is.
- [ ] A requirement is ready to be assessed when all it needs is provided.

### Step 33: Phase 1: the AI finds the fragments of a requirement

- [ ] For the chosen requirement the AI reads the documentation of its inputs part by part and proposes fragments: a quote and a separate AI note.
- [ ] Every quote checked on its page; the fragment shown highlighted in the viewer; accepted, corrected or rejected by the evaluator; fragments added by hand.

### Step 34: Phase 2: the assessment of a requirement

- [ ] The requirement (its control in the catalog and its quotes from the standard, P5) and **only its accepted fragments** go to the model; it proposes the four results as text (findings; analysis; assessment and verdict proposition; vulnerabilities and notes) and names what is missing.
- [ ] The proposals are marked AI and become results only when the evaluator accepts them (or writes their own); every run is recorded in the journal. Where something is missing, phase 1 can be repeated for this requirement.

### Step 35: Activities, tests and findings

- [ ] A test recorded as an observation with the method *test* (steps, tools and configuration as assessment assets of the plan, date, who); findings. The AI may propose a test plan as text; **no command reaches a device** (P2); guided execution on a device comes later.

### Step 36: Outputs

- [ ] The report assembled from steps 32 to 35 (the findings, the verdicts and their rationale, the vulnerabilities and notes from the accepted results of the requirements, user 2026-10-17); the AI drafts the text by the instructions of the laboratory, the evaluator corrects and approves it. Shown in the window until step 28 (LaTeX).

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
