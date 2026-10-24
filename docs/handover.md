# Handover for a new session

Read this file, `CLAUDE.md`, `review.md`, `docs/code_structure.md`, `docs/architecture.md` and the order table of `docs/todo.md` before doing anything. The user talks in Polish; code comments and documents are in English.

## Where the project is (2026-10-23)

- Repository `/home/kamil/Projects/io-master`, crate `io-master/` (Rust 2024, Tauri 2 + Svelte 5), branches **`main`** (the tested state) and **`test_branch`** (the work); no other branches and no version numbers yet (`CLAUDE.md`, "Branches"; the earlier branches are named in `docs/history.md`).
- **Step 40 (the cleanup of the architecture) is built**: the layers `model/`, `db/`, `ai/`, `hardware_tests/` (removed in step 48), `security/`, `tauri_ui/` (now `app/`), their rules and the tests that check them (`docs/code_structure.md`).
- **Step 41 (NIST OSCAL, `docs/oscal.md`) is being built**, part by part (`docs/todo.md`, step 41): 41.1 to 41.3 are done. The evaluation plan works on documents of OSCAL 1.2.3 (import, export, deletion, every write checked against the schemas of NIST); catalogs are made in the program from the PDFs of a standard, with the AI proposing groups and controls and the evaluator accepting, correcting or rejecting them in the editor of a catalog (`src/views/evaluation_plan/catalog_editor/`, page `ui/src/views/evaluation_plan/catalog_editor/`). The quotes and the proposals are the evaluation support (`db/evaluation_support/`), linked to OSCAL by UUID and id.
- **Step 42 (the views) is built** (2026-10-21): every function that joins the program with the page is a command of an element in `src/views/<view>/<element>/`; `business/` and the commands of `tauri_ui/` are removed (`docs/code_structure.md`, section 2.9).
- **Step 43 (one vocabulary, views and components) is built** (2026-10-23): the names of the code follow the vocabulary of `docs/architecture.md`, section 10 (`users` (was `start`, step 47), `shell`, `chat`, `evaluation_plan` with its editor of a catalog, `evaluation_lab`, `devices` (removed in step 48), `settings`); `app/` (was `tauri_ui/`) holds what every view uses; the list of the commands is `src/routes.rs`; the elements used more than once are in `components/` (`pdf_viewer`, `part_rows`). The test of the architecture checks that a view never uses another view and a component never a view; a test checks that the ids of the pages agree with `Shell.svelte`.
- **Step 44 (the tests in the file they test) is built** (2026-10-23): every unit test is the last item of the file whose functions it tests (`#[cfg(test)] mod tests { ... }`); the tests of a whole area of CRUD were split by the function each checks; `src/architecture_tests.rs` checks that the tests are the last item and refuses a new `*_tests.rs`. 
- **Step 45 (the tree of the modules in `main.rs`) is built** (2026-10-23): `main.rs` is the map of the program, every folder of the layers and its modules in the order of the layers from the bottom up, each group commented; the files that only declared modules are removed. 
- **Step 46 (the entry of a view) is built** (2026-10-23): every view and every component with commands has its entry, the file of its name (`views/chat/chat.rs`), with all its commands, those called when it opens first, each commented with what it does, where the page calls it and what it calls; the folders of the elements hold their data and logic; no `mod.rs` is left, every module is in the map of `main.rs`; on the page one file of commands per view (`ui/src/lib/api/chat.ts`), named as the commands.
- **Step 47 (the view `users`) is built** (2026-10-23): everything that touches the key of the database is in `views/users/users.rs` (the setup, the login, the password, the logout, the closing); the settings are split into the page User (the account) and Settings (the program); every end of the program ends the session through `AppState::log_out`; SQLCipher wipes and locks its memory, the process forbids its memory dumps. The way of the key: `docs/architecture.md`, section 12.
- **The entry is the only file of the program in the folder of a view** (2026-10-23): what the elements of a view share belongs to the element that owns it (`code_structure.md`, section 2.9).
- **Step 48 (the evaluation with AI as the only aim) is built in its code** (2026-10-24, `docs/todo.md`): no devices (48.1), no master, no TPM, no passkeys, no keyring (48.3), no git; the login stays the password (the login with OpenPGP was considered and withdrawn); backups optionally with OpenPGP later. 48.2 (the documents) and 48.3 (the master and the TPM removed, concept first) are next.
- **One theme only, dark** (2026-10-23): `ui/src/styles/01-tokens.css`, no light theme, it does not follow the system.
- **The state of the branches (2026-10-23):** `main` holds steps 42 to 46 and the rule of the entry (merge `4d514e3`); `test_branch` holds besides step 47 and the dark theme, **waiting for the user's manual tests** before the merge into `main`.
- **The review starts anew** on the code of step 40 (`review.md`): no approval moved with the code.

## What comes next

0. **The user tests the window by hand** (steps 47, 48.1 and the dark theme on `test_branch`); after that, ask for the merge into `main`.
0a. **Step 48**: 48.1 and 48.3 built (no devices, no master, no TPM, no keyring; the password stays); 48.2, a read of the documents for the framework, left. **Step 50.1** (the view Converter: the PDFs, the catalogs and their editor moved out of the Evaluation plan) built 2026-10-24; 50.2 (the UX of the converter) and the new role of the Evaluation plan (the graph, the flow of data of the evaluation) next, concepts first. **Step 49** (the recovery code) built 2026-10-24: shown after the setup, after every change or recovery of the password, and on demand on the page User (with the password).
1. **The rule of the comments for the rest of the code** (`CLAUDE.md`, "Where the comments are": what a function does, where it is called from, what it calls; the architecture in `docs/`): so far only the entries of the views and components follow it; the other layers get it when a file is rebuilt or reviewed, or in a step of its own if the user asks for it (not planned yet).
2. **41.4: profile and assessment plan** (the evaluation plan proper), shown as a graph; the CEM and the Protection Profiles as documents of OSCAL that a plan can take. Concept details first where `docs/oscal.md` leaves them open (section 4: the import of an SSP).
3. **41.5: assessment results** (the evaluation), then steps 31 to 36 on them.
4. The user's review (step 12), file by file: Claude answers precisely (the user knows C and is new to Rust: explain Rust through C analogies), points out errors, and changes code only on request; it does not push for decisions.
5. Later steps in the order table of `docs/todo.md`.

## Rules that are easy to forget

- Concept first, approved by the user, then code; small steps, each with tests (in the file they test, its last item `#[cfg(test)] mod tests { ... }`); `cargo fmt` before a commit.
- An approved step in short, mechanical phases runs to its end without asking before each phase; commit and push each; stop only for a decision of the user (a concept, a change of a rule) or for `main`.
- Commits go to `test_branch` and are pushed at once; `main` only after the user's manual tests, asked each time.
- **A new file of the program is declared in the map of `main.rs`**, in the group of its folder; there is no `mod.rs` and no file that only declares modules. **A new command** goes into the entry of its view (`views/<view>/<view>.rs`), with the comment: what it does, where the page calls it, what it calls; its function on the page into `ui/src/lib/api/<view>.ts`, named as the command.
- **Comments at the functions** (what, called from, calls); descriptions of the architecture only in `docs/` (Markdown with schemes).
- Everything that touches the key of the database stays in `views/users/users.rs`; every end of the program goes through `AppState::log_out` (`main.rs`, `RunEvent::Exit`).
- The direction of the dependencies is checked by `src/architecture_tests.rs`; a new use against it is a finding, or an exception written there with its reason.
- Every command of the window is listed in four places (the entry of its view or component, `routes.rs`, `build.rs`, `capabilities/default.json`); a test checks them.
- Serialization only in the `view.rs` of the elements of `src/views/` and in `src/app/serializer/` (the error of a command); the core types carry no serde attributes.
- Secrets only in the encrypted database of the evaluator; never shown back, never printed; the page holds no secret.
- Every connection leaving the computer: TLS 1.3, verified certificate. Plain HTTP only to 127.0.0.1/::1 (AI connections).
- `tauri.conf.json` keeps `freezePrototype: true`; `vite.config.ts` patches `d3-color` for it (the window does not start otherwise).
- Never capture the user's screen. Window tests run only on a private Xvfb display, see below.
- Before deleting or overwriting, look at the target. Dependencies: few, well known, versions pinned exactly, released at least two weeks before.
- Commits end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## How to build and test

```text
cd io-master
npm ci                      # once
npm run check               # svelte-check, must be clean
npm run build               # copies pdf.js data, builds ui/dist (needed before cargo build)
cargo fmt --check
cargo build && cargo test   # also the test of the architecture and of the command lists
cargo clippy --all-targets  # must be clean (rule 10 of CLAUDE.md); the known warnings of unused code still to be removed
```

The packages of the system for running, building and testing, on Debian 13: `README.md`, section "Requirements". Rust comes from rustup (stable, 1.99.1 on 2026-10-21), never from the packages `rustc`/`cargo` of the system beside it: two compilers in `PATH` make clippy fail with E0514 on a build made by the other one; check with `rustc --version` and `which cargo` (`~/.cargo/bin/cargo`); after a change of compiler, `cargo clean` once.

Window test on a private display (helpers in the scratchpad of the session; recreate them if missing):

```text
Xvfb :99 -screen 0 1600x1000x24 -nolisten tcp &
env -u WAYLAND_DISPLAY DISPLAY=:99 GDK_BACKEND=x11 XDG_DATA_HOME=<scratch>/data \
    WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1 \
    ./target/debug/io-master &
```

- Drive it with XTEST from Python (`ctypes`, libXtst): click, type, key, drag, scroll; screenshots with `import -display :99 -window root`. Name the modifier keys by their keysyms (`Control_L`, `Shift_L`): a name like `ctrl` is no keysym, and its keys go into the field as letters.
- **The scratchpad is emptied when a session starts again**: the helpers were lost once and written anew. The ones used on 2026-10-23 and 24: `x.py` (XTEST through ctypes, no xdotool: `click X Y`, `type TEXT`, `key NAME`), `mkpdf.py` (a small PDF of two pages, as `test_support::standards::pdf` makes it), `closewin.py` (sends `WM_DELETE_WINDOW` to the window, what the cross of a window manager does), `run.sh` (starts the program and writes its exit code to a file, to see that it ends with 0).
- The window of a fresh installation (1280x860 at 0,0): the setup form (name at 640,406, passwords at 640,486 and 640,566, Install at 640,627), then the recovery code (the group to type at 640,564, "go on" at 640,624); the login ("I do not remember my password" at 610,608); the navigation (Chat 108, Converter 151, Evaluation plan 194, Evaluation lab 237, User 280, Settings 323, at x 70); the navigation at x 83 (Chat 178, Evaluation plan 221, Evaluation lab 264, User 307, Settings 351: measured before step 48 removed Devices, check again); the user menu at the foot (log out 162,826, log out and close 207,826).
- The mock runtime of Tauri does not implement the exit of the program: `user_menu_close` is tested in the window only.
- The GTK file dialog is not visible in a root screenshot (no compositor): capture its own window (`xwininfo -display :99 -root -tree`, then `import -window <id>`), and enter a path by pasting it (`xclip -display :99 -selection clipboard`, then Ctrl+L, Ctrl+V, Enter).
- A fake AI model: a small Python HTTP server on 127.0.0.1:11434 answering in the chat-completions stream format; the preset "Local Gemma" points there.
- Stop processes with `pkill -x Xvfb`, `pkill -x io-master`; for Python servers use `kill $(pgrep -f '[n]ame.py')` (a plain `pkill -f` kills the shell itself).

## The environment

- **Debian 13 (trixie) only** (user, 2026-10-24): the target system and the only one the program is tested on; the earlier tests on Arch Linux are dropped. XFCE on X11.
- A full build of Tauri is heavy for 8 GB of RAM: if the linker runs out of memory, `cargo build -j 2`.
- Shell of the user is fish; tools run in bash.
