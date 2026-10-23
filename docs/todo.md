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
| 9 | 16 to 22: Evaluation schemes (data model, documents, viewer, drawing, AI analysis, checks, exchange) | built 2026-10-16; its model replaced by NIST OSCAL and removed in step 41.1; the documents, the viewer and the check of quotes kept (`history.md`) |
| 10 | 24 to 27: Test projects and chapters | built 2026-10-16; replaced by NIST OSCAL and removed in step 41.1 (`history.md`) |
| 11 | 37: The database layer by areas and CRUD | built 2026-10-17; reworked by step 40 (`history.md`) |
| 12 | 38: The AI layer | built 2026-10-17; reworked by step 40 (`history.md`) |
| 13 | 39: The domain folders and the objects in memory | built 2026-10-17; reworked by step 40 (`history.md`) |
| 14 | **40: The cleanup of the architecture** | **built 2026-10-17; waiting for the user's review** (below) |
| 14a | **41: The evaluation plan (was: test project) on the model of NIST OSCAL** | **concept in `docs/oscal.md`, approved 2026-10-18; 41.1 to 41.3 built, 41.4 next; before step 31** |
| 14b | **42: The views: one folder per element of the window, instead of `business/` and the commands of `tauri_ui/`** | **built 2026-10-21: every view moved, `business/` and the commands of `tauri_ui/` removed** (below) |
| 14c | **43: One vocabulary, views and components, `tauri_ui/` renamed** | **built 2026-10-23, in the phases 43.1 to 43.9** (below) |
| 14d | **44: The tests in the file they test** | **built 2026-10-23, in the phases 44.1 to 44.6** (below) |
| 14e | **45: The tree of the modules in `main.rs`** | **built 2026-10-23, in the phases 45.1 to 45.4** (below) |
| 14f | **46: The entry of a view** | **built 2026-10-23, in the phases 46.1 to 46.7** (below) |
| 14g | **47: The view `users`: the account, the session and the encryption in one place** | **approved 2026-10-23; built in phases 47.1 to 47.3** (below) |
| 15 | 12: Review, order and functional hardening of the whole code | started anew on the code of step 40 (`review.md`) |
| next | 23: Evaluation with AI: carrying out an evaluation plan, built as steps 29 to 36 | plan agreed (2026-10-17); step 29 built on the test projects and removed in step 41.1 (`history.md`); rebuilt on OSCAL assessment results in step 41.5, then step 31 |
| later | 30: Tool "Manufacturer documentation" | designed later (user); a part of the view Evaluation lab (step 43); needed by steps 32 to 34 |
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
4. **Approval** by the user, then a commit to `test_branch`; the tested and approved state goes to `main` (`CLAUDE.md`, "Branches").

Each step lists its **decisions** (to settle in the concept), its **tasks**, and when it is **done**. Prerequisites the user has to provide (packages, permissions) are marked **needs**.

Legend: `[ ]` to do, `[x]` done.

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

## Step 47: The view `users`: the account, the session and the encryption in one place (2026-10-23)

**State: approved by the user (2026-10-23); built phase by phase (below).**

The user (2026-10-23): the login is joined with the encryption of the database (the password, Argon2id, the unlocking key, the key of the database, the open database), so every function that touches it must be in one place, where the management of the keys and the encryption and decryption can be followed easily. The view `start` is renamed `users`, and its entry holds the setup (the installation and its user), the login, the change of the password, the logout and the closing of the program, and later every other function of the account. To keep the window and the code readable, the settings are split: **the settings of the user** (the account and the encryption: a page of their own, "User", in the navigation) and **the settings of the program** (the AI connections, what the installation is: the view `settings`).

**Decisions of the user**
- [x] `start` renamed `users`; `views/users/users.rs` is the entry of everything that touches the key of the database.
- [x] The settings of the user are a page of their own in the navigation ("User"); the view `settings` keeps the tabs AI and Program.
- [x] A new function: log out and close the program (`user_menu_close`), beside "Log out" in the user menu.
- The commands keep their names `<element>_<action>`; the user menu is shown in the frame of the window (`Shell.svelte`), which places it as it places the views.

**Phases** (one commit each, pushed, every test passing; the documents with each)
- [x] 47.1 `users`: the setup and the login (from `start`), the logout (from `shell`), the change of the password (from `settings`), each commented with what happens to the key; the page `ui/src/views/users/` (the forms, the user menu, the page "User"); `settings` without the tab User; the vocabulary.
- [ ] 47.2 `user_menu_close`: the session ended (the key wiped, the database closed) and the program closed; its button in the user menu. **The cross of the window ends the session the same way** (user, 2026-10-23): "Log out", "Log out and close" and the cross go through one function, not through the end of the process alone. With it, the memory of SQLCipher (user, 2026-10-23: only the minimum of decrypted data in RAM; SQLCipher already decrypts only the pages a query reads, so the concept stands): `cipher_memory_security` on (the memory it frees wiped, its keys locked in RAM), the temporary files of SQLite in memory only and a small, explicit cache, each checked by a test; the core dumps of the process off; the requirements of the laboratory (encrypted swap, no hibernation) in `architecture.md`, section 9.
- [ ] 47.3 `architecture.md`, section 12: the way of the key in every operation of `users`.

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

**Done when** no `<file>_tests.rs` is left besides the tests of the whole program, and the test of the architecture checks that every test module is the last item of its file.

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
- [x] `web_ui/`: not reviewed; removed when the window of step 14 replaced the web interface.
- [ ] `installation/verify.rs` (`verify_installation`): lower priority, reviewed later; it is rebuilt anyway in step 13 (keyring).
- [ ] The other modules: in an order the user chooses.

**What is checked in every module**
- [ ] Agreement with the concept: no remains of earlier concepts (comments, names, code paths, documentation of files that no longer exist).
- [ ] Dead code and `allow(dead_code)` that is no longer needed; empty folders (`uart/`) and placeholder files.
- [ ] One responsibility per file; repeated code (for example the `workstation()` check in every page) moved to one place; files that grew too large split.
- [ ] Rules of `CLAUDE.md`: core without printing, errors as enums, serialization only in `tauri_ui/serializer/` (with the exception of the types of `model/oscal/`, `oscal.md`), cancellation by the caller.
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
- [x] **Paste on Wayland (found 2026-10-18; gone the same day after the user put the packages of the system in order, so a fault of the unstable system, not of the program):** on KDE Plasma (Wayland, Parrot OS, WebKitGTK 2.54.0) the program ended with SIGSEGV when the clipboard was emptied by a clipboard cleaner while it was being pasted. The fault is inside WebKitGTK (`request_targets_received_func` of GTK calls back into WebKit with an empty list of targets); not reproduced on X11, where the same refusals are handled. Should it come back, the concept proposed then: the page catches every paste (Ctrl+V, Shift+Insert, middle click, the context menu of the fields), the program reads the clipboard itself through GTK (`request_text`, an empty clipboard is `None`) and shows "Nothing was pasted: copy the text again" instead of ending.
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

Designed with or after step 23. The laboratory provides a LaTeX template; the results of an evaluation (its OSCAL assessment results) are written into it, automatically by default.

---

## Step 23: Evaluation with AI: carrying out an evaluation plan (steps 29 to 36)

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

## Step 29: Evaluation: the evaluation project, the results and the journal

Built on 2026-10-17 on the test projects and removed with them in step 41.1; its record is in `history.md`. Its decisions (the four results, the proposals kept apart, the journal that is only added to) are carried onto the OSCAL assessment results in step 41.5.

## Step 30: Tool "Manufacturer documentation" (later)

Designed later (user, 2026-10-17); a part of the view Evaluation lab since step 43 (user, 2026-10-23), whose placeholder (`ui/src/views/evaluation_lab/`) names it. The documents of the manufacturer (Security Target, guidance, design) are loaded and analysed in a way proper to them; in OSCAL they become the system security plan and the component definition (`oscal.md`, section 1), which an assessment plan imports and an evaluation reads. Steps 32 to 34 need it.

## Step 31: The window of the evaluation

- [ ] The list of evaluations; a new evaluation from an evaluation plan (an assessment plan of OSCAL).
- [ ] Plan → one requirement: what it needs, its fragments, assessments, findings; the progress of the whole evaluation.

## Step 32: Inputs provided

- [ ] The evaluator marks what a requirement needs as provided, open or not applicable: by the documentation of step 30, an evidence file (with SHA-256), or a note; in OSCAL the subjects and the relevant evidence of the results. A need *provided by* another requirement is met when that requirement is.
- [ ] A requirement is ready to be assessed when all it needs is provided.

## Step 33: Phase 1: the AI finds the fragments of a requirement

- [ ] For the chosen requirement the AI reads the documentation of its inputs part by part and proposes fragments: a quote and a separate AI note.
- [ ] Every quote checked on its page; the fragment shown highlighted in the viewer; accepted, corrected or rejected by the evaluator; fragments added by hand.

## Step 34: Phase 2: the assessment of a requirement

- [ ] The requirement (its control in the catalog and its quotes from the standard, P5) and **only its accepted fragments** go to the model; it proposes the four results as text (findings; analysis; assessment and verdict proposition; vulnerabilities and notes) and names what is missing.
- [ ] The proposals are marked AI and become results only when the evaluator accepts them (or writes their own); every run is recorded in the journal. Where something is missing, phase 1 can be repeated for this requirement.

## Step 35: Activities, tests and findings

- [ ] A test recorded as an observation with the method *test* (steps, tools and configuration as assessment assets of the plan, date, who); findings. The AI may propose a test plan as text; **no command reaches a device** (P2); guided execution on a device comes later.

## Step 36: Outputs

- [ ] The report assembled from steps 32 to 35 (the findings, the verdicts and their rationale, the vulnerabilities and notes from the accepted results of the requirements, user 2026-10-17); the AI drafts the text by the instructions of the laboratory, the evaluator corrects and approves it. Shown in the window until step 28 (LaTeX).

---

## After this list

Not planned in detail yet; to be designed once the steps above are done:

- AI on the data of the devices (any provider and local models are configurable since step 15).
- Backup of the databases with its own encryption, independent of the TPM; also the way to move to a new computer (`architecture.md`, section 5.5).
- Git of the laboratory over a secure connection, with checks of its configuration (`architecture.md`, section 6).
- Filling the LaTeX report templates of the laboratory and committing the result.
- Git panel in the window.
- The other functions of version 0.2 (`v0.2_functionality.md`): UART read and write with the confirmation of every command (P2), Bluetooth.
- A build variant without the code of the master (`architecture.md`, section 3.2).
- Beyond the evaluation (steps 41 and 31 to 36): guided execution of functional tests on the device, their results as observations of the OSCAL assessment results. (The documentation of the product is step 30.)
