# IO Master

**A desktop tool for evaluation laboratories: it maps a standard into a plan of tests, carries the evaluation of a product through it with the help of AI, and works with the device under evaluation, while every decision stays with the evaluator.**

IO Master is installed on the computer of an evaluator, in their own Linux account, and used through one desktop window. It knows no particular standard: the documents of a standard (Common Criteria, the CEM, EN 18031, ...) are loaded as PDFs, and everything the program does with them is traced back to the passages they come from.

**Project status:** development, version 0.5 (branch `v0.5`). The functions listed under "What works today" are built and tested; the evaluation itself, the work on devices beyond the scan, and the release hardening are planned.

## Aims

1. **Support the evaluation of a product against a standard.** An evaluator works requirement by requirement: what the standard requires, what the evaluation needs for it (documentation, a sample, access, the result of another requirement), what was done and found, and the verdict. The program prepares and records that work; the AI reads the standard and the documentation of the product and proposes, and the evaluator accepts, corrects or rejects. Common Criteria (ISO/IEC 15408 with the CEM, ISO/IEC 18045) is the first target, but the program is not bound to one standard.
2. **Investigate the device under evaluation.** Find its interfaces (serial ports today; UART and Bluetooth, re-created from version 0.2, next), talk to it, and keep what was observed, so that tests can be carried out on the device and their record becomes part of the evaluation.

## Principles

These are requirements of the design, not preferences (`docs/architecture.md`, section 2):

| | Principle | Consequence |
|---|---|---|
| P1 | **The evaluator decides.** | The program never issues a verdict. It proposes, with its reasons and its sources; the evaluator accepts. |
| P2 | **No command reaches a device without an action of the evaluator for that command.** | The program proposes; the evaluator executes. Acting on a device carries responsibility. |
| P3 | **Everything is traceable.** | Every question to the AI and its answer is in the journal of the AI; every step of an evaluation in its journal, which is only ever added to. |
| P4 | **The standard is data, not code.** | Standards are loaded from their documents; nothing about a standard is written into the program. |
| P5 | **The model reads; it does not invent sources.** | Every passage the AI proposes is checked against the page it names; a passage that is not there is shown as such. |

## What works today

- **Installation and login.** At the first start the window asks for the user and the role of the installation (workstation or master). One Linux account holds one installation with one user. The data of the evaluator is in a database encrypted with SQLCipher, whose key opens only with their password (Argon2id).
- **AI connections.** The program knows no provider: a connection is described completely in the window (endpoint, model, headers, the template of the request, where the answer is), meant first of all for a local model of the laboratory, with presets for known services. HTTPS with TLS 1.3 and a verified certificate, or plain HTTP only to the loopback; an API key is bound to the address it was entered for. Every question to a model, from any part of the program, is written into the journal of the AI.
- **Chat** with the AI, kept in the encrypted database.
- **Scan of serial ports** with the kinds of port and the columns the evaluator chooses; the scan never opens a port.
- **Documents of standards.** PDFs loaded page by page (Poppler), searched, and shown in a built-in viewer with the quoted passages highlighted.
- **Test projects: the map of a standard.** A project holds one scheme per chapter; a scheme is drawn as tiles (inputs, requirements, outputs) joined by links whose meaning follows from the kinds they join, across chapters too. The AI reads the table of contents into proposed schemes and analyses the pages of each chapter into proposed tiles and links with verified quotes; the evaluator accepts or corrects. Checks name the gaps of a scheme; "Undo" takes back a change, also a whole analysis. A project is exported and imported as a file.
- **Evaluation projects (core).** A test project becomes an evaluation of one product: its accepted part frozen, four text results per requirement, proposals of the AI kept apart until the evaluator accepts them, and a journal of every step. The window of the evaluation is the next step.

## Planned

- The window of the evaluation and its work with the AI: the fragments of the documentation of the product that matter for a requirement, then the assessment on the fragments the evaluator accepted (`docs/todo.md`, steps 31 to 36).
- A tool for the documentation of the manufacturer (step 30) and reports in the LaTeX template of the laboratory, committed to its git (step 28).
- The work on devices: access rights to ports, UART and Bluetooth (steps 6 and later).
- Protection: the keyring of the Linux account and the secret of the application (step 13), the TPM 2.0 and passkeys as optional further factors (steps 8 and 9), and the master with an optional password reset (steps 4 and 11).

## Requirements

- Linux with a graphical desktop session and a keyring through the Secret Service interface (GNOME Keyring; GNOME and XFCE are the supported desktops).
- `poppler-utils` (`pdfinfo`, `pdftotext`) to load the documents of standards.
- For building: Rust (edition 2024), Node.js with npm, and the libraries of Tauri 2 on Linux (WebKitGTK 4.1 and its companions), OpenSSL (`libssl-dev`, used by SQLCipher) and `libtss2-dev` (the TPM library).

On Debian, Ubuntu or Parrot OS:

```bash
sudo apt install build-essential pkg-config libssl-dev libtss2-dev poppler-utils \
    libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

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
  src/              the core and the window, in layers: model, db, ai, hardware_tests,
                    security, business, tauri_ui (docs/code_structure.md)
  ui/               the page of the window
docs/
  architecture.md   what the program is and why: roles, secrets, AI, collaboration, devices
  code_structure.md where everything is in the code, and the rules that keep it in order
  todo.md           the plan of the work, step by step
  history.md        the steps already built, with their decisions
  handover.md       the state of the work, for a new working session
v0.2_functionality.md   the functions of version 0.2 (C), the catalogue the new version re-creates
review.md           the state of the code review
```

Version 0.2, written in C with console shells for UART, Bluetooth and an AI chat, is kept in the branch `v0.2` and in the git history; version 0.3 (a web interface) in the branch `v0.3`.

## License

GNU General Public License, version 2 (`LICENSE`).
