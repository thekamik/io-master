# Handover for a new session

Read this file, `CLAUDE.md`, `docs/data_model.md`, `docs/code_structure.md` and `docs/architecture.md` before doing anything. The user talks in Polish; code and documents are in English.

## Where the project is (2026-10-29)

- Branches: **`main`** holds the earlier program (the reference), **`test_branch`** the rebuild, **`V0.1`** a snapshot of the rebuild before the chat (nothing committed to it). Nothing goes to `main` until the user revokes it.
- **Built on the models of `model/`:** the first start and the login; the panel **Evaluation plan** (standards imported as OSCAL catalogs, plans, work units and their parameters, the order and the cross references, the agent of the planner, "Fill with AI" in two tasks with a preview); the panel **Evaluation** (an evaluation begun on a copy of a plan with its agent, the documents and proofs given once; the assessment by the AI comes next); the panel **Agents** (agents, shared knowledge bases with `.txt`/`.md`/`.pdf` sources, the test of the knowledge, the journal of each agent); the panel **Chat** (a chat with an agent, its answer streamed into the page, "Stop"); the panel **AI settings** (integrations, keys, the check of a connection); the way to the AI (`ai/`). Each tried on a private display.
- The data of the tests is temporary: after a change of a table, remove `~/.local/share/io-master` and start anew.

## What comes next

The user chooses; the open work is in `docs/todo.md`.

## How to build and test

```text
cd io-master
npm ci                      # once
npm run check               # svelte-check, must be clean
npm run build               # builds ui/dist (needed before cargo build)
cargo fmt --check
cargo build && cargo test   # also the test of the architecture
cargo clippy --all-targets  # clean but dead_code
```

Rust from rustup only (two compilers in `PATH` break clippy with E0514; after a change of compiler, `cargo clean` once).

**Window test on a private display** (helpers in the scratchpad of the session; written anew when missing):

```text
Xvfb :99 -screen 0 1600x1000x24 -nolisten tcp &
env -u WAYLAND_DISPLAY DISPLAY=:99 GDK_BACKEND=x11 XDG_DATA_HOME=<scratch>/data \
    WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1 \
    ./target/debug/io-master &
```

- Drive it with XTEST from Python (`ctypes`, libXtst): click, type, key, combo, scroll; screenshots with `import -display :99 -window root`. Name modifiers by keysym (`Control_L`). On Xvfb the keys may come late: pause between fields.
- The window is 1280x860 at 0,0. First start: name 640,406, passwords 640,486 and 640,566, Install 640,627; the recovery code: the group 640,564, "go on" 640,624. Login: name 640,446, password 640,526, Log in 640,587. Side bar: Evaluation plan 100,108, Evaluation 100,152, Agents 100,195, Chat 100,239, AI settings 100,282. A date field of WebKit takes no typed digits on Xvfb: click it, press Up to open its calendar, click the day, then Escape. Chat: "New chat" 404,127; the first chat of the list 404,176; the field of the question 895,720 (Ctrl+Enter sends it).
- A GTK file dialog: put the path with `xclip -display :99 -selection clipboard`, then Ctrl+L, Ctrl+V, Enter.
- A fake AI: a small Python server on 127.0.0.1:18080 answering in the chat-completions stream format, writing every request into a file (to check the key goes only into its header); an integration with the address `http://127.0.0.1:18080/v1/chat/completions` points to it.
- Stop with `pkill -x Xvfb`, `pkill -x io-master`; Python servers with `kill $(pgrep -f '[f]ake_ai.py')` (a plain `pkill -f` kills the shell itself).

## The environment

Debian 13 (trixie), XFCE on X11; the user's shell is fish, tools run in bash. A full build of Tauri is heavy for 8 GB of RAM: `cargo build -j 2` if the linker runs out of memory.
