# IO Master

**A desktop tool for security evaluation laboratories: it plans the evaluation of a product against standards kept as NIST OSCAL catalogs, and brings AI agents with their own knowledge to each work unit, while every decision stays with the evaluator.**

IO Master is installed on the computer of an evaluator, in their own Linux account, and used through one desktop window. It knows no particular standard: a standard is an **OSCAL 1.2.3 catalog**, checked against the JSON schema of NIST (Common Criteria with the CEM and ETSI EN 303 645 first).

**Status:** early development, rebuilt on a new data model. The work goes to `test_branch`; `main` and `V0.1` are brought to its state from time to time; the earlier program is kept at the commit `0797e85`. No releases; the data of the tests is not kept between versions until version 1.

## What works today

- **First start and login.** One Linux account, one installation, one user. The data is in a database encrypted with SQLCipher, its key opened by the password (Argon2id); a recovery code, shown once, is a second key.
- **Evaluation plan.** Standards imported as OSCAL catalogs; plans that name their standards and the places of the documents of the product; work units made from the controls, with their dependencies, documents, proofs of the laboratory, guidance and the agent that will assess them; the order of the work and the cycles of dependencies found (Kahn's algorithm). **Fill with AI:** the agent of the planner fills the parameters, then the dependencies, of every work unit.
- **Evaluation.** An evaluation begins on a copy of a plan, with the identification of the product, the client and the evaluator, and the agent of the evaluation. The documents of the product (PDF or text) and the proofs of the laboratory are given once and then frozen; the assessment does not start while one that a work unit needs is missing. **Assess with AI:** every work unit in the order of the work, autonomously (it may run overnight), with a preview of the questions and "Stop"; each question carries the work unit, the results of its dependencies, its documents (whole, or the passages that fit it best) and its proofs. The program checks every quote word for word in its document and makes the verdict the worst of its own and those of its dependencies. The list of the work units with filters and pages; a work unit with its results, which the evaluator may correct (then the AI never writes them again), and the last question and answer of the AI.
- **Chat.** A conversation with an agent chosen by the user, its knowledge searched for each question, the answer streamed into the window and stopped at once on "Stop"; a long chat leaves its oldest messages out of the question. Chats are kept, never removed.
- **Agents.** An agent is a layer of knowledge over the model of an integration: its instructions, shared knowledge bases (`.txt`, `.md`, `.pdf`, searched on this computer), a glossary, its limits, and its own journal of every question and answer.
- **AI settings.** Integrations with AI services (the chat completions of OpenAI, which local servers offer, or the messages of Anthropic): HTTPS with TLS 1.3, or plain HTTP only on this computer; the API key only in its header, never shown back.

Next: the state Report of a finished evaluation and the report itself, the login with the recovery code (`docs/todo.md`).

## Requirements

Debian 13 (trixie), the target system and the only one tested, with a graphical session (XFCE on X11 is tested).

| Needed for | Debian 13 (`apt`) |
|---|---|
| running | `poppler-utils` |
| building | `build-essential pkg-config libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev libdbus-1-dev nodejs npm` |
| tests in the window | `xvfb xclip imagemagick x11-utils libxtst6 python3` |

Rust (stable, edition 2024) from rustup (https://rustup.rs), then `rustup default stable`; not the `rustc` and `cargo` packages of the system beside it. Node.js 20.19 or newer.

## Building and testing

```bash
git clone https://github.com/thekamik/io-master.git
cd io-master/io-master
npm ci                 # once: the tools of the page
npm run check          # svelte-check of the page
npm run build          # the page, built into the program
cargo build
cargo test             # also the test of the architecture
./target/debug/io-master
```

The program is started without parameters; everything is set in its window.

## The repository

```text
io-master/          the program (Rust, Tauri 2 + Svelte 5)
  src/              model, db, ai, session, panels (logic and views), security, pdf
  ui/               the page of the window, a separate project
  oscal/            the JSON schemas of OSCAL 1.2.3 and example documents of NIST
docs/
  architecture.md   what the program is and why: the user, the keys, the AI, the standards
  data_model.md     the models, their links and their rules
  code_structure.md where everything is in the code, and the rules that keep it in order
  todo.md           the open work
  handover.md       the state of the work, for a new working session
testing_documents/  documents for trying the program by hand (the CEM:2022 as an OSCAL catalog, the script that made it)
review.md           the state of the code review
```

## License

GNU General Public License, version 2 (`LICENSE`).
