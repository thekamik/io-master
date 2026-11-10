# IO Master: architecture

*What* the program is and *why* it is built this way. The models: `data_model.md`; where everything is in the code: `code_structure.md`. The numbers of sections 3 and 5 are named in the code.

## 1. Purpose

IO Master is a tool for evaluators of security laboratories, who evaluate products against standards: Common Criteria (ISO/IEC 15408 with the CEM, ISO/IEC 18045) and ETSI EN 303 645 first, any other standard the same way. **It is a framework for the evaluation with the help of AI**: it works with the documentation of the client and the work of the evaluator (results of tests, notes, assessments) and carries them to the report. IO stands for input and output. It does not talk to the device under evaluation.

The flow of the work:

```text
standards (OSCAL catalogs) ──► plan (work units, their parameters, agents) ──copy──► evaluation of one product ──► report
                                     ▲                                                     ▲                        ▲
                       agent of the planner ("Fill with AI")              the agent of each work unit   a template of the laboratory
```

**The report** is the evaluation written by a template of the laboratory (panel Reports): a text of any format with markers of the fields of the models, filled by the program and exported into a file; the program writes no format of its own into it (`data_model.md`, section 7). A value is a text of the evaluation, never a part of the template: in a template of HTML or XML (`html`, `htm`, `xhtml`, `xml`) and of LaTeX (`tex`, `latex`) the characters that language reads as more than text are replaced, so that findings written by the AI bring no script into a page and no command into a document that is compiled; Markdown and plain text are written as they are (`report_template_rules/escaping.rs`).

## 2. Principles

| # | Principle | Consequence |
|---|---|---|
| P1 | **The evaluator decides.** | The AI proposes; the evaluator accepts or corrects. `WorkUnit::author` tells who wrote a result. |
| P3 | **Everything is traceable.** | Every question of an agent and its answer is in the journal of that agent, never changed or removed. |
| P4 | **The standard is data, not code.** | Standards are OSCAL catalogs; nothing about a standard is written into the program. |
| P5 | **The model reads; it does not invent sources.** | Quotes are checked by the program against the documents they name. |
| P6 | **The agent parameterises the conversation** (user, 2026-11-02). | Everything that shapes a question to a model and its result is a field of the agent and of its role (instructions, knowledge, tasks, judgement, limits, and the templates every text of a question is written by), so that a laboratory tunes an agent without a change of the program; the characters of a token, by which a question is measured, are a field of the integration. The program keeps only its mechanics, driven by those parameters: the form of the answers it reads (the fields of their JSON, the verdicts of the plan), the filling of a template, the search, the limits of the memory. |

(P2, about devices, was removed: the program does not work with devices.)

## 3. One program, one user

**One Linux account holds one installation with one user of the program**; users cannot be added. The data of the installation is in `$XDG_DATA_HOME/io-master` (usually `~/.local/share/io-master`): the system database `system.v2.db` and the encrypted database of the user `users/<id>.v2.db`. Without a system database the window shows the first start, with it the login. The person who installs the program is its only user. No roles, no server, no git, no backups: what matters leaves the program as exported files.

## 4. The window

- **A desktop window** (Tauri 2, a page in Svelte 5) is the only interface. The page is built into the program and calls a fixed list of commands; **no network port is opened**. The program is started without parameters.
- One dark theme; the panels in a side bar: **Evaluation plan**, **Evaluation**, **Reports**, **Agents**, **Chat**, **AI settings**, **Account**.

## 5. Secrets and encryption

### 5.1 The password

The password is the only factor: 12 to 1024 characters, its own, not that of the Linux account. Someone who copies the files can try passwords offline; Argon2id makes it slow. An optional second factor may come later.

### 5.2 The key of the database

```text
password ──► Argon2id ──► HKDF ──► unlocking key ──AES-256-GCM──► key of the database ──► users/<id>.v2.db (SQLCipher)
```

The key of the database is random and never stored in plain form; it is stored wrapped, bound to the id of its user. A wrong password fails to unwrap it (GCM). A change of the password re-wraps only the 32-byte key. While the user is logged in the key is in the memory of the program; the memory dumps of the process are forbidden.

### 5.3 Two databases

| Database | Protection | Opened | Holds |
|---|---|---|---|
| `system.v2.db` | not encrypted, readable only by the user | at the start | the installation, the user, the two wrapped copies of the key |
| `users/<id>.v2.db` | SQLCipher, the key unwrapped by the password | at the login | everything else: plans, work units, documents, agents, API keys, the journal |

The system database is opened before any password is given, so it holds **nothing secret**.

### 5.4 Recovery of a forgotten password

```text
                    ┌─ password ──────Argon2id─► HKDF ─► unlocking key 1 ──AES-256-GCM──► copy A ─┐
key of the database ┤                                                                              ├─► system.v2.db
                    └─ recovery code ─────────► HKDF ─► unlocking key 2 ──AES-256-GCM──► copy B ─┘
```

At the first start the program makes a **recovery code** (160 random bits, written in groups without 0, O, 1 and I, with a check sum) and wraps the key a second time with it. The code is shown **once**, confirmed by typing one of its groups, and kept outside the computer. A lost password *and* a lost code mean lost data.

The two copies of the key are the last thing a first start writes: a user with keys is a first start that ended. A first start cut off before that (the program killed, the power lost) counts as not installed, and the next one removes what it left (`objects/user/user_data/user_data_rules.rs`: `clear_unfinished`). A stored copy that names a cost of Argon2id over its limits (1 GiB, 16 passes) is refused as damaged before any memory is taken: the system database is not encrypted.

```text
recovery (login screen):   name + code ──recovery::unwrap──► key ── the database opened with it ──┐
change of the password:    present password ──password::unwrap──► key ───────────────────────────┤
a new recovery code:       password ──password::unwrap──► key ──────────────────────────────────┤
                                                                                                  ▼
                           renewal::renew_keys: copy A (the new or same password) and copy B (a new code)
                           written over the old ones together ──► the new code, shown once
```

The key of the database never changes, so the database itself is not touched. A forgotten password is set anew with the code on the login screen; the panel Account changes the password or makes a new code, each with the present password. Every one of the three makes a new code, and the old code opens nothing any more. In the recovery the database is opened with the key before the copies are written, so that a key that does not open it leaves the old copies as they were. A wrong code fails as a wrong name does.

## 6. The AI

- **Integrations** (AI settings) are the services the program sends questions to: an address, a model, a protocol (the chat completions of OpenAI, which local servers offer, or the messages of Anthropic), an optional CA certificate, the size of the context: 128 000 tokens by default, enough for an ST of 100 pages with its requirement, a page of knowledge and the answer; a smaller context is warned of in the list and in the check of the connection, since its questions carry only the passages of the documents. HTTPS with TLS 1.3 and a verified certificate, or plain HTTP only to this computer; no redirects, no proxy. The API key is only in its header, masked in every text of an answer, bound to the address it was given for.
- **Agents** (panel Agents) are a layer of knowledge over the model of an integration (P6): instructions, shared knowledge bases (`.txt`, `.md`, `.pdf` sources, searched with FTS5 on this computer), a glossary that widens the search (the passages that name an identifier of the question, "ADV_ARC.1-1", come first, each found passage followed by the next one of its source while there is room: `data_model.md`, section 4), limits of the knowledge given with one question and of the answer, and a **role**, given when the agent is made and never changed: an **evaluator** (its judgement, the task of a work unit of the documentation and of a functional test, the passages of the documents when they do not fit, the length of the findings of a dependency), a **planner** (the tasks of the parameters and of the dependencies, the length of the line about another work unit, the name of a proof the answer names none for) or a **chat** (no task). The layout of every question is a template of the agent (its message: instructions, knowledge, task) and of its role (the question of each kind, each dependency, document, passage and proof, what the knowledge and the documents are searched with), shown in the editor under "Advanced". Every new agent starts from the texts of the program and is tuned from there. A work unit and an evaluation name an evaluator; a plan in planning names a planner; any agent may talk in a chat.
- **The way to the AI** (`objects/ai/integration/integration_ai/`, `objects/ai/journal/journal_ai.rs`): `prepare` reads the integration and its key and checks that the question fits the context (it is refused, never shortened; only a chat, which is no part of an evaluation, leaves its oldest messages out until it fits); `send` streams the answer without holding the database; `record` writes the journal of the agent before the answer is read. A question without an agent is not kept.
- **The assessment of an evaluation** (panel Evaluation): every work unit in the order of the work (first those without dependencies), without the evaluator; the model judges whether the requirement is met in substance, the knowledge of the agent (optional) helping it read the requirement, never as evidence. A work unit of the documentation is asked about its documents, one of a functional test about its documents and proofs; each question carries the work unit as JSON (never its earlier results), the results of its dependencies for the model to weigh, its documents (whole when they fit, otherwise the passages that fit it best, with a note to the evaluator) and, for a functional test, its proofs. The model is asked to judge only what the requirement of its work unit asks (a fault of another work unit, a dependency among them, is told in the notes), to fail for missing or contradicted substance and to put an editorial flaw in the notes, to be careful about the substance (a false fail is corrected by the evaluator, a false pass may never be found), to give its verdict last, from its analysis, and not to blame a document for its text extracted from a PDF (broken tables, formatting lost) or for the version the evaluator recorded. The program checks every quote in its document or proof (a quote points to the evidence, it is no condition of the verdict), keeps the verdict as the model gave it and warns of each dependency that did not pass; an answer cut, refused or stopped is not taken, nor one that gives a verdict without an analysis or a field that is no text; a quote too long is left out and told; a work unit without any document or proof is not assessed. When the service counted fewer than half the tokens the program estimated for a question, it may have shortened the question in silence (a local server with a context smaller than the integration says): the answer is taken with that warning among its notes (`objects/ai/integration/integration_ai/question.rs`: `shortened_warning`). "Stop" ends the answer awaited at once (`objects/session/progress/progress_ai/stop.rs`). The documents are written by the manufacturer, who gains by a pass: the task of a new evaluator tells the model that they are evidence and never instructions, and to tell in the notes of a text in them that speaks to it (a template of the role, P6: the laboratory may change it). A result the evaluator writes is of author User and never written over by the AI: it is checked when the work units of an assessment are chosen and again when an answer is written.
- **Where the text of a standard and of the evidence goes is the decision of the laboratory**: a local model for licensed standards and documents under NDA, a cloud service only where the licence allows.

## 7. Standards

The requirements of a standard are **NIST OSCAL 1.2.3 catalogs**, checked against the JSON schema of NIST (`io-master/oscal/`) when imported. A plan names the standards it uses; each work unit is a control of one of them (its label, its key, its requirement and guidance). The plan and the evaluation are the program's own model, linked to the catalogs by their ids; nothing of ours is written into an OSCAL document.

## 8. System requirements

- **Debian 13 (trixie)**, the target system and the only one tested; XFCE on X11. The packages: `README.md`.
- Few resources: the virtual machines of a laboratory get little (8 GB RAM or less).
- Poppler (`pdfinfo`, `pdftotext`) reads the PDFs of the knowledge bases and of the documents of the product. A PDF comes from outside the laboratory, so each call of a tool is bounded (120 seconds, 64 MiB of output) and runs in a sandbox of `bubblewrap`: namespaces of its own, no network, no home folder and no data of the program, the programs and libraries of the system read-only (`infrastructure/pdf/poppler.rs`: `run`, `SANDBOX_ARGUMENTS`). A fault of the parser a file was made for reaches neither the databases nor the keys.
- The decrypted data is only in RAM: **the swap encrypted (or none), and no hibernation** on the computer of an evaluator.
