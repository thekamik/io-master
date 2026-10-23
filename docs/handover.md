# Handover for a new session

Read this file, `CLAUDE.md`, `review.md`, `docs/code_structure.md`, `docs/architecture.md` and the order table of `docs/todo.md` before doing anything. The user talks in Polish; code comments and documents are in English.

## Where the project is (2026-10-23)

- Repository `/home/kamil/Projects/io-master`, crate `io-master/` (Rust 2024, Tauri 2 + Svelte 5), branches **`main`** (the tested state) and **`test_branch`** (the work); no other branches and no version numbers yet (`CLAUDE.md`, "Branches"; the earlier branches are named in `docs/history.md`).
- **Step 40 (the cleanup of the architecture) is built**: the layers `model/`, `db/`, `ai/`, `hardware_tests/`, `security/`, `tauri_ui/` (now `app/`), their rules and the tests that check them (`docs/code_structure.md`).
- **Step 41 (NIST OSCAL, `docs/oscal.md`) is being built**, part by part (`docs/todo.md`, step 41): 41.1 to 41.3 are done. The evaluation plan works on documents of OSCAL 1.2.3 (import, export, deletion, every write checked against the schemas of NIST); catalogs are made in the program from the PDFs of a standard, with the AI proposing groups and controls and the evaluator accepting, correcting or rejecting them in the editor of a catalog (`src/views/evaluation_plan/catalog_editor/`, page `ui/src/views/evaluation_plan/catalog_editor/`). The quotes and the proposals are the evaluation support (`db/evaluation_support/`), linked to OSCAL by UUID and id.
- **Step 42 (the views) is built** (2026-10-21): every function that joins the program with the page is a command of an element in `src/views/<view>/<element>/`; `business/` and the commands of `tauri_ui/` are removed (`docs/code_structure.md`, section 2.9).
- **Step 43 (one vocabulary, views and components) is built** (2026-10-23): the names of the code follow the vocabulary of `docs/architecture.md`, section 10 (`start`, `shell`, `chat`, `evaluation_plan` with its editor of a catalog, `evaluation_lab`, `devices`, `settings`); `app/` (was `tauri_ui/`) holds what every view uses; the list of the commands is `src/routes.rs`; the elements used more than once are in `components/` (`pdf_viewer`, `part_rows`). The test of the architecture checks that a view never uses another view and a component never a view; a test checks that the ids of the pages agree with `Shell.svelte`.
- **Step 44 (the tests in the file they test) is built** (2026-10-23): every unit test is the last item of the file whose functions it tests (`#[cfg(test)] mod tests { ... }`); the tests of a whole area of CRUD were split by the function each checks; `src/architecture_tests.rs` checks that the tests are the last item and refuses a new `*_tests.rs`. Next: step 45 (the tree of the modules in `main.rs`), then step 46 (the entry of a view), both in `docs/todo.md`.
- **The review starts anew** on the code of step 40 (`review.md`): no approval moved with the code.

## What comes next

1. **41.4: profile and assessment plan** (the evaluation plan proper), shown as a graph; the CEM and the Protection Profiles as documents of OSCAL that a plan can take. Concept details first where `docs/oscal.md` leaves them open (section 4: the import of an SSP).
2. **41.5: assessment results** (the evaluation), then steps 31 to 36 on them.
3. The user's review (step 12), file by file: Claude answers precisely (the user knows C and is new to Rust: explain Rust through C analogies), points out errors, and changes code only on request; it does not push for decisions.
4. Later steps in the order table of `docs/todo.md`.

## Rules that are easy to forget

- Concept first, approved by the user, then code; small steps, each with tests (in the file they test, its last item `#[cfg(test)] mod tests { ... }`); `cargo fmt` before a commit.
- The direction of the dependencies is checked by `src/architecture_tests.rs`; a new use against it is a finding, or an exception written there with its reason.
- Every command of the window is listed in four places (the `commands.rs` of its element in `views/`, `routes.rs`, `build.rs`, `capabilities/default.json`); a test checks them.
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

The packages of the system for running, building and testing, on Debian 13 and on Arch Linux: `README.md`, section "Requirements". Rust comes from rustup (stable, 1.99.1 on 2026-10-21), never from the packages `rustc`/`cargo` of the system beside it: two compilers in `PATH` make clippy fail with E0514 on a build made by the other one; check with `rustc --version` and `which cargo` (`~/.cargo/bin/cargo`); after a change of compiler, `cargo clean` once.

Window test on a private display (helpers in the scratchpad of the session; recreate them if missing):

```text
Xvfb :99 -screen 0 1600x1000x24 -nolisten tcp &
env -u WAYLAND_DISPLAY DISPLAY=:99 GDK_BACKEND=x11 XDG_DATA_HOME=<scratch>/data \
    WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1 \
    ./target/debug/io-master &
```

- Drive it with XTEST from Python (`ctypes`, libXtst): click, type, key, drag, scroll; screenshots with `import -display :99 -window root`.
- The GTK file dialog is not visible in a root screenshot (no compositor): capture its own window (`xwininfo -display :99 -root -tree`, then `import -window <id>`), and enter a path by pasting it (`xclip -display :99 -selection clipboard`, then Ctrl+L, Ctrl+V, Enter).
- A fake AI model: a small Python HTTP server on 127.0.0.1:11434 answering in the chat-completions stream format; the preset "Local Gemma" points there.
- Stop processes with `pkill -x Xvfb`, `pkill -x io-master`; for Python servers use `kill $(pgrep -f '[n]ame.py')` (a plain `pkill -f` kills the shell itself).

## The environment

- **Why these machines (user, 2026-10-21).** The T480 stands for the virtual machines of the laboratories, which usually get few resources: one may be as strong as the T480 or weaker, so the program must work well on it. The T14 Gen 7 (new, 2026) with Arch and the newest packages checks that the code is future proof for the coming versions of Debian; the user starts to work on it once the code is cleaned up. **Later: a Raspberry Pi 5** (ARM64, no TPM) as a third machine for tests.
- Since 2026-10-21 the user tests on two machines on purpose: an old one with the stable target system and a new one with a rolling system (newer libraries, WebKitGTK and kernel first), Intel and AMD, X11 and Wayland, XFCE and GNOME, so that a fault of one environment is told from a fault of the program. A fault seen on one machine only is checked on the other before it is fixed. The packages of both systems: `README.md`, "Requirements" (Debian: the plain repositories, no backports).

  | Machine | System | Desktop | Notes |
  |---|---|---|---|
  | ThinkPad T480, Intel Core i5, 8 GB RAM | **Debian 13 (trixie), the target system** | XFCE (X11) | the keyring needs `libpam-gnome-keyring` and LightDM (`architecture.md`, section 9); a full build of Tauri is heavy for 8 GB: if the linker runs out of memory, `cargo build -j 2` |
  | ThinkPad T14 Gen 7, AMD Ryzen 5, 16 GB RAM | Arch Linux | GNOME (Wayland) | GNOME Keyring unlocked at the login by GNOME itself; the TPM of an AMD processor is usually a firmware TPM (fTPM), which some BIOS updates clear (`architecture.md`, section 5.1) | Before 2026-10-21 the work was on Parrot OS (KDE on Wayland); notes in `history.md` and `todo.md` that name Parrot OS refer to it.
- Shell of the user is fish; tools run in bash.
