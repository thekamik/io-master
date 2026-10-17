# Handover for a new session (2026-10-16)

Read this file, `CLAUDE.md`, `review.md`, `docs/architecture.md` and the order table of `docs/todo.md` before doing anything. The user talks in Polish; code comments and documents are in English.

## Where the project is

- Repository `/home/kamil/Projects/io-master`, crate `io-master/` (Rust 2024, system rustc 1.95), branch **`v0.4`**. Branches `v0.2`/`V0.2` must never be changed. `v0.3` keeps the old web interface.
- **Nothing since commit `acedc8b` is committed.** A commit is made only after the user approves it, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Built in this phase (all tested; 289 Rust tests pass, `npm run check` clean, clippy only known dead-code warnings):
  - the desktop window (Tauri 2.12 + Svelte 5, `src/tauri_ui/`, `ui/`), the old `web_ui` removed;
  - step 15: AI connections without any provider code (`src/ai/`: connection, connections, request, response, json_path, client, presets, field_help); Settings has tabs User / AI / Program;
  - a new look of the window (`ui/src/style.css`, tokens, dark variant);
  - steps 16–22: evaluation schemes (`src/evaluation/`, `tauri_ui/standard_actions.rs`, `tauri_ui/scheme_actions.rs`, `ui/src/pages/Schemes.svelte`, `SchemeEditor.svelte`, `PdfViewer.svelte`, `AnalysisPanel.svelte` …). Details: `docs/todo.md`, section "Evaluation schemes", "State (2026-10-16)".
- The logo of v0.3 may not be used; it was removed from the git history of v0.3 and v0.4. Never bring back a picture from history.

## What comes next

1. **Code review, step 12 of `docs/todo.md`, led by the user.** The user reads the code file by file, asks and edits by hand. Claude answers precisely (the user knows C, is new to Rust: explain Rust through C analogies), points out errors, and changes code **only on request**. Claude does not review on its own initiative and does not push for decisions.
   - `// ok kk` above a function: approved; every line added or removed in it needs the user's approval first. Reading and suggesting is fine; its tests may be changed. Claude never adds or removes the marker.
   - When the user says a whole file is reviewed, write it into `review.md` with the date; every function of it then counts as `// ok kk`.
   - `review.md` already notes: `io-master/src/main.rs` approved 2026-10-16, then changed only in its module lists (new modules `ai`, `evaluation`, `tauri_ui`); the function `main` is unchanged. The commented-out block `mod web_ui` in it waits for a decision.
2. Steps 24 to 27 of `docs/todo.md` were built on 2026-10-16 (a requirement as the condition of another; test projects holding one scheme per chapter; links between schemes and moving tiles; AI reading the table of contents and analysing chapters). Details: `docs/todo.md`, "State (2026-10-16): steps 24 to 27". **The user must delete the database of the old test installation before starting the program** (the tables changed; no migration). Next: the user tests them with a real standard and a real model. Step 28 (reports in the LaTeX template of the laboratory) and step 23 (evaluation with AI) come later.
3. **Evaluation (step 23), planned 2026-10-17** as steps 29 to 36 of `docs/todo.md` (section "Step 23"): one requirement at a time; the documentation of the product in two phases (fragments = verified quote + separate AI note; only fragments accepted by the evaluator go into the assessment); the documentation of the manufacturer in a tool of its own (step 30, designed later; placeholder page `Documentation.svelte` in the sidebar). Step 29 (the evaluation project, the four results, the journal; `src/product_evaluation/`) was built on 2026-10-17 and waits for the user's review.
4. **Step 37 (2026-10-17): the database layer by areas and CRUD**, raised by the user as critical and built the same day: all SQL in `src/db/<area>/{schema,rows,create,read,update,delete}.rs`, encryption in `src/db/encryption/` moved unchanged (the user audits it next and plans to redesign it). Rule in CLAUDE.md, "Database code". 312 Rust tests pass.
5. **Step 38 (2026-10-17): the AI layer**: `ai/api/` (REST-like: roles, tasks, completions, checks; `provider/` private), `ai/chat/`, `db/ai/` subfolders (connection, memory, roles, instructions, journal, presets), the journal of every question to the AI. Rules in CLAUDE.md, "AI layer". 325 Rust tests pass; committed as e7711ee (steps 29, 37, 38).
6. **Step 39 (2026-10-17): the domain folders**: `standards/`, `test_projects/` (open projects in memory with versions for "Undo", CRUD folders, write-through by `db/`), `evaluation/` (was `product_evaluation/`, open evaluation in memory). Rules in CLAUDE.md, "The pipeline of data". Next: the user's review of `db/`, `ai/` and the domain folders, then the concept of step 31 (the window of the evaluation). 331 Rust tests pass.

## Rules that are easy to forget

- Concept first, approved by the user, then code; small steps, each with tests (`<module>_tests.rs`, attached with `#[cfg(test)] #[path = "..."] mod tests;`).
- Core separate from the window; serialization only in `src/tauri_ui/serializer/`; core types carry no serde attributes (the scheme file `evaluation/exchange.rs` builds JSON by hand).
- Secrets only in the encrypted database of the evaluator; never shown back, never printed; the page holds no secret.
- Every connection leaving the computer: TLS 1.3, verified certificate. Plain HTTP only to 127.0.0.1/::1 (AI connections).
- Every command of the window is listed in four places: `commands.rs`, `main.rs` (generate_handler in `tauri_startup`), `build.rs` (AppManifest), `capabilities/default.json`.
- `tauri.conf.json` keeps `freezePrototype: true`; `vite.config.ts` patches `d3-color` for it (the window does not start otherwise).
- Never capture the user's screen. Window tests run only on a private Xvfb display, see below.
- Before deleting or overwriting, look at the target. Dependencies: few, well known, versions pinned exactly, released at least two weeks before.

## How to build and test

```text
cd io-master
npm ci                      # once
npm run check               # svelte-check, must be clean
npm run build               # copies pdf.js data, builds ui/dist (needed before cargo build)
cargo build && cargo test
T=$(rustup run stable rustc --print sysroot); RUSTC=$T/bin/rustc $T/bin/cargo clippy --all-targets --target-dir target/rustup; rm -rf target/rustup
```

Window test on a private display (the scratchpad of the session holds the helpers; recreate them if missing):

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
