# Handover for a new session

Read this file, `CLAUDE.md`, `docs/data_model.md`, `docs/code_structure.md` and `docs/architecture.md` before doing anything. The user talks in Polish; code and documents are in English.

## Where the project is (2026-11-03)

- Branches: the work goes to **`test_branch`**; **`main`** and **`V0.3`** were last brought to its state on 2026-11-03, with the approval of the user; **`V0.2`** holds the closed stage of cleaning and hardening (2026-11-03), `V0.1` an earlier snapshot (`CLAUDE.md`, "Branches and commits"). The earlier program, the reference: the commit `0797e85`.
- **Built on the models of `objects/`**, in three groups (`objects/`, `panels/`, `infrastructure/`): the first start and the login; the panel **Evaluation plan** (standards imported as OSCAL catalogs, plans, work units and their parameters, the order and the cross references, the agent of the planner, "Fill with AI" in two tasks with a preview); the panel **Evaluation** (an evaluation begun on a copy of a plan with its agent; the tab Inputs: the documents and proofs given once; the tab Work units: the assessment by the AI in the order of the work with a preview and "Stop", the list with its filters, a work unit with its results corrected by the evaluator and the last question and answer of the AI); the panel **Agents** (agents, shared knowledge bases with `.txt`/`.md`/`.pdf` sources, the test of the knowledge, the journal of each agent); the panel **Reports** (the report of an evaluation written by a template with the markers of the fields of the models, shown and exported into a file; the templates, their editor, their import and export); the panel **Chat** (a chat with an agent, its answer streamed into the page, "Stop"); the panel **AI settings** (integrations, keys, the check of a connection); the panel **Account** (the change of the password, a new recovery code) and the recovery of a forgotten password with the code on the login screen. Each tried on a private display, the last time on 2026-11-01; the panel Reports on 2026-11-04 (a template made and changed, the report of an evaluation shown and exported).
- **The stage: new functionality** (user, 2026-11-03), on the line `V0.3`. The stage of cleaning and hardening (2026-11-02) is closed and kept as `V0.2`. Done in it: every function of `src/` (but the approved stores and `encryption/`) commented with `Input:`, `Output:`, `Called by:` and `Calls:` by full path, the callers checked against the code; the silent defaults of the order of the work, the limits of an agent and the recovery code replaced by checks that return an error; no `#[cfg(not(test))]` (Argon2id at its full cost in the tests too); the own closure of `poppler.rs` made a function, its loop bounded by `MAX_PARTS`.
- **The assessment by the AI reworked** (2026-11-02, after a test where Haiku failed every work unit of a certified ST: the ST had been given before its text was indexed, so the model got an empty document). Now: a given file without passages is an error; the knowledge of the agent (optional) goes before the task, each passage under its source; a task of the documentation and one of a functional test; the verdict is the model's, a dependency that did not pass only warned of; no earlier results in the question; an answer cut or refused is not taken; a work unit without evidence is not assessed; the default context of an integration is 128 000 tokens, a smaller one warned of. **The user starts the tests anew** with a fresh data folder.
- **Every text of a question is a parameter** (user, 2026-11-02: no magic numbers, no hardcoded parameters): the templates of the agent (`system_template`, `knowledge_template`, `passage_template`) and of its role (the questions, each dependency, document, passage and proof, the searches, the name of a default proof), filled in one pass by `objects/ai/agent/agent_ai.rs`: `filled`; the characters of a token are a field of the integration. A template without a placeholder its question needs is saved with a warning in the editor. The tables `ai_agents`, `ai_agent_evaluators`, `ai_agent_planners` and `ai_integrations` changed: a fresh data folder is needed.
- The data of the tests is temporary: after a change of a table, remove `~/.local/share/io-master` and start anew.

## What comes next

The user chooses; the open work is in `docs/todo.md`, the review of the code in `review.md`.

## How to build and test

```text
cd io-master
npm ci                      # once
npm run check               # svelte-check, must be clean
npm run build               # builds ui/dist (needed before cargo build)
cargo fmt --check
cargo build && cargo test   # also the test of the architecture
cargo clippy --all-targets  # clean, pedantic included (Cargo.toml)
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
- The window is 1280x860 at 0,0. First start: name 640,406, passwords 640,486 and 640,566, Install 640,627; the recovery code: the group 640,564, "go on" 640,624. Login: name 640,424, password 640,505, Log in 640,566, "I do not remember my password" 640,611; the recovery: name 640,355, code 640,435, new passwords 640,514 and 640,595, "Set the new password" 640,655 (all 29 lower after an error). Side bar: Evaluation plan 100,108, Evaluation 100,152, Reports 100,195, Agents 100,238, Chat 100,281, AI settings 100,324, Account 100,367. Account: present password 523,264, new passwords 523,344 and 523,424, "Change the password" 406,476; the password of a new code 523,708, "Make a new recovery code" 426,759 (79 lower after a notice above). A date field of WebKit takes no typed digits on Xvfb: click it, press Up to open its calendar, click the day, then Escape. Chat: "New chat" 404,127; the first chat of the list 404,176; the field of the question 895,295 in an empty chat, below the last answer when it has messages (Ctrl+Enter sends it). AI settings: "A new integration" 392,270; its fields start from the values of the protocol (Ollama on this computer), "Add the integration" 399,842. Agents: the name of a new agent 763,434 (the role Evaluator and the integration chosen first), "Make the agent" 431,707. Reports: the tabs Reports 330,128 and Templates 437,128; the first evaluation "Open" 1109,283, then "Export" 347,315 (a save dialog of GTK: Ctrl+A, the whole path, Enter); a new template: its name 763,460, "Make the template" 396,511. The fake AI must not be killed by a pattern that stands in the command line of the shell itself: find it in `/proc/<pid>/cmdline`.
- A GTK file dialog: put the path with `xclip -display :99 -selection clipboard`, then Ctrl+L, Ctrl+V, Enter.
- A fake AI: a small Python server on 127.0.0.1:18080 answering in the chat-completions stream format, writing every request into a file (to check the key goes only into its header); an integration with the address `http://127.0.0.1:18080/v1/chat/completions` points to it. For the assessment of an evaluation it answers with the JSON of an assessment (findings, analysis, vulnerability notes, notes, quotes, verdict), a quote taken from the text of the document given, so that the check of the quotes is tried both ways.
- Stop with `pkill -x Xvfb`, `pkill -x io-master`; Python servers with `kill $(pgrep -f '[f]ake_ai.py')` (a plain `pkill -f` kills the shell itself).

## The environment

Debian 13 (trixie), XFCE on X11; the user's shell is fish, tools run in bash. A full build of Tauri is heavy for 8 GB of RAM: `cargo build -j 2` if the linker runs out of memory.
