# IO Master

**A desktop tool for evaluation laboratories: it turns a standard into an evaluation plan in NIST OSCAL, carries the evaluation of a product through it with the help of AI, and works with the device under evaluation, while every decision stays with the evaluator.**

IO Master is installed on the computer of an evaluator, in their own Linux account, and used through one desktop window. It knows no particular standard. The requirements of a standard, the plan of their evaluation and its results are kept in **NIST OSCAL 1.2.3** (Open Security Controls Assessment Language), the published, machine-readable format of NIST, and every document is checked against the JSON schemas of NIST. Catalogs published in OSCAL are imported as they are; standards that are not (Common Criteria with the CEM, ETSI EN 303 645, ...) are made into OSCAL catalogs in the program, from their PDFs, with the help of AI, and every part traced back to the passage it comes from.

**Project status:** early development (no releases and no version numbers yet; `main` holds the tested state, the work goes on in `test_branch`). The functions listed under "What works today" are built and tested; the evaluation itself, the work on devices beyond the scan, and the release hardening are planned.

## Aims

1. **Support the evaluation of a product against a standard**, in two functions:
   - **the evaluation plan:** the requirements of the standard as an OSCAL catalog (made in the program or loaded), the ones chosen for one evaluation as a profile, and the plan of the work as an OSCAL assessment plan, shown as a graph;
   - **the evaluation:** the plan carried out for one product, requirement by requirement: the evidence, the checks whether each requirement is met, and the report, recorded as OSCAL assessment results.

   The AI reads the standard and the documentation of the product and proposes; the evaluator accepts, corrects or rejects. Common Criteria (ISO/IEC 15408 with the CEM, ISO/IEC 18045) and ETSI EN 303 645 are the first targets, but every standard is represented the same way, in OSCAL, by the same code.
2. **Investigate the device under evaluation.** Find its interfaces (serial ports today; UART and Bluetooth, re-created from version 0.2, next), talk to it, and keep what was observed, so that tests can be carried out on the device and their record becomes part of the evaluation.

## Principles

These are requirements of the design, not preferences (`docs/architecture.md`, section 2):

| | Principle | Consequence |
|---|---|---|
| P1 | **The evaluator decides.** | The program never issues a verdict. It proposes, with its reasons and its sources; the evaluator accepts. |
| P2 | **No command reaches a device without an action of the evaluator for that command.** | The program proposes; the evaluator executes. Acting on a device carries responsibility. |
| P3 | **Everything is traceable.** | Every question to the AI and its answer is in the journal of the AI; every step of an evaluation in its journal, which is only ever added to. |
| P4 | **The standard is data, not code.** | Standards are kept as OSCAL documents; nothing about a standard is written into the program. |
| P5 | **The model reads; it does not invent sources.** | Every passage the AI proposes is checked against the page it names; a passage that is not there is shown as such. |

## What works today

- **Installation and login.** At the first start the window asks for the user and the role of the installation (workstation or master). One Linux account holds one installation with one user. The data of the evaluator is in a database encrypted with SQLCipher, whose key opens only with their password (Argon2id).
- **AI connections.** The program knows no provider: a connection is described completely in the window (endpoint, model, headers, the template of the request, where the answer is), meant first of all for a local model of the laboratory, with presets for known services. HTTPS with TLS 1.3 and a verified certificate, or plain HTTP only to the loopback; an API key is bound to the address it was entered for. Every question to a model, from any part of the program, is written into the journal of the AI.
- **Chat** with the AI, kept in the encrypted database.
- **Scan of serial ports** with the kinds of port and the columns the evaluator chooses; the scan never opens a port.
- **Documents of standards.** PDFs loaded page by page (Poppler), searched, and shown in a built-in viewer with the quoted passages highlighted.
- **OSCAL documents.** Catalogs, mappings, profiles, assessment plans and assessment results of OSCAL 1.2.3 imported from a file, exported and deleted; every document is checked against the schema of NIST when it is written, and kept in the encrypted database.
- **Catalogs made in the program.** A standard that is not published in OSCAL is made into a catalog in an editor: a tree of its groups and controls, each with the passages of the standard it comes from (checked on their pages, shown in the viewer) and its fields as JSON of OSCAL. The AI proposes the groups from the table of contents and the controls from the pages of a group, with progress and a Stop button; the evaluator accepts, corrects or rejects every proposal.
- **The evaluation support.** What the program adds to OSCAL (the quotes and their checks, the proposals of the AI, the instructions to the AI) is kept apart, in its own tables, and linked to the OSCAL objects only by their UUIDs and ids, so that nothing of ours is written into an OSCAL document.

## Planned

- The evaluation plan as a profile and an assessment plan, shown as a graph; the CEM and the Protection Profiles as OSCAL documents a plan can take (`docs/todo.md`, step 41.4).
- The evaluation as OSCAL assessment results (step 41.5), and its work with the AI: the fragments of the documentation of the product that matter for a requirement, then the assessment on the fragments the evaluator accepted (steps 31 to 36).
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
  oscal/            the JSON schemas of OSCAL 1.2.3 and example documents of NIST
docs/
  architecture.md   what the program is and why: roles, secrets, AI, standards, collaboration, devices
  oscal.md          the evaluation plan and the evaluation on NIST OSCAL: the concept of step 41
  code_structure.md where everything is in the code, and the rules that keep it in order
  todo.md           the plan of the work, step by step
  history.md        the steps already built, with their decisions
  handover.md       the state of the work, for a new working session
v0.2_functionality.md   the functions of version 0.2 (C), the catalogue the new version re-creates
review.md           the state of the code review
```

Version 0.2, written in C with console shells for UART, Bluetooth and an AI chat, is in the history of git (commit `6cc4091`); the later web interface in commit `acedc8b`.

## License

GNU General Public License, version 2 (`LICENSE`).
