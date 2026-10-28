# Handover for a new session

Read this file, `CLAUDE.md`, `review.md`, `docs/code_structure.md`, `docs/architecture.md`, `docs/data_model.md` and `docs/todo.md` before doing anything. The user talks in Polish; code comments and documents are in English. A high-level overview of what the program does: `docs/presentation.md`.

## Where the project is (2026-10-28)

- Repository `/home/kamil/Projects/io-master`, crate `io-master/` (Rust 2024, Tauri 2 + Svelte 5), branches **`main`** (the earlier program, the reference) and **`test_branch`** (the rebuild). **Nothing goes to `main` until the user revokes it** (2026-10-28).
- **The rebuild (user, 2026-10-28):** the users of the program found it weak in its function, so it is rebuilt from its data up. Done: the new models (`model/`, agreed field by field with the user and frozen, `docs/data_model.md`), their database (`db/`: create, read, update, no delete, two new files `users/<id>.v2.db` and `system.v2.db`, the copy of a plan into an evaluation checked by Kahn's algorithm), the earlier code removed (it is in `main`). The window shows the first start and the login (tried on a private display); 86 tests, clippy and svelte-check clean.
- **The old databases are dropped**: nothing is moved from them; the data of the tests is temporary until version 1.

## What comes next

The parts come back one by one, rewritten on the new models (each a concept approved by the user first):
1. **The functions of the making of a plan** in a folder of their own above `db/` (its name to be agreed): the order of the work, `cross_reference` and `blocked` set after every change of the dependencies; and the check of an OSCAL document against the schema of NIST in a file of its own (its name to be agreed).
2. **The login and the session: built (2026-10-28)**, the first start, the recovery code shown once, the login and the logout (`account/`, `session/`, `views/program_start/`). Still to come: the login with the recovery code, the change of the password, a new recovery code.
3. **The AI layer** on the new models: the integration, the agent, the RAG, the journal (the HTTP and TLS client of `main` reused).
4. **The views**, one by one, each with its page: the evaluation plan, the evaluation, the knowledge, the settings, the chat.

## Rules that are easy to forget

- Concept first, approved by the user, then code; small steps, each with tests (in the file they test, its last item `#[cfg(test)] mod tests { ... }`); `cargo fmt` before a commit.
- An approved step in short, mechanical phases runs to its end without asking before each phase; commit and push each; stop only for a decision of the user (a concept, a change of a rule) or for `main`.
- Commits go to `test_branch` and are pushed at once; `main` only after the user's manual tests, asked each time.
- A fake AI for the window tests: a small Python HTTP server on 127.0.0.1:11434 that answers in the chat-completions stream format, its answer chosen by the instructions in the question (a JSON object for the tasks that read JSON, a plain text for the chat and the outputs); the default new connection points there.
- **A new file of the program is declared in the map of `main.rs`**, in the group of its folder; there is no `mod.rs` and no file that only declares modules. When the views come back (the rules of `main`): **a new command** goes into the entry of its view (`views/<view>/<view>.rs`), with the comment: what it does, where the page calls it, what it calls; its function on the page into `ui/src/lib/api/<view>.ts`, named as the command.
- **Comments at the functions** (what, called from, calls); descriptions of the architecture only in `docs/` (Markdown with schemes).
- In the earlier program (to keep when the login comes back): everything that touches the key of the database stayed in `views/users/users.rs`; every end of the program went through `AppState::log_out` (`main.rs`, `RunEvent::Exit`).
- The direction of the dependencies is checked by `src/architecture_tests.rs`; a new use against it is a finding, or an exception written there with its reason.
- When the commands come back, each is listed in four places (the entry of its view or component, `routes.rs`, `build.rs`, `capabilities/default.json`), checked by a test (`command_lists_tests.rs` of `main`); today there is none.
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
cargo clippy --all-targets  # must be clean (rule 10 of CLAUDE.md)
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
- The window of a fresh installation (1280x860 at 0,0): the setup form (name at 640,406, passwords at 640,486 and 640,566, Install at 640,627), then the recovery code (the group to type at 640,564, "go on" at 640,624); the login (name 640,430, password 640,510, Log in 640,570; "I do not remember my password" at 610,608); the navigation at x 110 (Chat 108, Converter 151, Evaluation plan 194, Evaluation lab 237, Knowledge (RAG) 280, User 323, Settings 366); the user menu at the foot (log out 162,826, log out and close 207,826).
- The GTK dialogs: a file to open is pasted (Ctrl+L, Ctrl+V, Enter); a file to save is typed into the field Name as a whole path; a folder to choose is picked from the list (its typed path does not enable the button "Open").
- The mock runtime of Tauri does not implement the exit of the program: `user_menu_close` is tested in the window only.
- The GTK file dialog is not visible in a root screenshot (no compositor): capture its own window (`xwininfo -display :99 -root -tree`, then `import -window <id>`), and enter a path by pasting it (`xclip -display :99 -selection clipboard`, then Ctrl+L, Ctrl+V, Enter).
- Stop processes with `pkill -x Xvfb`, `pkill -x io-master`; for Python servers use `kill $(pgrep -f '[n]ame.py')` (a plain `pkill -f` kills the shell itself).

## The environment

- **Debian 13 (trixie) only** (user, 2026-10-24): the target system and the only one the program is tested on; the earlier tests on Arch Linux are dropped. XFCE on X11.
- A full build of Tauri is heavy for 8 GB of RAM: if the linker runs out of memory, `cargo build -j 2`.
- Shell of the user is fish; tools run in bash.
