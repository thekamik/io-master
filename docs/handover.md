# Handover for a new session

Read this file, `CLAUDE.md`, `review.md`, `docs/code_structure.md`, `docs/architecture.md` and the order table of `docs/todo.md` before doing anything. The user talks in Polish; code comments and documents are in English.

## Where the project is (2026-10-17)

- Repository `/home/kamil/Projects/io-master`, crate `io-master/` (Rust 2024, Tauri 2 + Svelte 5), branch **`v0.5`**. `v0.4` keeps the state before step 40 and is not changed; `v0.2`/`V0.2` are never changed; `v0.3` keeps the old web interface.
- **Step 40 (the cleanup of the architecture) is built** on `v0.5`, one commit per part, all pushed: the layers `model/`, `db/`, `ai/`, `hardware_tests/`, `security/`, `business/`, `tauri_ui/`, their rules and the tests that check them. Everything is in `docs/code_structure.md`; the parts in `docs/todo.md`, step 40. It waits for the user's review.
- Built before and carried into the new layout (details in `docs/history.md`): the window, the installation and login, AI connections and chat, the scan of serial ports, documents of standards with the viewer, test projects with schemes, tiles, links and the analysis with the AI, export and import, and the core of the evaluation project (step 29; no window yet).
- **The review starts anew** on the code of v0.5 (`review.md`): no approval moved with the code.

## What comes next

1. **The user reviews the new layout** (step 40), then the code file by file (step 12), from the bottom of the layers up. Claude answers precisely (the user knows C and is new to Rust: explain Rust through C analogies), points out errors, and changes code only on request; it does not push for decisions.
2. Then **step 31: the window of the evaluation** (concept first), on top of `business/evaluations/`.
3. Later steps in the order table of `docs/todo.md` (13, 6, 8, 9, 10, 4, 11; 30 and 28 with step 23).

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
T=$(rustup run stable rustc --print sysroot); RUSTC=$T/bin/rustc $T/bin/cargo clippy --all-targets --target-dir target/rustup; rm -rf target/rustup
```

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
