# Handover for a new session

Read this file, `CLAUDE.md`, `review.md`, `docs/code_structure.md`, `docs/architecture.md`, `docs/data_model.md` and `docs/todo.md` before doing anything. The user talks in Polish; code comments and documents are in English. A high-level overview of what the program does: `docs/presentation.md`.

## Where the project is (2026-10-29)

- Repository `/home/kamil/Projects/io-master`, crate `io-master/` (Rust 2024, Tauri 2 + Svelte 5), branches **`main`** (the earlier program, the reference) and **`test_branch`** (the rebuild). **Nothing goes to `main` until the user revokes it** (2026-10-28).
- **The rebuild (user, 2026-10-28):** the users of the program found it weak in its function, so it is rebuilt from its data up. Done: the new models (`model/`, agreed field by field with the user and frozen, `docs/data_model.md`), their database (`db/`: create, read, update, no delete, two new files `users/<id>.v2.db` and `system.v2.db`, the copy of a plan into an evaluation, `delete_work_unit` only in planning), the earlier code removed (it is in `main`). The window shows the first start and the login (tried on a private display). The panel of the plan is built (`panels/logic/evaluation_plan/`, `panels/views/evaluation_plan/`, its page, 2026-10-29): tried on a private display with the catalog of the CEM (813 work units added at once, a dependency, a proof, a cross reference that blocks the plan). Seen in that trial: on the software rendering of Xvfb the page is slow with 813 rows (the keys typed come late); the list of the dependencies shows only what a filter finds; a list of the work units by pages may be needed on a real plan. **The way to the AI and the panel "AI settings" are built (2026-10-29):** `ai/` (the only way to a service: prepare, send, record into the journal; the protocols of OpenAI and Anthropic; TLS 1.3, the key only in its header), `panels/logic/settings/`, `panels/views/settings/` and `ui/src/views/settings/` (integrations, their keys, the check of the connection, the instructions of the tasks, the journal), tried on a private display against a fake service (an integration refused for its address, one added and active, a key stored and sent only in its header, the check answered and failed, both in the journal, the instructions changed and kept). 129 tests; clippy clean but for the warnings `dead_code` of the parts not joined yet (user, 2026-10-29); svelte-check clean.
- **The old databases are dropped**: nothing is moved from them; the data of the tests is temporary until version 1.

## What comes next

**Built on 2026-10-29 as agreed with the user:** the panel of the plan follows the model of the data, where the work units belong to their plan: two tabs, **Standards** (the list, the import) and **Plans** (the list, a new plan opened at once); **the editor of a plan** ("← Plans": its data and its work units by pages of 50, a filter, "Add from a standard" folded); **the editor of a work unit** ("← the plan" keeping the page and the filter, "Previous / Next", the requirement, the dependencies, "Depended on by", the documents, the guidance, the proofs). Tried on a private display with the catalog of the CEM; to be tried by the user.

The parts come back one by one, rewritten on the new models (each a concept approved by the user first):
1. **The panel of the plan: built (2026-10-29)**, the import of a standard checked against the schema of NIST, the plans, the work units one or all, their parameters and proofs, the order of the work and the cross references, its commands and its page; to be tried by the user. Later (user, 2026-10-29): an agent that reads the standard and proposes the parameters of the work units and of the plan instead of the planner.
2. **The login and the session: built (2026-10-28)**, the first start, the recovery code shown once, the login and the logout (`panels/logic/program_start/`, `session/`, `panels/views/program_start/`). Still to come: the login with the recovery code (in `program_start`, user 2026-10-28), the change of the password and a new recovery code.
3. **The AI, in three panels (user, 2026-10-29), in this order:**
   1. **The AI settings: built (2026-10-29)**, with the way to the AI (`ai/`, the client of `main` reused) and the journal; to be tried by the user against a real service.
   2. **The agents: built (2026-10-29)**, after the user's correction: the AI settings handle only `Integration`; the model `Instructions` is removed; every entry of the journal names its agent (a question without an agent, the check of an integration, is not kept). The panel "Agents" makes the agents (an agent is a layer of knowledge over the model of an integration: its integration, instructions, shared knowledge bases, glossary, limits, its own journal, the test of the knowledge it would give) and the knowledge bases (sources from `.txt`, `.md`, `.pdf`, never removed but replaced or renamed; the search). The editor of a work unit chooses its agent. Tried on a private display.
   3. **The chat**, to try the agents, its answer streamed into the page (a channel of Tauri) with "Stop" (the receiver of the pieces dropped).
   Then, built (2026-10-29): **"Fill with AI"**: a plan names its agent of the planner (`EvaluationPlan::planning_agent`, `None` in an evaluation); the agent goes through every work unit of the plan in its order, over what was there, in two tasks with a button each (user, 2026-10-29): "Fill parameters" (every parameter but the dependencies) and "Fill dependencies" (when every work unit is described: each question shows every other work unit in one line); a dependency that closes a cycle is written and left to the planner (its work units red, the plan blocked); each question is in the journal of the agent; "Stop" ends it after the present work unit. Tried on a private display against a fake service.
4. **The views**, one by one, each with its page: the evaluation, and the rest of the evaluation plan.

## Rules that are easy to forget

- Concept first, approved by the user, then code; small steps, each with tests (in the file they test, its last item `#[cfg(test)] mod tests { ... }`); `cargo fmt` before a commit.
- An approved step in short, mechanical phases runs to its end without asking before each phase; commit and push each; stop only for a decision of the user (a concept, a change of a rule) or for `main`.
- Commits go to `test_branch` and are pushed at once; `main` only after the user's manual tests, asked each time.
- A fake AI for the window tests: a small Python HTTP server on 127.0.0.1 (2026-10-29: port 18080, `fake_ai.py` in the scratchpad) that answers in the chat-completions stream format and writes every request it gets (head and body) into a file, to check that the key goes only into its header. The tests of Rust use `test_support/ai_service.rs` (real TLS, both protocols).
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
- The window of 2026-10-29 (1280x860 at 0,0): the side bar "Evaluation plan" at 100,108, "AI settings" at 100,152; the tabs of the AI settings at y 128 (Integrations 348, Instructions 478, Journal 588).
- The window of a fresh installation (1280x860 at 0,0): the setup form (name at 640,406, passwords at 640,486 and 640,566, Install at 640,627), then the recovery code (the group to type at 640,564, "go on" at 640,624); the login (name 640,430, password 640,510, Log in 640,570; "I do not remember my password" at 610,608); the navigation at x 110 (Chat 108, Converter 151, Evaluation plan 194, Evaluation lab 237, Knowledge (RAG) 280, User 323, Settings 366); the user menu at the foot (log out 162,826, log out and close 207,826).
- The GTK dialogs: a file to open is pasted (Ctrl+L, Ctrl+V, Enter); a file to save is typed into the field Name as a whole path; a folder to choose is picked from the list (its typed path does not enable the button "Open").
- The mock runtime of Tauri does not implement the exit of the program: `user_menu_close` is tested in the window only.
- The GTK file dialog is not visible in a root screenshot (no compositor): capture its own window (`xwininfo -display :99 -root -tree`, then `import -window <id>`), and enter a path by pasting it (`xclip -display :99 -selection clipboard`, then Ctrl+L, Ctrl+V, Enter).
- Stop processes with `pkill -x Xvfb`, `pkill -x io-master`; for Python servers use `kill $(pgrep -f '[n]ame.py')` (a plain `pkill -f` kills the shell itself).

## The environment

- **Debian 13 (trixie) only** (user, 2026-10-24): the target system and the only one the program is tested on; the earlier tests on Arch Linux are dropped. XFCE on X11.
- A full build of Tauri is heavy for 8 GB of RAM: if the linker runs out of memory, `cargo build -j 2`.
- Shell of the user is fish; tools run in bash.
