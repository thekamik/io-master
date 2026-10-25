# IO Master: the plan of the work

**A heap of reminders, not a plan in order** (user, 2026-10-21): the user chooses what is done next. The number of a step is its name and never changes, so that references in the code (`TODO (step 12 of docs/todo.md)`) stay valid. **The steps already built, dropped or replaced are in `history.md`**, with their decisions; this file keeps only what is open.

**Where the program is (2026-10-25):** steps 0 to 53 are built or settled (`history.md`). IO Master is a framework for the evaluation with AI (step 48): the Converter makes catalogs of OSCAL from the PDFs of a standard (step 50), the Evaluation plan makes master projects with a scheme per task and exports work packages (step 51), the Evaluation lab carries a package out for one product and exports the report (step 52). The program is experimental: the data of the tests is a temporary resource until version 1 (`CLAUDE.md`).

**The workflow agreed on 2026-10-25** (user): steps 57, 58, 59 and 60 in this order, each stage approved before the next; the changes go to `test_branch` only. Settled the same day and moved to `history.md`: step 53 (zero warnings; eslint dropped), step 55 (backups dropped: the export of the work is the backup).

| Step | What | State |
|---|---|---|
| 12 | Review, order and functional hardening of the whole code | the user's review, file by file (`review.md`); open |
| 10 | Before the release | open |
| 54 | The removal of the installation in the window | open |
| 56 | The rule of the comments for the whole code | open |
| 57 | Knowledge for the AI (RAG): the view `knowledge`, agents with a knowledge base, the agent of the requirement | concept approved 2026-10-25; 57.1 and 57.2 built |
| 58 | The journal of the AI on a page | after 57 |
| 59 | The progress of an evaluation | after 58 |
| 60 | A short presentation (5 minutes) with screenshots | after 59, the end of this workflow |
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
- [ ] `cargo-audit` and `cargo-deny` on the dependencies (user, 2026-10-25: not critical before the release; what matters is code that is readable and free of logical errors).
- [ ] The installation package for Debian 13 (the program, its desktop entry, its dependencies: Poppler, WebKitGTK).
- [ ] Version 1: from then on the data is kept, and every change of the layout of a database is a migration (`db/user/migrations.rs`).
- [x] **Paste on Wayland (found 2026-10-18; gone the same day, a fault of the unstable system of the time, not of the program):** on KDE Plasma (Wayland, WebKitGTK 2.54.0) the program ended with SIGSEGV when the clipboard was emptied by a clipboard cleaner while it was being pasted; the fault was inside WebKitGTK. Should it come back: the page catches every paste, the program reads the clipboard itself through GTK and says "Nothing was pasted: copy the text again" instead of ending.

---

## Step 54: The removal of the installation in the window

The removal is done in the window only (`architecture.md`, section 3.2): after the login, the password and a typed confirmation, it removes the databases of the installation. Its functions are written anew for it: the console version was removed on 2026-10-24, and the unused `lifecycle::uninstall` and `user_db::remove_all` in step 53.

---

## Step 56: The rule of the comments for the whole code

`CLAUDE.md`, "Where the comments are": at every function what it does, where it is called from and which functions it calls; the architecture in `docs/`. The entries of the views and components and the code of steps 49 to 52 follow it; the other layers get it when a file is rebuilt or reviewed (step 12), or in this step if the user asks for it.

---

## Step 57: Knowledge for the AI (RAG)

**State: concept approved by the user on 2026-10-25 ("działamy"), with the answers below; built stage by stage, each approved before the next.** The user's idea (2026-10-24 and 25): the agent of the requirement should understand the context of the standard; a view of its own lets the evaluator make an agent with knowledge (for example of Common Criteria) and attach it to the analysis of a requirement.

**The answers of the user (2026-10-25):**
- The view is `knowledge`, "Knowledge (RAG)" in the navigation.
- **Every place that uses the AI offers "use RAG"**: an agent with knowledge can be attached to the chat, to the requirement, to the input (its chat and its fragments) and the other works of the AI.
- **Many agents**, for different standards and tasks; each is **exported and imported as a file of its own** (its instructions, glossary and knowledge bases). Not the whole database: single projects and single agents are exported, so a file taken by someone holds less data and a leak is smaller.

**What RAG is here.** *Retrieval-augmented generation*: the model is not trained; at the moment of a question the program finds the passages of the knowledge base that fit it and puts them into the question, marked as context, with their source and page. "Tuning" an agent means choosing its sources, its instructions and its glossary and trying it on questions, never changing the weights of a model (that needs GPUs and data and could not be audited).

```text
view knowledge                                       Evaluation lab
──────────────                                       ──────────────
sources (PDF, catalog, note) ──► passages ──► index   requirement block
                                 (page, ≤ N chars)    agent chosen ──► retrieval ──► passages shown,
agent: instructions + bases + glossary + limits                        the evaluator may untick them
proving ground: a question ──► passages with scores ──► answer, quotes of the standard checked
```

**Stages (each approved before the next):**
- [x] **57.1 The whole requirement in the question** (no new view; built 2026-10-25: `ai/tasks/requirement_context.rs`, `Catalog::titles_above`, `lab_task/read.rs`; every work of the AI of a task gets where the requirement stands, its parameters and links, the assessment also its related elements up to 24 000 characters): the parameters (assignments and selections), the guidance and the application notes of the control, its parents in the catalog, and the text of its related elements (`lab_related`: the work units of the CEM) given to the agent of the requirement; today it gets the title and prose of the parts only.
- [x] **57.2 The view `knowledge`, the knowledge bases** (built 2026-10-25: `model/knowledge.rs`, `db/knowledge/`, `views/knowledge/`, the page `ui/src/views/knowledge/`; the stems of English words found by the tokenizer `porter`): a base has a name and sources: a PDF (read page by page by Poppler, as in the Converter), a catalog of OSCAL of the installation, a note of the laboratory (an interpretation, a supporting document of the scheme). Every source is cut into passages of a bounded size, by page and paragraph, each with its source and page, and indexed by FTS5 of SQLite (BM25; built into the SQLCipher of the program, no new dependency). A search field shows what the index finds.
- [ ] **57.3 Agents and the proving ground:** an agent has a name, instructions (who it is, how it answers), its knowledge bases, a glossary (terms and their synonyms, e.g. "TSF" = "TOE Security Functionality", used to widen the search) and limits (number of passages, characters). The proving ground: a question, the passages found with their scores, the answer of the model with its quotes of the knowledge checked in the passages (P5). The evaluator tunes the agent there.
- [ ] **57.4 The agent attached to the requirement** (the first place; the others in 57.6): in the Evaluation lab the evaluator chooses an agent for an evaluation (or none) and may change it for a task; before the question the panel of the requirement lists the passages found (query: the requirement and what the planner asks to check), the evaluator may untick some; the question holds them in a section of their own, "Context of the standard: to understand the requirement, not evidence"; the verdict still rests on the accepted fragments only; the journal keeps the passages with the question (P3). The knowledge stays in the installation that made it: a work package carries no base (licensed standards).
- [ ] **57.5 (only if 57.3 shows that the search by words is not enough) Embeddings:** the vectors of the passages from the endpoint of embeddings of the connection (a local model), kept in SQLite, compared in Rust within a bounded number; joined with FTS5 (hybrid search).

- [ ] **57.6 Every work of the AI may use an agent:** the chat, the chat and the fragments of an input, the works of the AI of the Evaluation plan and of the Converter; the agent chosen where the work is started, the passages shown and written into the journal as in 57.4.
- [ ] **57.7 Export and import of an agent:** one file per agent (its instructions, glossary, limits and the passages of its bases with their sources), imported into another installation; the window says that the file holds the text of the sources (licensed standards).

---

## Step 58: The journal of the AI on a page

The journal is written by `ai/api/` and read today only by the tests (`db/ai/journal/read.rs::last`). A page (in Settings or of its own) lists the questions and answers: when, the task, the role, the model, the size, and with step 57 the passages of the knowledge given. Open: how long the entries are kept (`db/ai/journal/schema.rs`).

---

## Step 59: The progress of an evaluation

In the Evaluation lab, a summary over the tasks of an evaluation: how many have a verdict (met, not met, inconclusive), how many wait, how many outputs are accepted and ready for the report.

---

## Step 60: A short presentation

`presentation.md` made a presentation of about five minutes (user, 2026-10-25: the present one is too long), with screenshots of the window taken on a private display; the knowledge of the AI (RAG) shown. No screenshots in the README.

---

## Reminders without a step yet

- **The Evaluation lab:** other forms of evidence (images, office documents, archives); the import of a new version of a work package into an evaluation begun.
- **The Evaluation plan:** a manual addition of a related element (from the browser of the standards), besides the AI.
- **OSCAL beyond catalogs:** the plan and the results are kept in the program's own model (`oscal.md`); an export of an evaluation as OSCAL assessment results, and of a master project as a profile and an assessment plan, when a laboratory or a scheme asks for it.
- **The login:** an optional second factor (`architecture.md`, section 5.1).
- **Few resources** (`architecture.md`, section 9): the program measured on a virtual machine of 8 GB RAM with a large catalog (the CEM, 813 work units) and a large PDF.
