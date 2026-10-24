# IO Master: the plan of the work

**A heap of reminders, not a plan in order** (user, 2026-10-21): the user chooses what is done next. The number of a step is its name and never changes, so that references in the code (`TODO (step 12 of docs/todo.md)`) stay valid. **The steps already built, dropped or replaced are in `history.md`**, with their decisions; this file keeps only what is open.

**Where the program is (2026-10-24):** steps 0 to 52 are built or settled (`history.md`). IO Master is a framework for the evaluation with AI (step 48): the Converter makes catalogs of OSCAL from the PDFs of a standard (step 50), the Evaluation plan makes master projects with a scheme per task and exports work packages (step 51), the Evaluation lab carries a package out for one product and exports the report (step 52). The program is experimental: the data of the tests is a temporary resource until version 1 (`CLAUDE.md`).

| Step | What | State |
|---|---|---|
| 12 | Review, order and functional hardening of the whole code | the user's review, file by file (`review.md`); open |
| 10 | Before the release | open |
| 53 | Zero warnings: the unused code removed; eslint on the page | eslint open |
| 54 | The removal of the installation in the window | open |
| 55 | Backups, optionally encrypted with OpenPGP | open (`architecture.md`, section 5.5) |
| 56 | The rule of the comments for the whole code | open |
| — | Reminders without a step yet | below |

## How every step is done

1. **Concept.** Claude proposes how the step is built (files, dependencies, open decisions). The user approves or corrects it. No code before that.
2. **Implementation** in small parts, each with its unit tests (in the file it tests, its last item `#[cfg(test)] mod tests { ... }`), in the layers of `code_structure.md`.
3. **Check.** `cargo fmt --check` clean, builds without warnings, all tests (also of the architecture and of the command lists), `npm run check` and `clippy` pass; the window tried on a private display; the user tries the result.
4. **Approval** by the user, then a commit to `test_branch`; the tested and approved state goes to `main` after the user's manual tests (`CLAUDE.md`, "Branches").

Legend: `[ ]` to do, `[x]` done.

---

## Step 12: Review, order and functional hardening of the whole code

The whole code is gone through once, module by module, so that it matches `architecture.md` and `CLAUDE.md` and stays easy to keep an overview of. No new functions in this step.

**How**
- The review is led by the user: the user goes through the files in an order of their choice and asks questions. Claude answers, explains the code and its reasons, and proposes a change where a question shows a problem (`CLAUDE.md`, "Code review"). The user knows C and is new to Rust: Rust is explained through analogies with C.
- Claude does not review the architecture on its own initiative. Changes are made only when the user asks for them or approves a proposal; each change with its tests.
- The record of what is approved: `review.md`.

**What is checked in every module**
- [ ] Agreement with the concept: no remains of earlier concepts (comments, names, code paths, documentation of files that no longer exist).
- [ ] Dead code (step 53); one responsibility per file; repeated code moved to one place; files that grew too large split.
- [ ] The rules of `CLAUDE.md` (The Power of 10 for Rust): bounded loops and inputs, `ensure!` with recovery, every result handled, no closures as parameters of our functions.
- [ ] Errors: every failure reaches the user as a clear message; nothing panics on input from outside; no secret in any message.
- [ ] Tests: missing cases, tests that test nothing, the same test helper written twice.
- [ ] Comments: what a function does, where it is called from, what it calls (step 56).

**Functional hardening (candidates, settled in the concept of each module)**
- [ ] Login: a limit on failed attempts, without telling which names exist; a delay after a failure.
- [ ] The data directory: permissions (`0700`, files `0600`) checked at the start; a clear message when they are wider or the owner differs.
- [ ] Databases: behaviour on a full disk, a damaged file, a database locked by a second running program (one program per data directory).
- [ ] The limits of the inputs from files (a work package, a document of OSCAL, a PDF, evidence) measured on real documents of the laboratories.

**Done when:** every module has been reviewed and approved, the documentation describes the code as it is, and build, tests and clippy pass.

---

## Step 10: Before the release

- [ ] Security review of the whole program, with the findings of step 12.
- [ ] The installation package for Debian 13 (the program, its desktop entry, its dependencies: Poppler, WebKitGTK).
- [ ] Version 1: from then on the data is kept, and every change of the layout of a database is a migration (`db/user/migrations.rs`).
- [x] **Paste on Wayland (found 2026-10-18; gone the same day, a fault of the unstable system of the time, not of the program):** on KDE Plasma (Wayland, WebKitGTK 2.54.0) the program ended with SIGSEGV when the clipboard was emptied by a clipboard cleaner while it was being pasted; the fault was inside WebKitGTK. Should it come back: the page catches every paste, the program reads the clipboard itself through GTK and says "Nothing was pasted: copy the text again" instead of ending.

---

## Step 53: Zero warnings

Rule 10 of `CLAUDE.md`: the code compiles without warnings and clippy is clean. The unused code of earlier steps was removed on 2026-10-24 ("nieużywane funkcje usuwamy"): the forms of the answer and the rules of memory of the roles (`model/ai.rs`), the fields and objects no rule names (`model/text.rs`), the connection of its own for a role (`db/ai/roles/`, removed: every role uses the active connection), the removal of the installation (`lifecycle::uninstall`, `user_db::remove_all`, written anew in step 54). Kept only for the tests, under `#[cfg(test)]`: `db/system/read.rs::user_names` and the reading of the journal of the AI (`db/ai/journal/read.rs::last`), until a page of the journal exists.
- [x] rustc and `cargo clippy --all-targets`: zero warnings.
- [ ] eslint on the page (`CLAUDE.md`, rule 10 for the page): only svelte-check runs today.

---

## Step 54: The removal of the installation in the window

The removal is done in the window only (`architecture.md`, section 3.2): after the login, the password and a typed confirmation, it removes the databases of the installation. Its functions are written anew for it: the console version was removed on 2026-10-24, and the unused `lifecycle::uninstall` and `user_db::remove_all` in step 53.

---

## Step 55: Backups

The evaluator exports backups of the work (also the only way to move it to a new computer); **a backup may be encrypted with an OpenPGP key, optionally** (user, 2026-10-24): without a key the backup is not encrypted, and the window says so and recommends a key. Open: whether a backup without encryption leaves out the API keys. `architecture.md`, section 5.5.

---

## Step 56: The rule of the comments for the whole code

`CLAUDE.md`, "Where the comments are": at every function what it does, where it is called from and which functions it calls; the architecture in `docs/`. The entries of the views and components and the code of steps 49 to 52 follow it; the other layers get it when a file is rebuilt or reviewed (step 12), or in this step if the user asks for it.

---

## Reminders without a step yet

- **The Evaluation lab:** a viewer of the evidence (the PDF on its page, as the viewer of a standard, the quote of a fragment highlighted); other forms of evidence (images, office documents, archives); the import of a new version of a work package into an evaluation begun.
- **The Evaluation plan:** a manual addition of a related element (from the browser of the standards), besides the AI.
- **OSCAL beyond catalogs:** the plan and the results are kept in the program's own model (`oscal.md`); an export of an evaluation as OSCAL assessment results, and of a master project as a profile and an assessment plan, when a laboratory or a scheme asks for it.
- **The login:** an optional second factor (`architecture.md`, section 5.1).
- **The journal of the AI:** how long its entries are kept (they go only with the whole database today, `db/ai/journal/schema.rs`), and a page to read it (its reading is kept for the tests only, step 53).
- **Few resources** (`architecture.md`, section 9): the program measured on a virtual machine of 8 GB RAM with a large catalog (the CEM, 813 work units) and a large PDF.
