# IO Master: architecture

*What* the program is and *why* it is built this way. The models: `data_model.md`; where everything is in the code: `code_structure.md`. The numbers of sections 3 and 5 are named in the code.

## 1. Purpose

IO Master is a tool for evaluators of security laboratories, who evaluate products against standards: Common Criteria (ISO/IEC 15408 with the CEM, ISO/IEC 18045) and ETSI EN 303 645 first, any other standard the same way. **It is a framework for the evaluation with the help of AI**: it works with the documentation of the client and the work of the evaluator (results of tests, notes, assessments) and carries them to the report. IO stands for input and output. It does not talk to the device under evaluation.

The flow of the work:

```text
standards (OSCAL catalogs) ──► plan (work units, their parameters, agents) ──copy──► evaluation of one product ──► report
                                     ▲                                                     ▲
                       agent of the planner ("Fill with AI")              the agent of each work unit
```

## 2. Principles

| # | Principle | Consequence |
|---|---|---|
| P1 | **The evaluator decides.** | The AI proposes; the evaluator accepts or corrects. `WorkUnit::author` tells who wrote a result. |
| P3 | **Everything is traceable.** | Every question of an agent and its answer is in the journal of that agent, never changed or removed. |
| P4 | **The standard is data, not code.** | Standards are OSCAL catalogs; nothing about a standard is written into the program. |
| P5 | **The model reads; it does not invent sources.** | Quotes are checked by the program against the documents they name. |

(P2, about devices, was removed: the program does not work with devices.)

## 3. One program, one user

**One Linux account holds one installation with one user of the program**; users cannot be added. The data of the installation is in `$XDG_DATA_HOME/io-master` (usually `~/.local/share/io-master`): the system database `system.v2.db` and the encrypted database of the user `users/<id>.v2.db`. Without a system database the window shows the first start, with it the login. The person who installs the program is its only user. No roles, no server, no git, no backups: what matters leaves the program as exported files.

## 4. The window

- **A desktop window** (Tauri 2, a page in Svelte 5) is the only interface. The page is built into the program and calls a fixed list of commands; **no network port is opened**. The program is started without parameters.
- One dark theme; the panels in a side bar: **Evaluation plan**, **Evaluation**, **Agents**, **Chat**, **AI settings**.

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

At the first start the program makes a **recovery code** (160 random bits, written in groups without 0, O, 1 and I, with a check sum) and wraps the key a second time with it. The code is shown **once**, confirmed by typing one of its groups, and kept outside the computer. A lost password *and* a lost code mean lost data. (The login with the code and the change of the password come back later.)

## 6. The AI

- **Integrations** (AI settings) are the services the program sends questions to: an address, a model, a protocol (the chat completions of OpenAI, which local servers offer, or the messages of Anthropic), an optional CA certificate, the size of the context. HTTPS with TLS 1.3 and a verified certificate, or plain HTTP only to this computer; no redirects, no proxy. The API key is only in its header, masked in every text of an answer, bound to the address it was given for.
- **Agents** (panel Agents) are a layer of knowledge over the model of an integration: instructions, shared knowledge bases (`.txt`, `.md`, `.pdf` sources, searched with FTS5 on this computer), a glossary that widens the search, limits of the knowledge given with one question. A work unit names its agent; a plan names its agent: in planning the agent of the planner, in an evaluation the agent of the work units without one of their own.
- **The way to the AI** (`ai/ask.rs`): `prepare` reads the integration and its key and checks that the question fits the context (it is refused, never shortened; only a chat, which is no part of an evaluation, leaves its oldest messages out until it fits); `send` streams the answer without holding the database; `record` writes the journal of the agent before the answer is read. A question without an agent is not kept.
- **The assessment of an evaluation** (panel Evaluation): every work unit in the order of the work (first those without dependencies), without the evaluator; each question carries the work unit as JSON, the results of its dependencies, its documents (whole when they fit, otherwise the passages that fit it best) and its proofs. The program checks every quote in its document and makes the verdict the worst of its own and those of the dependencies. A result the evaluator writes is of author User and never written over by the AI.
- **Where the text of a standard and of the evidence goes is the decision of the laboratory**: a local model for licensed standards and documents under NDA, a cloud service only where the licence allows.

## 7. Standards

The requirements of a standard are **NIST OSCAL 1.2.3 catalogs**, checked against the JSON schema of NIST (`io-master/oscal/`) when imported. A plan names the standards it uses; each work unit is a control of one of them (its label, its key, its requirement and guidance). The plan and the evaluation are the program's own model, linked to the catalogs by their ids; nothing of ours is written into an OSCAL document.

## 8. System requirements

- **Debian 13 (trixie)**, the target system and the only one tested; XFCE on X11. The packages: `README.md`.
- Few resources: the virtual machines of a laboratory get little (8 GB RAM or less).
- Poppler (`pdfinfo`, `pdftotext`) reads the PDFs of the knowledge bases.
- The decrypted data is only in RAM: **the swap encrypted (or none), and no hibernation** on the computer of an evaluator.
