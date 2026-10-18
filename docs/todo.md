# IO Master: the plan of the work

The steps that take the program from where it is now to the design in `architecture.md`. They are done **one at a time, in the order of work below**; the order is a guide, not an absolute rule (user, 2026-10-17): architecture, readability and security come first. When the last step is done, the next functionality is planned anew.

The number of a step is its name and never changes, so that references in the code (`TODO (step 7 of docs/todo.md)`) stay valid. **The steps already built, with their decisions and tasks, are in `history.md`**; this file keeps the order of all steps and the details of the open ones.

| Order | Step | State |
|---|---|---|
| 1 | 0: Put the current state in order | done (`history.md`) |
| 2 | 1: Web server skeleton | done; replaced by the window of step 14 (`history.md`) |
| 3 | 2: Secrets in the TPM (now an optional factor, see step 8) | done (`history.md`) |
| 4 | 3: Databases, accounts and the management of users | done (`history.md`) |
| 5 | 5: Scan page | done (`history.md`) |
| 6 | 7: AI chat with Anthropic | built; its connection replaced by step 15 (`history.md`) |
| 7 | 14: The user interface: Tauri 2 + Svelte | built; waiting for the user's review (`history.md`) |
| 8 | 15: Configurable AI connections (local models) | built; waiting for the user's test (`history.md`) |
| 9 | 16 to 22: Evaluation schemes (data model, documents, viewer, drawing, AI analysis, checks, exchange) | built 2026-10-16; waiting for the user's test (`history.md`) |
| 10 | 24 to 27: Test projects and chapters | built 2026-10-16; waiting for the user's test (`history.md`) |
| 11 | 37: The database layer by areas and CRUD | built 2026-10-17; reworked by step 40 (`history.md`) |
| 12 | 38: The AI layer | built 2026-10-17; reworked by step 40 (`history.md`) |
| 13 | 39: The domain folders and the objects in memory | built 2026-10-17; reworked by step 40 (`history.md`) |
| 14 | **40: The cleanup of the architecture** | **built 2026-10-17 on the branch `v0.5`; waiting for the user's review** (below) |
| 14a | **41: The evaluation plan (was: test project) on the model of NIST OSCAL** | **concept in `docs/oscal.md`, approved 2026-10-18; 41.1 to 41.3 built, 41.4 next; before step 31** |
| 15 | 12: Review, order and functional hardening of the whole code | started anew on the code of v0.5 (`review.md`) |
| next | 23: Evaluation with AI: carrying out a scheme, built as steps 29 to 36 | plan agreed (2026-10-17); step 29 built (its core), waiting for the user's review; step 31 next |
| later | 30: Tool "Manufacturer documentation" | designed later (user); a placeholder in the sidebar; needed by steps 32 to 34 |
| later | 28: Reports in the LaTeX template of the laboratory | with or after step 23 |
| 16 | 13: Start: keyring, secret of the application, one user per account | after the review |
| 17 | 6: Port permissions | |
| 18 | 8: The TPM as an optional factor | |
| 19 | 9: Passkeys as an optional factor | |
| 20 | 10: Before release of the workstation | |
| 21 | 4: The TLS server of the reset service (changed 2026-10-16) | before the master |
| 22 | 11: Master and password reset | last |

## How every step is done

1. **Concept.** Claude proposes how the step is built (files, dependencies, open decisions). The user approves or corrects it. No code before that.
2. **Implementation** in small parts, each with its unit tests (`<file>_tests.rs`), in the layers of `code_structure.md`.
3. **Check.** `cargo fmt --check` clean, builds without warnings, all tests (also of the architecture and of the command lists), `npm run check` and `clippy` pass; the user tries the result.
4. **Approval** by the user, then a commit to the branch of the version (`v0.5` now; `v0.4` keeps the state before step 40).

Each step lists its **decisions** (to settle in the concept), its **tasks**, and when it is **done**. Prerequisites the user has to provide (packages, permissions) are marked **needs**.

Legend: `[ ]` to do, `[x]` done.

---

## Step 40: The cleanup of the architecture (2026-10-17)

Built on the branch `v0.5` (the state before it is kept in `v0.4`), on the analysis of the code of 2026-10-17 and the guidelines of the user. **The structure and the rules that came out of it are in `code_structure.md`**; the guidelines are in `CLAUDE.md`, section "The architecture".

**Decisions of the user**
- [x] The folders `model/`, `db/`, `ai/`, `hardware_tests/`, `security/`, `business/`, `tauri_ui/`; the business folder named `business/`.
- [x] The operations of a resource are functions named after REST verbs (`projects::create`), as in `ai/api/`.
- [x] The questions to the AI and the reading of its answers are in `ai/tasks/`.
- [x] The README keeps both aims, written anew for the project as it is now.
- [x] No approval of the review moves with the code: every file of v0.5 is reviewed from the start.
- [x] The modified code goes to the branch `v0.5`.

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

**Parts** (one commit each, on `v0.5`):
- [x] 41.1 The rename to *evaluation plan*; the old model of the test projects and the evaluations removed with its tables (no migration).
- [x] 41.2 The foundation: the JSON schemas of OSCAL 1.2.3 in the repository (`oscal/`), the types of `model/oscal/` with the check against the schemas, `db/oscal/`; import, export and deletion of documents in the window (part 1: types and schemas; part 2: storage and window).
- [x] 41.3 Own catalogs made in the program: part 1 the catalogs and the evaluation support (quotes, proposals); part 2 the AI proposes the groups (from a table of contents) and the controls (from the pages of a group), the evaluator accepts, corrects or rejects; part 3 the window: the editor of a catalog (tree, object, its quotes in the viewer, its fields as JSON of OSCAL; the AI with progress and Stop; the proposals; the instructions of the two tasks). Tested in the window with a fake model.
- [ ] 41.4 Profile and assessment plan (the evaluation plan), shown as a graph; the CEM and the Protection Profiles as documents of OSCAL that a plan can take.
- [ ] 41.5 Assessment results (the evaluation), then steps 31 to 36 on them.

---

## Step 4: The TLS server of the reset service (changed 2026-10-16)

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

---

## Step 6: Port permissions

**Decisions**
- [ ] How access to a port is granted from the browser: (a) a button that runs `pkexec` (system password dialog), or (b) the command shown for the user to copy and run. Proposed: (b) first.
- [ ] Which advice is shown: membership in the group of the port (usually `dialout`) as the lasting solution, `chmod` until reconnect as the quick one.

**Tasks**
- [ ] Access of the current user shown for every port (read, write), with the owner, group and permissions.
- [ ] For a port without access: the explanation and the command(s), per the decision above.
- [ ] Fill `permission/permission.rs` with this logic, independent of the UI.

**Done when:** a port without access is explained on the page with a working command; tests cover the advice for each case.

---

## Step 12: Review, order and functional hardening of the whole code

The concept changed several times while steps 0 to 7 were built (CLI removed, TPM made optional, single host removed, roles). Before more functions are added, the whole code is gone through once, module by module, so that it matches `architecture.md` and `CLAUDE.md` again and stays easy to keep an overview of. No new functions in this step.

**How**
- The review is led by the user: the user goes through the functions in an order of their choice and asks questions. Claude answers, explains the code and its reasons, and proposes a change where a question shows a problem.
- Claude does not review the architecture on its own initiative. Changes are made only when the user asks for them or approves a proposal; each change with its tests, then build, tests and clippy; commits when the user approves.
- The lists below are a checklist for the user, not a plan that Claude works through by itself.

**Progress** (the user decides the order and the priority)
- [x] `main.rs`: reviewed and approved by the user (2026-10-15). Changes: `--data-dir` and `--uninstall` removed; a flag in the place of a value is refused; the check of the installation moved to `installation/verify.rs`, the start of the server (HTTP, or TLS with `--address`) and Ctrl+C to `web_ui/startup.rs`; the header describes the present state.
- [ ] `web_ui/`: high priority, **on hold** until step 14 decides whether the web interface stays (no review of code that may be replaced).
- [ ] `installation/verify.rs` (`verify_installation`): lower priority, reviewed later; it is rebuilt anyway in step 13 (keyring).
- [ ] The other modules: in an order the user chooses.

**What is checked in every module**
- [ ] Agreement with the concept: no remains of earlier concepts (comments, names, code paths, documentation of files that no longer exist).
- [ ] Dead code and `allow(dead_code)` that is no longer needed; empty folders (`uart/`) and placeholder files.
- [ ] One responsibility per file; repeated code (for example the `workstation()` check in every page) moved to one place; files that grew too large split.
- [ ] Rules of `CLAUDE.md`: core without printing, errors as enums, serialization only in `web_ui/serializer/` (does not exist yet; decide what goes there, for example the JSON of the chat events), cancellation by the caller.
- [ ] Errors: every failure reaches the user as a clear message; nothing panics on input from outside; no secret in any message.
- [ ] Tests: missing cases, tests that test nothing, the same test helper written twice.
- [ ] Comments: thorough, English, true to the code.

**Functional hardening (candidates; the list is settled in the concept of each module)**
- [ ] Login: limit on failed attempts (per name and in total), without telling which names exist.
- [ ] Sessions: expiry after inactivity and a longest lifetime; a limit of open sessions; all sessions of a user ended on a change of password.
- [ ] Forms: answer every POST with a redirect (as the scan page does), so that reloading a page never sends a form again.
- [ ] Limits of request size and of the length of every field; the same checks of names and passwords everywhere.
- [ ] The data directory: permissions (`0700`, files `0600`) checked at the start; a clear message when they are wider or the owner differs.
- [ ] Databases: behaviour on a full disk, a damaged file, a database locked by a second running program (one program per data directory).
- [ ] Two programs started on the same data directory or port.
- [ ] Protective headers and the CSP checked again for every page, also for the answers of errors and the event stream.

**Done when:** every module has been reviewed and approved, the documentation describes the code as it is, and build, tests and clippy pass.

---

## Step 13: Start: keyring, secret of the application, one user per account

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
- [ ] How the location is chosen in the web interface at the installation (the user has an idea; settled when the interface is reviewed).
- [x] The names of the keyring entries: `KEYRING_DATA_DIR_ENTRY = "io-master.data-dir"` and `KEYRING_APP_SECRET_ENTRY = "io-master.app-secret"` (now in `main.rs`; they move to `installation/verify.rs` with the code that uses them).
- [ ] The red bar "No TPM is configured...": its text, now that the secret of the application protects copied files.
- [ ] The removal in the web interface: what it asks for (password, typed confirmation).
- [ ] The library for the keyring (for example the crate `keyring` with the Secret Service backend), explained when it is added.
- [ ] Test the keyring on GNOME and on XFCE; describe the setup of XFCE (PAM module `libpam-gnome-keyring`, LightDM, "Launch GNOME services on startup") in the installation guide; a clear message at the start when no Secret Service is available.

**Tasks**
- [ ] `db/`: the check of the location (absolute path, a folder, the owner is the user, permissions `0700`, the system database present and readable).
- [ ] `installation/verify.rs`: reading the two keyring entries, the states of `architecture.md` section 3.2, the critical errors as an enum (the skeleton with `InstallationState` exists since the review of `main.rs`). Tests without the real keyring (a replaceable store).
- [ ] `db/keys.rs`: the secret of the application in the unlocking key.
- [ ] One user: remove adding users (Settings, `accounts::add`, its tests); the system database keeps one account.
- [ ] The start page: new installation, or restoring with location + recovery code.
- [ ] Settings: show the recovery code (after the password), replace the secret.
- [ ] **Removal of the installation in the web interface** (a function to build, not forgotten): after login, password and a typed confirmation; removes the databases, the two keyring entries and, with the TPM factor, the marker in the TPM. The old console version is kept as a reminder in `installation/uninstall.rs` (not compiled, marked "do not use"); `lifecycle::uninstall` and `user_db::remove_all` are kept for it.
- [x] `--data-dir` and `--uninstall` removed from `main.rs` and `main_tests.rs` (review of `main.rs`, 2026-10-15).
- [ ] Documentation: `architecture.md` is already updated; `CLAUDE.md` and the comments of the code follow.

**Done when:** a new installation records its location and secret in the keyring, a start with a damaged entry or installation refuses to start with a clear reason, a lost keyring is restored with the recovery code, the secret can be shown and replaced, and the tests pass without a real keyring.

---

## Step 8: The TPM as an optional factor

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

---

## Step 9: Passkeys as an optional factor

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

---

## Step 10: Before release of the workstation

- [ ] **The TPM factor (step 8) uses a real TPM 2.0 (`/dev/tpmrm0`, the user in the group `tss`) instead of the emulator `swtpm`; the release refuses an emulated TPM.**
- [ ] Without the TPM factor the system database is not encrypted and the private key of the TLS server (step 4) has no key to protect it at program start: decide how it is protected without a TPM.
- [ ] Final login: limits on failed attempts, session expiry, logout everywhere.
- [ ] Security review of the whole program.
- [ ] Installation package: the program and its start as a service; the role is chosen at the first start (step 1).

---

## Step 11: Master and password reset (last stage)

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

---

## Step 28: Reports in the LaTeX template of the laboratory (later)

Designed with or after step 23. The laboratory provides a LaTeX template; the results of an evaluation (the outputs and their seven sections) are written into it, automatically by default.

---

## Step 23: Evaluation with AI: carrying out a scheme (steps 29 to 36)

Planned with the user on 2026-10-17. Step 23 is the name of the whole; it is built in the steps 29 to 36 below, each with its own concept, tests and approval. Decisions of the user are marked (user).

**The idea.** An **evaluation** carries out one test project (marked ready) for one product (the TOE). The project says *what* has to be done (inputs, requirements, outputs, links); the evaluation records *how it was done*. The AI proposes, the evaluator decides (P1); every proposal carries verified quotes; everything is traceable (P3).

```text
ready test project ──copy──► evaluation of one product
                               ├─ inputs:        provided by (a documentation project of step 30, an evidence file,
                               │                 the result of another requirement), marked by the evaluator
                               ├─ requirements:  phase 1: fragments of the documentation, accepted by the evaluator
                               │                 phase 2: the AI proposes, on the accepted fragments only, the four results:
                               │                 findings, analysis, assessment and verdict proposition,
                               │                 vulnerabilities and notes; the evaluator accepts or writes them
                               ├─ outputs:       the seven sections of the report, drafted by the AI, approved by the evaluator
                               └─ journal:       every question to the AI, every acceptance and every change (P3)
```

**Decisions (user, 2026-10-17)**
- [x] **One requirement at a time.** The window of the evaluation goes project → scheme (chapter) → one requirement, which the evaluator works on; not the whole graph of a chapter at once, so that the work stays readable.
- [x] **The documentation of the product is analysed in two phases.** Phase 1 finds the fragments of the documentation that matter for the requirement; phase 2 assesses the requirement on those fragments only, not on the whole document.
- [x] **A fragment = a verified quote + a separate AI note.** The quote is exact text of the document, checked on its page by the program (as `quote.rs` does for the standard). A summary of the context or any explanation of the model goes into a separate field, shown as *AI note*, never as a quote.
- [x] **Only accepted fragments take part in the evaluation.** A methodological requirement (user): a fragment proposed by the AI is used in phase 2 only after the evaluator has accepted it; a workflow driven by the AI alone would not stand an audit. The evaluator may also add a fragment by hand (selected in the viewer).
- [x] **The documentation of the manufacturer is analysed in a tool of its own** (step 30, designed later): an input tile of the evaluation (for example "Security Target") reads the projects of that tool.

**Decisions (user, 2026-10-17, second round)**
- [x] **An evaluation is a separate process that starts when a project is imported.** The test project is transformed into a new form, the **evaluation project** (a copy: later changes of the test project do not touch it), and the workflow of the evaluation starts on it.
- [x] **No fixed verdicts.** The program supports many standards, so it imposes no list of verdicts. Every requirement gets four **results of the evaluation**, each a text field filled by the AI or the evaluator according to the standard:
  1. *Findings*;
  2. *Analysis*;
  3. *Assessment and verdict proposition*;
  4. *Vulnerabilities and notes*: the vulnerabilities that may follow from the solution used.
- [x] **The AI writes text proposals, marked as AI; the evaluator must accept them.** No text of the AI becomes a result without that acceptance.
- [x] **The AI service is the matter of the laboratory**, also for the documents of the manufacturer (often under NDA): the program imposes no provider and does not block (as for the standard).
- [x] **The journal comes at once** (from step 29).
- [x] The evaluation is built next (user, 2026-10-17).

## Step 29: Evaluation: the evaluation project, the results and the journal

Concept approved by the user on 2026-10-17: the import from both sources (a ready test project of the installation, and the file of an exported project); only the accepted part is copied; a seventh section of an output, *Vulnerabilities and notes*; the module `product_evaluation/`. The states of the inputs come with step 32, the fragments with step 33.

- [x] The import: a test project → an evaluation project (a frozen copy of its accepted schemes, tiles, sources and links; the proposals left out and counted); the rules of `scheme.rs` checked again; the fields of the evaluation (name, product, version, manufacturer, evaluator) and its origin as text.
- [x] The four results of every requirement (findings; analysis; assessment and verdict proposition; vulnerabilities and notes) as text; the proposals of the AI kept apart, marked AI, made results only by the evaluator (accept, correct, reject; or written alone); the origin of every result.
- [x] The journal, append-only, kept so by the database (two triggers); every step written in the same transaction as its change.
- [x] The seventh section of an output, *Vulnerabilities and notes* (`evaluation/scheme.rs`).
- [ ] The user's review and approval.

**As built (2026-10-17)**
- `src/product_evaluation/`: `model.rs` (types, `ResultField`, `ResultOrigin`, `JournalAction`, `EvaluationError`, checks), `import.rs` (`from_project`: only a project marked ready; `from_exchange`: the documents found or loaded by the caller, the quotes checked again), `projects.rs` (`create` in one transaction with the first entry of the journal, `list`, `get`, `data`, `tile`, `delete` with `secure_delete`), `results.rs` (`propose`, `accept`, `correct`, `reject`, `write`, `of`, `proposals`), `journal.rs` (`append`, `of`), `test_support.rs`; 21 new tests (310 in all).
- Database: tables `evaluations`, `evaluation_documents`, `evaluation_schemes`, `evaluation_tiles`, `evaluation_tile_sources`, `evaluation_links`, `evaluation_results`, `evaluation_proposals`, `evaluation_journal` with the triggers `evaluation_journal_unchanged` and `evaluation_journal_kept` (`db/user_db.rs`). A document of a standard used by an evaluation cannot be deleted (`evaluation/documents.rs`).
- Not used by the window yet (step 31): the compiler warns of unused code until then.
- Not committed.

## Step 30: Tool "Manufacturer documentation" (later)

Designed later (user, 2026-10-17); its place in the sidebar holds a placeholder (`ui/src/pages/Documentation.svelte`). The documents of the manufacturer (Security Target, guidance, design) are loaded and analysed in a way proper to them, as projects of this tool; the input tiles of an evaluation read those projects. Steps 32 to 34 need it.

## Step 31: The window of the evaluation

- [ ] The list of evaluations; a new evaluation from a ready project.
- [ ] Project → scheme → one requirement: its inputs, fragments, assessments, verdict; the progress of the whole evaluation.

## Step 32: Inputs provided

- [ ] The evaluator marks an input provided, open or not applicable: by a project of step 30, an evidence file (with SHA-256), or a note. An input *provided by* a requirement is met when that requirement is.
- [ ] A requirement is ready to be assessed when all its inputs are met.

## Step 33: Phase 1: the AI finds the fragments of a requirement

- [ ] For the chosen requirement the AI reads the documentation of its inputs part by part and proposes fragments: a quote and a separate AI note.
- [ ] Every quote checked on its page; the fragment shown highlighted in the viewer; accepted, corrected or rejected by the evaluator; fragments added by hand.

## Step 34: Phase 2: the assessment of a requirement

- [ ] The requirement (its quotes from the standard, P5) and **only its accepted fragments** go to the model; it proposes the four results as text (findings; analysis; assessment and verdict proposition; vulnerabilities and notes) and names what is missing.
- [ ] The proposals are marked AI and become results only when the evaluator accepts them (or writes their own); every run is recorded in the journal. Where something is missing, phase 1 can be repeated for this requirement.

## Step 35: Activities, tests and findings

- [ ] Sections 3 and 5 of an output: a test recorded (method, steps, tools and configuration, date, who); findings. The AI may propose a test plan as text; **no command reaches a device** (P2); guided execution on a device comes later.

## Step 36: Outputs

- [ ] The seven sections assembled from steps 32 to 35 (sections 5 *Findings*, 6 *Verdict and rationale* and 7 *Vulnerabilities and notes* from the accepted results of the requirements, user 2026-10-17); the AI drafts the text by the instructions of the output, the evaluator corrects and approves it. Shown in the window until step 28 (LaTeX).

---

## After this list

Not planned in detail yet; to be designed once the steps above are done:

- Further AI providers (Gemini, local models) and AI on the data of the devices.
- Backup of the databases with its own encryption, independent of the TPM; also the way to move to a new computer (`architecture.md`, section 5.5).
- Git of the laboratory over a secure connection, with checks of its configuration (`architecture.md`, section 6).
- Filling the LaTeX report templates of the laboratory and committing the result.
- Git panel in the web UI.
- The other functions of v0.2 (`v0.2_functionality.md`): UART read and write with the confirmation of every command (P2), Bluetooth.
- A build variant without the code of the master (`architecture.md`, section 3.2).
- Beyond the evaluation (steps 16 to 36): guided execution of functional tests on the device. (The documentation of the product is now step 30.)
