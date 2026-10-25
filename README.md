# IO Master

**A desktop tool for evaluation laboratories: it turns a standard into a catalog of NIST OSCAL, plans its evaluation as tasks with a scheme each, and carries the evaluation of a product through it with the help of AI, while every decision stays with the evaluator.**

IO Master is installed on the computer of an evaluator, in their own Linux account, and used through one desktop window. It knows no particular standard: the requirements of a standard are kept as catalogs of **NIST OSCAL 1.2.3** (Open Security Controls Assessment Language), every document checked against the JSON schemas of NIST. Catalogs published in OSCAL are imported as they are; standards that are not (Common Criteria with the CEM, ETSI EN 303 645, ...) are made into catalogs in the program, from their PDFs, with the help of AI, and every part traced back to the passage it comes from.

**Project status:** early development, experimental (no releases and no version numbers yet; `main` holds the tested state, the work goes on in `test_branch`; the data of the tests is not kept between versions until version 1). A high-level overview of the functions: `docs/presentation.md`.

## Aims

1. **Support the evaluation of a product against a standard** (Common Criteria with the CEM and ETSI EN 303 645 first; any standard the same way, by the same code):
   - **the catalog**: the requirements of the standard in OSCAL, made from its PDFs with the AI or imported;
   - **the evaluation plan**: a master project of the standard, its tasks (each a whole requirement), the scheme of each task (the evidence expected, the instructions of the agents of the AI, the outputs and when they are written), and the work packages delegated to the evaluators;
   - **the evaluation**: a work package carried out for one product, requirement by requirement: the evidence, the fragments of it that matter, the four results and the verdict, the texts of the outputs, and the report.

   The AI reads and proposes; the evaluator accepts, corrects or rejects.
2. **Carry the inputs to the output.** IO stands for input and output: the inputs are the documentation of the client and the work of the evaluator (results of tests, notes, assessments), the output is the report for the template of the laboratory. The program does not talk to the device under evaluation; what the laboratory finds on it enters as evidence.

## Principles

These are requirements of the design, not preferences (`docs/architecture.md`, section 2):

| | Principle | Consequence |
|---|---|---|
| P1 | **The evaluator decides.** | The program never issues a verdict. It proposes, with its reasons and its sources; the evaluator accepts. |
| P2 | *Removed (step 48): the program no longer works with devices.* | |
| P3 | **Everything is traceable.** | Every question to the AI and its answer is in the journal of the AI. |
| P4 | **The standard is data, not code.** | Standards are kept as OSCAL documents; nothing about a standard is written into the program. |
| P5 | **The model reads; it does not invent sources.** | Every quote the AI proposes is looked for by the program in the page or the evidence it names; a quote that is not there is shown as such. |

## What works today

- **Installation, login and recovery.** One Linux account holds one installation with one user. The data of the evaluator is in a database encrypted with SQLCipher, whose key opens only with their password (Argon2id); a **recovery code** (160 bits, shown once at the setup and after every change of the password) opens it when the password is forgotten.
- **AI connections.** The program knows no provider: a connection is described completely in the window (endpoint, model, headers, the template of the request, where the answer is), meant first of all for a local model of the laboratory, with presets for known services. HTTPS with TLS 1.3 and a verified certificate, or plain HTTP only to the loopback; an API key is bound to the address it was entered for. Every question to a model is written into the journal of the AI.
- **Chat** with the AI, kept in the encrypted database.
- **Converter.** The PDFs of a standard loaded page by page (Poppler), searched and shown in a built-in viewer; a catalog of OSCAL made from them step by step: the AI finds the table of contents, proposes the groups and, group by group, the controls, each with quotes the program checks on their pages; the evaluator accepts, corrects or rejects every proposal, or edits the catalog by hand.
- **Evaluation plan.** The documents of OSCAL (catalogs, mappings, profiles, assessment plans and results) imported, exported and deleted. **Master projects** of a standard: the tasks chosen in a browser of the standards or proposed by the AI from the scope of the project; requirements of the laboratory outside the standards; the **scheme of every task** drawn as blocks (inputs → requirement → outputs, each output with its format, text or LaTeX, and its condition: always, only when the requirement is met, only when not met), proposed or corrected by the AI; a template of the outputs of the project; the **related elements** of a task (the work units of the CEM, ...) found by the AI; **work packages** delegated to the evaluators and exported as one file with the catalogs attached.
- **Evaluation lab.** A work package imported for one product. For every task: the **evidence** of each input (PDF, text file, pasted notes), kept as delivered; a **chat** about whether the evidence is ready; the **fragments** that matter (exact quotes the program finds in the evidence, with a separate note of the AI), accepted by the evaluator; the **agent of the requirement**, which assesses only the accepted fragments and proposes the four results (findings, analysis, assessment and verdict proposition, vulnerabilities and notes) and a verdict (met, not met, inconclusive), which the evaluator decides; the **outputs** whose condition holds, drafted by the AI from the results and accepted by the evaluator; the **report**, one file per output (`.txt` or `.tex`).
- **The evaluation support.** What the program adds to OSCAL (quotes, proposals of the AI, instructions, master projects, evaluations) is kept in tables of its own, linked to the documents of OSCAL only by UUIDs and ids, so that nothing of ours is written into a document of OSCAL.

## Planned

`docs/todo.md`: the review of the code by the user and its hardening, knowledge for the AI (RAG: agents with a knowledge base of a standard, attached to the analysis of a requirement), the journal of the AI on a page, the progress of an evaluation, the removal of the installation in the window, an export of the plan and of the results as OSCAL documents, the package for Debian.

## Requirements

- Linux with a graphical desktop session (XFCE on X11 is the tested one).
- **System:** Debian 13 (trixie), the target system and the only one tested. Other distributions may work but are not tested.
- Poppler (`pdfinfo`, `pdftotext`) to load the documents of standards.
- For building: Rust (stable, edition 2024) from rustup, Node.js 20.19 or newer with npm, and the libraries of Tauri 2 on Linux (WebKitGTK 4.1 and its companions), OpenSSL (used by SQLCipher) and D-Bus (used by the window library of Tauri).

The packages, in three groups: to run the program, to build it, and for the tests (the window on a private display).

| Needed for | Debian 13 (`apt`) |
|---|---|
| running | `poppler-utils` |
| building | `build-essential pkg-config libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev libdbus-1-dev nodejs npm` |
| tests | `xvfb xclip imagemagick x11-utils libxtst6 python3` |

All at once (the plain repositories of Debian, no backports):

```bash
sudo apt install poppler-utils \
    build-essential pkg-config libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev libdbus-1-dev nodejs npm \
    xvfb xclip imagemagick x11-utils libxtst6 python3
```

Rust comes from rustup (the installer of https://rustup.rs), then `rustup default stable`. Do not install the `rustc` and `cargo` packages of the system beside it: two compilers in `PATH` build with one and run clippy with the other, which fails (E0514). After a change of compiler, run `cargo clean` once.

## Building and testing

```bash
git clone https://github.com/thekamik/io-master.git
cd io-master/io-master
npm ci                 # once: the tools of the page
npm run check          # svelte-check of the page, must be clean
npm run build          # the page, built into the program
cargo build            # the program
cargo test             # the tests, also of the architecture and of the command lists
./target/debug/io-master
```

The program is started without parameters; everything is set in its window.

## The repository

```text
io-master/          the program (Rust, Tauri 2 + Svelte 5)
  src/              the core and the window, in layers: model, db, ai,
                    security, pdf, installation, app, components, views
                    (docs/code_structure.md); one folder per view and per element
  ui/               the page of the window, a separate project
  oscal/            the JSON schemas of OSCAL 1.2.3 and example documents of NIST
docs/
  presentation.md   a high-level overview of the functions
  architecture.md   what the program is and why: the account, the keys, the AI, the standards, the flow of data
  oscal.md          how NIST OSCAL is used, and where the program has a model of its own
  code_structure.md where everything is in the code, and the rules that keep it in order
  todo.md           the open work: a heap of reminders, not an order
  history.md        the steps already built, dropped or replaced, with their decisions
  handover.md       the state of the work, for a new working session
testing_documents/  documents for trying the program by hand (the CEM:2022 as an OSCAL catalog)
review.md           the state of the code review
```

The earlier versions (0.2 in C with console shells, the later web interface) are in the history of git; `docs/history.md` names the commits.

## License

GNU General Public License, version 2 (`LICENSE`).
