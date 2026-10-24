---
marp: true
title: IO Master — an overview of the functions
---

# IO Master

**A desktop framework for security evaluations with the help of AI**

The state of 2026-10-24 (early development, no release yet)

- From the PDF of a standard to the report of an evaluation, in one window
- Common Criteria (ISO/IEC 15408, CEM / ISO/IEC 18045) and ETSI EN 303 645 first; any standard by the same code
- The AI reads and proposes; **the evaluator decides**

<!-- Slides separated by "---": readable as a document, or shown with Marp. -->

---

## The problem it addresses

An evaluation is a chain of documents that lose their link to each other:

```text
standard (PDF) → evaluation plan → evidence of the developer → assessment → report (template of the lab)
```

- The requirements live in PDFs; the plan in spreadsheets; the evidence in folders; the assessment in documents written by hand
- An AI can speed up the reading, but an assessment that rests on what a model *said* does not stand an audit
- **IO Master keeps the chain traceable**: every requirement is a structured object, every proposal of the AI points to the exact text it rests on, and nothing becomes a result without the evaluator

---

## Five principles (requirements of the design)

| | Principle | In practice |
|---|---|---|
| P1 | **The evaluator decides** | The AI proposes requirements, schemes, fragments, results, verdicts, texts; each waits for an explicit accept, correct or reject |
| P3 | **Everything is traceable** | Every question to a model and its answer go to the journal of the AI, written by the one gate to the AI |
| P4 | **The standard is data, not code** | Standards are catalogs of NIST OSCAL; no standard is written into the program |
| P5 | **The model reads; it does not invent sources** | Every quote the AI gives is looked for by the program in the page or the evidence it names; a quote not found is marked so |
| — | **No devices** | The program does not talk to the product; results of tests on it enter as evidence |

---

## The flow, end to end

```text
 Converter            Evaluation plan                         Evaluation lab
 ─────────            ───────────────                         ──────────────
 PDFs of a  ──AI──►  CATALOG  ──►  master project  ──export──►  evaluation of  ──►  report
 standard            (OSCAL)       tasks + schemes   work        one product        (.txt/.tex
                       ▲           related elements  package     evidence           per output)
         imported ─────┘           work packages     (.json +    fragments
         (SP 800-53, CEM, PPs)                       catalogs)   results, verdict
```

- Three views, three roles: **converting** a standard, **planning** its evaluation, **carrying it out**
- The planner and the evaluator may be different people on different computers: the work package is the hand-over

---

## 1. Converter: a standard becomes a catalog of OSCAL

A guided conversion, step by step, every stage confirmed and resumable:

1. **Sources**: the PDFs of the standard, read page by page (Poppler), searchable, shown in a built-in viewer
2. **Table of contents**: the AI finds its pages; the evaluator confirms them in the viewer
3. **Groups** (classes, families, sections): proposed by the AI from the table of contents; the program locates where each group starts by its heading in the text, never by the page numbers a model claims
4. **Controls**, group by group: proposed with their statements, parameters (CC assignments and selections) and **quotes checked on their pages**
5. **Review**: what is left (quotes not found, empty groups, undecided proposals), then "finished"

A manual editor (the tree, the JSON of every object, its quotes) is always available. Every document is checked against the **JSON schemas of NIST OSCAL 1.2.3** at every write.

---

## 2. Evaluation plan: the master project

One **master project per version of a standard**, made once and reused:

- **Tasks**: one per requirement (a control of a catalog), chosen in a browser of the standards (tree and full-text search) — or **proposed by the AI** from the scope of the project, requirement by requirement, each with its reason
- **Requirements of the laboratory**: own requirements outside the standards, kept in one shared catalog of OSCAL, under identifiers the planner chooses
- **Several catalogs per project**: the criteria, the methodology, the Protection Profiles
- **Related elements**: the AI links a requirement to the elements of the other catalogs of the project — e.g. a CC Part 3 component to its **CEM work units** — each accepted by the planner

---

## 2. Evaluation plan: the scheme of a task

Every task has a **scheme**, drawn as connected blocks:

```text
 input  ──┐                                ┌──►  output: pETR          (always, LaTeX)
 input  ──┼──►  REQUIREMENT  ──────────────┤
 input  ──┘     agent of the AI            └──►  output: Observation Report  (only when not met)
                four results + verdict
```

- **Input**: the evidence the evaluator must deliver, and what its agent checks before the evidence is ready
- **Requirement**: what its agent checks and how it decides; the **four results** (Findings · Analysis · Assessment and verdict proposition · Vulnerabilities and notes) and a verdict
- **Output**: a target document, its format (text or LaTeX) and its **condition**: always, only when met, only when not met
- **Standard outputs**: a template of the outputs, set once per project (e.g. pETR + OR), copied into every new task and editable per task
- **The AI proposes** missing inputs and corrections of the blocks (dashed until accepted); the standard text stays beside the drawing

---

## 2. Evaluation plan: work packages

- The planner **delegates tasks to the evaluators** in work packages (a task in one package at most)
- A package is **exported as one file**: our format (the project, the package, every task with its *accepted* scheme and related elements) and, **attached, the catalogs of OSCAL** it points to, whole and unchanged
- A picture of the state at the moment of the export: the evaluator's work never changes the plan

The documents of OSCAL themselves (catalogs, mappings, profiles, assessment plans, assessment results) are imported, validated, kept and exported in the same view.

---

## 3. Evaluation lab: one product, one requirement at a time

The evaluator imports a work package for a product (TOE, version). The package is a **frozen copy**; the catalogs attached are validated and stored.

For each task, the scheme is shown read only, the requirement beside it, and each block opens its panel:

| Block | What the evaluator does | What the AI does |
|---|---|---|
| **Input** | delivers evidence: a PDF, a text file, pasted notes (interview, tool output) — kept **exactly as delivered** | answers questions about whether the evidence is ready (chat); proposes **fragments** |
| **Requirement** | edits the four results, **sets the verdict** (met · not met · inconclusive) | its agent assesses **only the accepted fragments** and proposes the results and a verdict with its reason |
| **Output** | accepts or corrects the text | drafts the text from the results, as the output asks (format, target, template instructions) |

---

## 3. Evaluation lab: the two phases (an audit-ready method)

```text
 evidence (raw, kept)                         phase 1                      phase 2
 ─────────────────────                        ───────                      ───────
 ST.pdf, guidance, test logs, notes  ──AI──►  fragments                ──►  agent of the requirement
                                              exact quote + page           reads ONLY accepted fragments
                                              found / NOT found            proposes 4 results + verdict
                                              AI note (never a quote)      evaluator decides
                                              accepted by the evaluator
```

- **A fragment is an exact quote**, looked for by the program in the text of the evidence (white space normalised, letters exact); the explanation of the model is kept apart as an *AI note*
- **Only accepted fragments reach the assessment**: the result rests on text the evaluator saw and approved, not on what the model claims to have read
- The evaluator may add fragments by hand; the program marks whether their quote is found

---

## 3. Evaluation lab: outputs and the report

- An output is written **only when its condition holds for the verdict**: the Observation Report only for "not met"; while the verdict is inconclusive, conditional outputs wait
- **The report**: one file per output (e.g. `pETR.tex`, `Observation Report.txt`), the accepted texts **task by task in the order of the package**, with a header (project, package, evaluator, product)
- LaTeX outputs are bodies for the template of the laboratory, not standalone documents
- **Nothing returns to the planner**: the result of the evaluation is the report, exported out of the program

---

## The AI: one governed gate

- **No provider is built in**: a connection is described completely in the window (endpoint, model, headers, request template, where the answer is) — meant first for a **local model of the laboratory**; presets for known services
- **Every question goes through one gate** (`ai/api/`), in a **task** and a **role** (text analysis · summary · consultation · formal analysis), and is **written into the journal** by the gate itself — no caller can skip it
- **Instructions of every task are editable** by the laboratory (and restorable to the defaults)
- **Inputs are bounded**: long documents are cut into parts of a bounded size by whole pages; a requirement is never cut; quotes of a standard are never shortened
- **Where text goes is the laboratory's decision**: the program sends it over the chosen connection; a loopback model never leaves the computer (recommended for licensed standards and NDA documents)

---

## Security model

- **One Linux account, one installation, one user**; a desktop window only — **no network port is opened**, no console interface
- **The database of the evaluator is encrypted** (SQLCipher); its random 256-bit key is wrapped with the password (**Argon2id → HKDF → AES-256-GCM**); the system database holds nothing secret
- **Recovery code** (160 bits, shown once, BitLocker/LUKS-style second key slot): a new code after every change of the password; the old one stops working
- **TLS 1.3 with a verified certificate** for every connection leaving the computer; plain HTTP only to the loopback; API keys bound to the address they were entered for, never shown back
- Memory: the key wiped after opening; SQLCipher locks and wipes its pages; core dumps of the process forbidden
- No TPM, no keyring, no master server: runs the same on a laptop and on a virtual machine of the laboratory

---

## Standards as data: NIST OSCAL

- **Catalogs of OSCAL 1.2.3** are the working model of the requirements, validated against the **schemas of NIST** at every write
- CC classes / families / components → groups and controls; assignments and selections → parameters; CEM work units → a catalog of their own, linked as related elements
- What OSCAL has no place for — schemes of tasks, agents' instructions, work packages, raw evidence and fragments — is **the program's own model**, linked to OSCAL only by UUIDs and ids
- **Nothing of ours is written into an OSCAL document**: every export is valid OSCAL
- An export of master projects and evaluations as OSCAL profiles, assessment plans and assessment results is on the list, when a laboratory or a scheme asks for it

---

## Engineering: built to be audited

- **Rust (Tauri 2) + Svelte 5**; the page and the program are two projects joined by a fixed list of commands, each listed in four places that a test compares
- **Layers with a tested direction** (`model → db → ai/installation → components → views`): a test fails on a dependency against the direction, and on a view that uses another view
- **All SQL in one layer**, as CRUD per area; all encryption in one folder; all AI behind one gate
- **"The Power of 10" (NASA/JPL) adapted to Rust**: no recursion, bounded loops and inputs, no `unwrap`/`panic` outside tests, checks that return errors, functions under 60 lines, no `unsafe`
- **455 automated tests** (unit, database, commands called as the page calls them, the AI against a fake service); every command of the window drawn in a flow of data in the documentation; window tests on a private display

---

## Where it stands, what comes next

**Built and tried** (2026-10-24): installation and recovery · AI connections and chat · Converter · Evaluation plan with master projects, schemes, AI proposals, work packages · Evaluation lab with evidence, fragments, assessment, outputs, report

**Next** (chosen by the project owner):
- The owner's code review, file by file, and functional hardening (login limits, permissions of the data folder, behaviour on damaged or locked databases)
- Zero compiler warnings; removal of the installation from the window; backups optionally encrypted with OpenPGP
- A viewer of the evidence, more evidence formats, OSCAL export of plans and results
- A Debian 13 package and version 1, from which the data is kept across versions

**Limits today**: experimental, single-user, Debian 13 only; the data of test installations is not migrated between development versions
