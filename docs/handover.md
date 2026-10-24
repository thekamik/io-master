# Handover for a new session

Read this file, `CLAUDE.md`, `review.md`, `docs/code_structure.md`, `docs/architecture.md` and `docs/todo.md` before doing anything. The user talks in Polish; code comments and documents are in English. A high-level overview of what the program does: `docs/presentation.md`.

## Where the project is (2026-10-24)

- Repository `/home/kamil/Projects/io-master`, crate `io-master/` (Rust 2024, Tauri 2 + Svelte 5), branches **`main`** (the tested state) and **`test_branch`** (the work); no other branches and no version numbers yet.
- **Steps 0 to 52 are built or settled** (`docs/history.md`): the layers and their tests (step 40), the views and the components (42 to 47), the framework for the evaluation with AI without devices, master, TPM or keyring (48), the recovery code (49), the Converter (50), the master projects of the Evaluation plan with the schemes of tasks, the proposals of the AI and the work packages (51), the Evaluation lab with the evidence, the fragments, the assessment, the outputs and the report (52).
- **The documents were read through on 2026-10-24** (step 48.2): the catalogue of the functions of version 0.2 and the console function of the removal of the installation were removed; `todo.md` keeps only the open steps (10, 12, 53 to 56 and reminders).
- **The state of the branches (2026-10-24):** `main` and `test_branch` hold the same state after the merge of steps 48 to 52 and the documents; the user tests the functions of the AI by hand.
- **The data of the tests is a temporary resource until version 1**: a change of a schema may need a fresh database (`~/.local/share/io-master` removed).
- **The review starts anew** on the code of step 40 (`review.md`): no approval moved with the code.

## What comes next

The user chooses (`docs/todo.md`). The candidates:
1. **Step 12, the user's review**, file by file: Claude answers precisely (the user knows C and is new to Rust: explain Rust through C analogies), points out errors, and changes code only on request; it does not push for decisions.
2. **Step 53, zero warnings**: the unused code of earlier steps, each a decision of the user (use it or remove it); eslint on the page.
3. The reminders of the Evaluation lab (a viewer of the evidence, other forms of evidence), steps 54 (removal of the installation) and 55 (backups).

## Rules that are easy to forget

- Concept first, approved by the user, then code; small steps, each with tests (in the file they test, its last item `#[cfg(test)] mod tests { ... }`); `cargo fmt` before a commit.
- An approved step in short, mechanical phases runs to its end without asking before each phase; commit and push each; stop only for a decision of the user (a concept, a change of a rule) or for `main`.
- Commits go to `test_branch` and are pushed at once; `main` only after the user's manual tests, asked each time.
- A fake AI for the window tests: a small Python HTTP server on 127.0.0.1:11434 that answers in the chat-completions stream format, its answer chosen by the instructions in the question (a JSON object for the tasks that read JSON, a plain text for the chat and the outputs); the default new connection points there.
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
cargo clippy --all-targets  # must be clean (rule 10 of CLAUDE.md); the 15 known warnings of unused code: step 53
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
- **The scratchpad is emptied when a session starts again**: the helpers are written anew when they are missing. The ones used on 2026-10-24: `x.py` (XTEST through ctypes, no xdotool: `click X Y`, `type TEXT`, `key NAME`, `scroll X Y N`, `drag X1 Y1 X2 Y2`, `paste` into a GTK dialog), `tools/start.sh` (a private display, the fake AI and the program on a fresh data folder), `tools/restart.sh` (the program again on the same data), `tools/stop.sh`, `tools/fakeai.py` (the fake AI).
- The window of a fresh installation (1280x860 at 0,0): the setup form (name at 640,406, passwords at 640,486 and 640,566, Install at 640,627), then the recovery code (the group to type at 640,564, "go on" at 640,624); the login (name 640,430, password 640,510, Log in 640,570; "I do not remember my password" at 610,608); the navigation at x 110 (Chat 108, Converter 151, Evaluation plan 194, Evaluation lab 237, User 280, Settings 323); the user menu at the foot (log out 162,826, log out and close 207,826).
- The GTK dialogs: a file to open is pasted (Ctrl+L, Ctrl+V, Enter); a file to save is typed into the field Name as a whole path; a folder to choose is picked from the list (its typed path does not enable the button "Open").
- The mock runtime of Tauri does not implement the exit of the program: `user_menu_close` is tested in the window only.
- The GTK file dialog is not visible in a root screenshot (no compositor): capture its own window (`xwininfo -display :99 -root -tree`, then `import -window <id>`), and enter a path by pasting it (`xclip -display :99 -selection clipboard`, then Ctrl+L, Ctrl+V, Enter).
- Stop processes with `pkill -x Xvfb`, `pkill -x io-master`; for Python servers use `kill $(pgrep -f '[n]ame.py')` (a plain `pkill -f` kills the shell itself).

## The environment

- **Debian 13 (trixie) only** (user, 2026-10-24): the target system and the only one the program is tested on; the earlier tests on Arch Linux are dropped. XFCE on X11.
- A full build of Tauri is heavy for 8 GB of RAM: if the linker runs out of memory, `cargo build -j 2`.
- Shell of the user is fish; tools run in bash.
