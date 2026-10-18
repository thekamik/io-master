# Handover for a new session

Read this file, `CLAUDE.md`, `review.md`, `docs/code_structure.md`, `docs/architecture.md` and the order table of `docs/todo.md` before doing anything. The user talks in Polish; code comments and documents are in English.

## Where the project is (2026-10-18)

- Repository `/home/kamil/Projects/io-master`, crate `io-master/` (Rust 2024, Tauri 2 + Svelte 5), branch **`v0.5`**. `v0.4` keeps the state before step 40 and is not changed; `v0.2`/`V0.2` are never changed; `v0.3` keeps the old web interface.
- **Step 40 (the cleanup of the architecture) is built** on `v0.5`: the layers `model/`, `db/`, `ai/`, `hardware_tests/`, `security/`, `business/`, `tauri_ui/`, their rules and the tests that check them (`docs/code_structure.md`).
- **Step 41 (NIST OSCAL, `docs/oscal.md`) is being built**, part by part (`docs/todo.md`, step 41): 41.1 to 41.3 are done. The evaluation plan works on documents of OSCAL 1.2.3 (import, export, deletion, every write checked against the schemas of NIST); catalogs are made in the program from the PDFs of a standard, with the AI proposing groups and controls and the evaluator accepting, correcting or rejecting them in the editor of a catalog (`ui/src/components/CatalogEditor.svelte`). The quotes and the proposals are the evaluation support (`db/evaluation_support/`), linked to OSCAL by UUID and id.
- **The review starts anew** on the code of v0.5 (`review.md`): no approval moved with the code.

## What comes next

1. **41.4: profile and assessment plan** (the evaluation plan proper), shown as a graph; the CEM and the Protection Profiles as documents of OSCAL that a plan can take. Concept details first where `docs/oscal.md` leaves them open (section 4: the import of an SSP).
2. **41.5: assessment results** (the evaluation), then steps 31 to 36 on them.
3. The user's review (step 12), file by file: Claude answers precisely (the user knows C and is new to Rust: explain Rust through C analogies), points out errors, and changes code only on request; it does not push for decisions.
4. Later steps in the order table of `docs/todo.md`.

## Rules that are easy to forget

- Concept first, approved by the user, then code; small steps, each with tests (`<file>_tests.rs`, attached with `#[cfg(test)] #[path = "..."] mod tests;`); `cargo fmt` before a commit.
- The direction of the dependencies is checked by `src/architecture_tests.rs`; a new use against it is a finding, or an exception written there with its reason.
- Every command of the window is listed in four places (`tauri_ui/commands.rs`, `main.rs`, `build.rs`, `capabilities/default.json`); a test checks them.
- Serialization only in `src/tauri_ui/serializer/`; the core types carry no serde attributes.
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
cargo clippy --all-targets  # only the known warnings of unused code
```

Rust comes from rustup (stable, 1.99.1 on 2026-10-18). The system also has the rustc/cargo of the Debian packages (1.95), and `/etc/fish/config.fish` puts `/usr/bin` first in `PATH`; `~/.config/fish/config.fish` moves `~/.cargo/bin` back to the front (`fish_add_path --move --path`). If `rustc --version` shows 1.95, that line is missing, and clippy (rustup only) fails with E0514 on a build made by the other compiler; after a change of compiler, `cargo clean` once.

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

- Parrot OS (KDE on Wayland, dark GTK preference). Installing packages needs `apt install -t echo-backports ...`. Poppler (`pdftotext`, `pdfinfo`) is installed.
- Shell of the user is fish; tools run in bash.
