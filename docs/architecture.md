# IO Master: architecture

This document records the concept of the program, as agreed from 2026-10-14 on (one user per installation on 2026-10-15; the desktop window on 2026-10-16; the layers of the code on 2026-10-17; the standards in NIST OSCAL on 2026-10-18; the framework for the evaluation with AI, without devices, a master, the TPM and the keyring, on 2026-10-24, step 48 of `todo.md`). It describes *what* the program is and *why* it is built this way. *Where* everything is in the code is in `code_structure.md`; the work with standards on NIST OSCAL in `oscal.md`; the functions of v0.2 that are being re-created are listed in `v0.2_functionality.md`.

Status markers: **agreed** is decided; **open** still needs a decision; **TODO** is known work that must not be forgotten.

**The vocabulary of the project is in section 10, the tasks of the folders in section 11, the flow of data in the views in section 12.** They are at the end so that the numbers of the earlier sections, named all over the code and the documents, stay as they are.

---

## 1. Purpose

IO Master is a tool for evaluators of security laboratories, who evaluate products against standards: first of all Common Criteria (ISO/IEC 15408, CEM ISO/IEC 18045) and ETSI EN 303 645, and any other standard in the same way. The requirements of a standard, the plan of their evaluation and its results are kept in **NIST OSCAL** (section 7.1). The program brings together the evaluation plan and the evaluation with AI assistance, and the record of everything that was done. **It is a framework for the evaluation with the help of AI and nothing else** (user, 2026-10-24, step 48): it works with the documentation of the client and with the work of the evaluator, and it does not talk to the device under evaluation.

**The name.** IO stands for input and output: the inputs are the documentation of the client and the results of the work of the evaluator (tests, notes, assessments), the output is the report. The flow of the data from the input to the output is what the program does, and it has to do it masterfully.

The program is installed **locally, on the computer of the evaluator**, in their own Linux account: **one Linux account holds one installation with one user of the program** (section 3). It is used through a **desktop window** (section 4). There is no console interface.

## 2. Principles

Carried over from the design of v0.2; they are requirements, not preferences.

| # | Principle | Consequence |
|---|---|---|
| P1 | **The evaluator decides.** | The program never issues a verdict. It offers an estimate with its reasons. |
| P2 | *Removed (user, 2026-10-24, step 48).* It read: no command reaches a device without an action of the evaluator. The program no longer works with devices; that the AI only proposes and the evaluator decides is P1. The number stays, so that P3 to P5 keep theirs. | |
| P3 | **Everything is traceable.** | Every passage used, every proposal of the model, and every approval is recorded. |
| P4 | **The norm is data, not code.** | Every standard is kept as OSCAL documents (catalogs, profiles, mappings), made or loaded; nothing about a standard is written into the program (section 7.1). |
| P5 | **The model reads; it does not invent sources.** | Passages of the norm are found by the identifiers of their OSCAL objects and the quotes kept with them; every quote the AI proposes is checked on its page. The model works on the text it is given. |

## 3. One program, one user (agreed; changed 2026-10-24)

There is one program, installed on the computer of the evaluator. **One Linux account, one installation, one user** (agreed 2026-10-15): an installation belongs to the Linux account that installed it, and it has exactly one user of the program; users cannot be added. The data of a laboratory is critical, so the installation is bound as tightly as possible to one person and one account. Evaluators share their results by the files the program exports (the report in the LaTeX template of the laboratory, step 28; backups, section 5.5), as the laboratory organises it (section 6).

**No roles** (user, 2026-10-24, step 48): until then the first start chose between a *workstation* and a *master*, a separate machine that served the reset of forgotten passwords to the workstations. The user found it a strong overcomplication of the architecture: the master, its server mode and its rule "never on one computer" (section 3.1 until then) were removed before they were built. A forgotten password is recovered on the workstation itself (section 5.4).

### 3.1 (Removed 2026-10-24)

Was: a master and a workstation never on one computer. There is no master any more (section 3).

### 3.2 The first start (agreed; changed 2026-10-24)

The first start is the real moment of installation. **It is told by the system database**: without `system.db` in the folder of the installation (`$XDG_DATA_HOME/io-master`, usually `~/.local/share/io-master`) the window shows the setup, with it the login. The check runs in `installation/verify.rs` before the window opens; `main` only calls it. A system database that cannot be read is a damaged installation: the program does not start (critical error).

- The setup is made in the window, by the person in front of it: the name of the user and the password (twice). The window opens no network port, so no other user or program of the computer can reach the setup.
- The person who installs the program is its **only user**.
- The data of an installation lives in its folder (the system database and the encrypted database of the user); all settings are in the databases.
- **Removing the installation** is done in the window, after a login and a confirmation; it removes the databases. There is no console command for it: whoever can type in a terminal of the account must not be able to remove the work of the evaluator. TODO (step 12): `installation/uninstall.rs` is a console function marked "do not use" (not compiled) until it is reworked.
- **Until 2026-10-24** the location and a *secret of the application* were to be kept in the keyring of the Linux account (GNOME Keyring), which would also tell the first start; it was never built and was dropped (section 5.6).

## 4. User interface (agreed, changed 2026-10-16)

- **A desktop window** (Tauri 2 with a page in Svelte, since 2026-10-16; step 14 of `todo.md`). The page is built into the program and talks to it through a fixed list of commands; **the program opens no network port for its interface**. The earlier web interface on 127.0.0.1 (with its Host and Origin checks against DNS rebinding and cross-site requests) was removed with step 14; it is in the git history (commit `acedc8b`).
- **The program is started without parameters.** A parameter given anyway is refused. All configuration is made in the window.
- **The program serves nothing on the network**; it is only a client of the AI services. A remote access to the interface does not exist: the evaluator works at the computer.
- TLS 1.3 is used wherever data leaves the computer: the AI services.
- **AI connections (agreed 2026-10-16, step 15 of `todo.md`).** The program knows no AI provider. It is meant for a local model of the laboratory; Anthropic serves only for the tests and is set up the same way. The evaluator describes each service completely on the tab AI of Settings: endpoint, model, optional API key and CA certificate, the headers and the JSON template of the request, and the paths to the text, the stop reason, an error, the model and the token counts in the answer (a stream of events or one JSON). Rules:
  - `https://` with TLS 1.3 and a verified certificate (the system's, or only the CA certificate of the connection); **plain `http://` only to the loopback** (`127.0.0.0/8`, `::1`, written as an IP address), because that traffic does not leave the computer; no redirects, no proxy of the environment.
  - The API key only where `{{api_key}}` stands in a header, never in the URL or the body; never shown back; masked in the report of a check; bound to the scheme, host and port it was entered for (a changed address removes it).
  - Placeholders of the body are whole JSON values, replaced as values: a message can never change the structure of the request.
  - **The AI layer (steps 38 and 40, 2026-10-17):** the rest of the program asks the AI only through `ai/api/`, in the form of REST (connections, roles, tasks, completions, checks); the communication with the provider (`ai/api/provider/`) is reachable only from there, and the text of every question and the reading of every answer are in `ai/tasks/`. Every question is asked in a task and a role (analysis of a text and its fragments, summary, consultation, formal analysis) and written into the journal of the AI by `ai/api/` itself (`completions::ask`), which is never changed. The connections, the roles, the instructions, the chats and the journal are stored by `db/ai/`.
- Minimal layout: **navigation on the left**, the selected page on the right. **One theme only, dark** (user, 2026-10-23): it does not follow the preference of the system; no light theme is planned.
- The pages, in this order in the navigation:
  1. **Chat**: the AI assistant, over the active AI connection.
  2. **Evaluation plan**: the OSCAL documents of the standards (catalogs made in the program or loaded, mappings, profiles, assessment plans), and the documents of standards (PDF) they quote (section 7.1).
  3. **Evaluation lab**: an evaluation plan carried out for one product, with what the manufacturer and the laboratory deliver (section 7.1; a placeholder until step 41.5, the documentation of the manufacturer with step 30).
  4. **User** (step 47): the settings of the evaluator, those of the account and of the encryption of its database: the password; later the recovery code (section 5.4), an optional second factor of the login and the removal of the installation.
  5. **Settings**: the settings of the program, in tabs: **AI** (the connections to AI services and their API keys, the active connection, the check of a connection; changed 2026-10-16: settings, not a tool of its own), **Program** (the version and the folder of the installation, its protection; later the backups and the removal of the installation).
- **Logging in** with the name and the password; the keys of section 5.

## 5. Secrets and encryption (agreed)

### 5.1 The password (agreed; changed 2026-10-24)

**The program authenticates with the password alone** (user, 2026-10-24, step 48): simple to use, as an open source program should be, and it runs everywhere, also on virtual machines. **An optional second factor of the login may be added later**; the derivation of the key (section 5.2) has the place for it.

- The program has its **own password**, independent of the password of the Linux account: 12 to 1024 characters, nothing else demanded (as NIST recommends).
- The evaluator has **their own API keys** (one per AI connection), stored in the encrypted database of their installation.
- Someone who copies the files can try passwords offline; Argon2id (64 MiB per try) and the minimum of 12 characters make that slow. The strength of the password is the protection of copied files.
- **Dropped on 2026-10-24** (step 48), as the overcomplication of a simple program: the TPM 2.0 as a factor (step 8; its half-built code and the emulator `swtpm` removed), passkeys (step 9), the secret of the application in the keyring (section 5.6). For a few hours of the same day the login with the OpenPGP key of the evaluator (GnuPG) was considered; the user kept the password, since OpenPGP keys have no recovery of a forgotten passphrase either and the program is to be easy to use (`todo.md`, step 48).

### 5.2 Key of the database

```text
password of the evaluator ──► Argon2id ──► HKDF ──► unlocking key ──AES-256-GCM──► key of the database ──► database (SQLCipher)
                                            ▲
                              (a further factor would join here)
```

- The key of the database is random (`db/encryption/generate_key.rs`) and is never stored in plain form; it is stored encrypted with the unlocking key (`db/encryption/keys.rs`), in the system database.
- Opening the database needs the password; a wrong one fails to decrypt the key (GCM detects it).
- Changing the password re-encrypts only the 32-byte key of the database, not the database.
- Limit: while an evaluator is logged in, the key is in the memory of the program. The scheme protects the data when the program is closed or logged out.

### 5.3 Two levels of the database (agreed)

| Level | Protection | Opened | Holds |
|---|---|---|---|
| **System database** (of the installation) | not encrypted, readable only by the user | at program start, without a password | the installation (identifier), the account with the key of its database in encrypted form |
| **Database of the evaluator** | encrypted; key opened by the password | at login | API keys, chats, working results |

The system database is opened before anyone gives a password, so there is no key that could protect it: it therefore holds **nothing secret**. The key of the database of the evaluator it holds is encrypted and useless without the password.

**The code (steps 37 and 40 of `todo.md`, 2026-10-17):** all SQL is in `src/db/`, one folder per area of data, each with its tables and its files of create, read, update and delete; all code of encryption is in `src/db/encryption/`, and `sqlcipher.rs` is the only place where a database file is opened or created. The connection of the database of an evaluator is visible only inside `db/`, so SQL elsewhere does not compile. The database of an evaluator is opened once at the login and kept in the session of the evaluator (`installation/session.rs`) until the logout; the key that opened it is wiped. The OSCAL documents are kept as their JSON, checked against the schema of NIST at every write, with tables of index for their UUIDs and ids (`db/oscal/`); the evaluation support has tables of its own (`db/evaluation_support/`). The commands of the views change them and write them through to the database at once. Details: `code_structure.md`.

Both databases are **SQLite files** (agreed, implemented in step 3), the database of the evaluator encrypted with SQLCipher: no separate database service, one file per database in the folder of the installation (section 3.2), transactions that survive an interrupted write, and a built-in way to copy a running database.

### 5.4 Recovery of a forgotten password (agreed 2026-10-24; planned, step 49 of `todo.md`)

**No server and no master** (section 3). A forgotten password is recovered on the workstation with a **recovery code**, the mechanism of BitLocker, FileVault and LUKS (a second "slot" of the key), approved by the user on 2026-10-24:

```text
                    ┌─ password ───────Argon2id─► HKDF ─► unlocking key 1 ──AES-256-GCM──► copy A ─┐
key of the database ┤                                                                               ├─► system.db
                    └─ recovery code ──────────► HKDF ─► unlocking key 2 ──AES-256-GCM──► copy B ─┘
```

- **At the setup** the program makes a random recovery code (160 bits from the kernel, written as groups of characters without 0, O, 1 and I, with a check sum against typing errors) and wraps the key of the database a second time with it. The code is shown **once**; the evaluator prints or copies it, confirms it by typing a group, and keeps it outside the computer (the safe of the laboratory). Argon2id is not needed for it: 160 random bits cannot be guessed.
- **A forgotten password:** "I do not remember my password" on the login page; the code opens copy B, the evaluator sets a new password (a new copy A); the database is not re-encrypted.
- **After a use the code is replaced** by a new one (shown once) and copy B is made anew; the evaluator can also replace it on the page User at any time, after entering the password.
- **Limits, said in the window:** the code is a second key to the data (whoever finds it has the data); a lost password *and* a lost code mean lost data (only a backup is left); a replaced code protects the files on the computer, not copies of them taken earlier.
- **Until the recovery code is built** every page shows an orange bar: "Your password cannot be recovered: a forgotten password means lost data."

### 5.5 Backup (agreed; later stage; changed 2026-10-24)

The evaluator exports backups of the work (also the only way to move it to a new computer). **A backup can be encrypted with an OpenPGP key, optionally** (user, 2026-10-24): for the key of the evaluator and, if the laboratory wishes, for a key of the laboratory, a standard the laboratories use. **The key is never required**: without it the backup is not encrypted, and the window says so and recommends a key. (Open, decided when the function is built: whether a backup without encryption leaves out the API keys.)

### 5.6 (Removed 2026-10-24) The keyring and the secret of the application

Was: the location of the installation and a *secret of the application* (a "pepper" of the unlocking key, also the recovery code) in the keyring of the Linux account through the Secret Service (GNOME Keyring). It was never built (step 13); the user dropped it (2026-10-24, step 48): it added security at the cost of chaos (a desktop session with GNOME Keyring required, its PAM module on XFCE, the restoring of a lost keyring). The location is the usual data directory (section 3.2); the recovery is section 5.4.

## 6. Collaboration (changed 2026-10-24)

**No git in the program** (user, 2026-10-24, step 48): the laboratory manages its documents itself; not every laboratory uses git, and the configurations of the program would hold the work up. The program does not share data between evaluators. What leaves it is exported by the evaluator: the report filled into the LaTeX template of the laboratory (step 28), the documents of OSCAL, and backups (section 5.5). Everything else (API keys, chats, working results) stays in the encrypted database of the evaluator.

## 7. Work on standards

The program works on documents: the standards, the documentation of the client and what the evaluator delivers. **It does not talk to devices** (user, 2026-10-24, step 48): the scan of the serial ports and the planned work on UART and Bluetooth were removed; the results of the tests the laboratory carries out on a device are inputs of the evaluator, which the program reads but does not produce. The code of the scan is in the history of git (`history.md`, step 48).

### 7.1 Standards: the evaluation plan and the evaluation on NIST OSCAL (agreed 2026-10-18, step 41 of `todo.md`)

The program works against any **standard**, not only Common Criteria. **The data of the standards is stored in NIST OSCAL 1.2.3** (Open Security Controls Assessment Language), the published, machine-readable format of NIST for requirements and their assessment; every OSCAL document is checked against the JSON schemas of NIST (kept in the repository, `io-master/oscal/`) whenever it is written. The whole concept, with the decisions of the user and the examples of how Common Criteria and ETSI EN 303 645 are represented, is in **`oscal.md`**; this section gives its frame.

- **Two functions.** The **evaluation plan** makes or loads the OSCAL scheme of a standard, shows it as a graph and hands it over; the **evaluation** takes a plan and, with the AI, provides the evidence, checks whether the requirements are met and makes the report. The results of the functional tests the laboratory carries out are inputs of the evaluation, not functions of the program (section 7). It supports the laboratory; it is not the whole process of certification.
- **The models of OSCAL in use:** catalog (the requirements of a standard), mapping (relations between requirements, also of two standards), profile (the requirements chosen for one evaluation, their parameters set), assessment plan (the plan: tasks, activities, subjects, tools) and assessment results (observations, findings, risks, the log of the assessment). The system security plan and the component definition come with the documentation of the manufacturer (step 30).
- **No solution of its own for one standard.** Every document that defines the work is an OSCAL document and can be taken into a plan: the criteria (CC Parts 1 to 5, ETSI EN 303 645), the methodology with its work units (the CEM), the Protection Profiles, test specifications (ETSI TS 103 701). Catalogs published in OSCAL (NIST SP 800-53, ...) are imported as they are; the others are made in the program.
- **Documents of a standard.** Its PDFs are loaded, read page by page (`pdftotext` of Poppler, the PDF on its standard input) and kept with their text in the encrypted database of the evaluator; a catalog names the PDFs it comes from as resources of its back matter, by the SHA-256 of the file. The viewer (pdf.js, built in) shows a quoted passage highlighted on its page.
- **Catalogs made in the program.** The AI proposes the groups of a catalog from the table of contents and the controls of a group from its pages, part by part over the active AI connection; every proposal carries quotes, each checked on its page by the program. The evaluator accepts, corrects or rejects every proposal (P1) and edits the catalog by hand.
- **The evaluation support (our own model).** What the program adds to OSCAL (the quotes of an object with their pages and the result of their check, the AI notes, the proposals of the AI and their state, the instructions to the AI and the prompts, the layout of the graph) is kept in tables of its own and linked to OSCAL objects only by their UUIDs and ids; **nothing of ours is written into an OSCAL document**, so an export is valid OSCAL and the evaluation support travels beside it.
- **Where the text of a standard goes is the decision of the laboratory**: the program sends it over the connection the user chose and does not warn or block. Recommended: a local model for licensed standards (a connection to `http://127.0.0.1`, which never leaves the computer); a cloud service only where the licence of the standard and the rules of the laboratory allow it.
- **The evaluation (step 41.5, then steps 31 to 36)** records its work as OSCAL assessment results. The decisions of the user of 2026-10-17 stand: the evaluator works on **one requirement at a time**; the documentation of the product is used in **two phases**, first the fragments that matter for the requirement (each an exact quote checked on its page, the explanation of the model in a separate *AI note*, never presented as a quote), then the assessment on **only the fragments the evaluator has accepted**, a methodological requirement that keeps the evaluation defensible in an audit; the program imposes **no list of verdicts**, each requirement gets four text results (*Findings*, *Analysis*, *Assessment and verdict proposition*, *Vulnerabilities and notes*), which map onto the findings, observations and risks of OSCAL (`oscal.md`, section 5); what the AI proposes becomes a result only when the evaluator accepts it; every step is logged. **Step 30:** the documentation of the manufacturer (Security Target, guidance, design) is analysed in a tool of its own. **Step 28:** the results are written into the LaTeX template of the laboratory.
- **Before step 41** (steps 16 to 29, 2026-10-16 and 17) the program had a model of its own, the *test project*: schemes of tiles (inputs, requirements, outputs) and links per chapter, and an *evaluation project* copied from it. It was replaced by OSCAL and removed with its data in step 41.1 (`history.md`); the loading of the PDFs, the viewer, the check of quotes and the work of the AI part by part were kept.

## 8. TODO

The work is planned step by step in `todo.md`. Nothing must be carried over from here at present: the TPM in the release and the login as a mock-up are gone (step 48).

## 9. System requirements (agreed 2026-10-15; changed 2026-10-24)

- **Debian 13 (trixie) is the target system and the only one tested** (user, 2026-10-24). The packages for running, building and testing are listed in `README.md`, section "Requirements".
- **Few resources are the measure** (user, 2026-10-21): the virtual machines of a laboratory usually get little of the hardware (a processor of some years ago, 8 GB RAM or less), so the program must work well on them.
- Poppler (**`poppler-utils`**: `pdfinfo`, `pdftotext`) for loading the documents of standards (section 7.1).
- Linux with a **graphical desktop session** (the window). XFCE on X11 is the tested desktop. No keyring is needed any more (section 5.6).
- **The memory (step 47, 2026-10-23).** The decrypted data and the key of the database are only in RAM, and only what the queries of the moment read (SQLCipher decrypts page by page, its cache is 2 MiB); the file on the disk is always encrypted, so a cut of the power leaves nothing readable. The program forbids the memory dumps of its process and SQLCipher locks its keys in RAM; what the program cannot enforce is a requirement of the laboratory: **the swap encrypted (or none), and no hibernation** on the computer of an evaluator, since both write the RAM to the disk.

## 10. Vocabulary (agreed 2026-10-23)

The user prompts in Polish and the code is written in English, so every name below is used the same way in Rust, in the page, in the navigation of the window and in the documents.

### 10.1 The views

A **view** is a whole page of the window, the high-level display.

| Name | The user's words | What it is |
|---|---|---|
| `users` (was `start`) | użytkownicy | the account of the evaluator, its session and the encryption of its database in one place (step 47): the setup that makes the user, the login, the page User (the password), the user menu (the logout) |
| `shell` | shell | the frame of the window around every view: the navigation, the user menu |
| `chat` | chat | the panel with the chatbot (the AI in the role *consultation*) |
| `evaluation_plan` | plan ewaluacji | everything from the PDFs of a standard to OSCAL: the documents of standards, catalogs made from them (the editor of a catalog), OSCAL documents loaded, the scheme of the evaluation (profile, assessment plan) |
| `evaluation_lab` | evaluation lab | the structure of a standard joined with what the client and the laboratory deliver (the documentation of the manufacturer, the Security Target, the results of the tests), and the evidence assessed with the AI |
| `settings` | ustawienia | the settings panel |

The view `evaluation_plan` is wider than the OSCAL document *assessment plan* it makes; the two are never called by the same name.

### 10.2 Components

A **component** is a small element used more than once: several times in one view, or in several views. Components live in `components/` (in Rust and in the page) and are imported by the views; a component never imports a view. A part of a view used only once stays in the folder of its view, and is called an **element** of the view.

| Name | The user's words | What it is |
|---|---|---|
| `pdf_viewer` | podgląd normy | a document of a standard, a quote highlighted on its page; in the evaluation plan and in the editor of a catalog |
| `part_rows` | części obiektu | the parts of a group or a control as rows; the object chosen and the proposals of the AI |
| `requirement` (planned, step 41.5) | wymaganie | a tile that describes one requirement of a standard; many times in a view and in several views; functions of the AI, such as the analysis of the requirement |

**Open (user, 2026-10-23): the tiles of the evidence.** Not thought through yet, so nothing is decided and nothing is built. The user's present idea: a requirement is linked to its **input** tiles, which say what is needed to satisfy it; a **proof** tile (dowód: a result of a test, quotes of a standard or of the Security Target) is linked to an input, and the AI in the input checks whether the proof meets the criteria to be that input of the requirement.

### 10.3 Other terms

| Name | The user's words | What it is |
|---|---|---|
| the page | strona, front end | `io-master/ui/`, Svelte and TypeScript: a separate project |
| `app/` | app | what every view uses: the state of the window (`AppState`), the errors of the commands (`AppError`) |
| `routes.rs` | routes | the list of every command the page may call, as `urls.py` of Django; above the views it names |
| command | komenda | a function of Rust the page calls through Tauri, named `<element>_<action>` |
| evaluation support | wsparcie ewaluacji | what the program adds to OSCAL (quotes, proposals of the AI, instructions, the layout of the graph), linked to OSCAL by UUID and id (section 7.1) |

## 11. The folders and their tasks (agreed 2026-10-17, 2026-10-21 and 2026-10-23)

Each folder does one thing; a folder uses only itself and the folders below it, which a test checks (`src/architecture_tests.rs`). Details, the rules of each folder and the whole tree: `code_structure.md`.

| Folder | Its task | Uses |
|---|---|---|
| `views/` | the views of section 10.1: one folder per view, with the elements of its page used only there; a command takes the parameters from the page, does the work with the folders below and gives the data back | `components/`, everything below |
| `components/` | the components of section 10.2, built as an element: its commands and its data | everything below `views/` |
| `routes.rs` (a file, not a folder) | the list of every command the page may call; the window of the program and the window of the tests use it | `views/` |
| `app/` | the state of the window (the installation, the session, the stop signals), the errors of the commands as the page tells them apart | `installation/`, `model/`, the errors of the folders below |
| `installation/` | the installation of the Linux account, its one account and the session of the logged-in evaluator: the core of the protection | `db/`, `security/`, `model/` |
| `ai/` | the whole communication with AI services: `ai/api/` the only way to a model, `ai/tasks/` the questions and the reading of the answers | `db/`, `model/` |
| `pdf/` | reading PDFs with Poppler: the only place where the program starts another program on a document | nothing of the program |
| `db/` | every operation on a database, as CRUD, one folder per area of data; all SQL and all encryption of the databases | `model/`, `security/` |
| `security/` | the building blocks of protection: random numbers, no memory dumps of the process | nothing of the program |
| `model/` | the data as plain Rust types and the rules of their validity; no I/O | nothing of the program |
| `ui/` (the page) | what the evaluator sees: the views and the components of the page, and one typed function per command (`ui/src/lib/api/`) | the commands only |

## 12. The flow of data in the views (agreed 2026-10-21 and 2026-10-23)

**How the window loads (user, 2026-10-23).** The page is loaded once, when the window opens, and is never loaded again. A view is shown without asking the program for the whole page; every element and every component reads its own data with its own command and refreshes only itself after a change. An element that changed data tells its view, and the view asks the elements that show that data to read it again. Only what matters goes to the program (the AI, processing, storing); what is only presentation (a tab, a selection, an open dialog) stays in the page.

```text
the page: a view, its elements and components     what it shows; no secret, no state that matters
  │  a command <element>_<action>, its arguments as JSON
  ▼
views/<view>/<view>.rs (the entry of the view)   its command: the session taken, the parameters checked
  ├──► model/                                     the rules of the data
  ├──► db/                                        written through at once, in one transaction
  └──► ai/api/, pdf/                              the only ways to a model, a PDF
  ▼
views/<view>/<element>/view.rs                    the data the element gets back (JSON)
  │  a long work: its events through a channel of Tauri (progress, Stop)
  ▼
the page: the element shows it; "changed" ──► the view ──► the other elements read again
```

The flow inside each view, as it is now (the entry of each view, `views/<view>/<view>.rs`, holds its commands in this order):

**`users` and `shell`** (entries `views/users/users.rs` and `views/shell/shell.rs`). Everything that touches the key of the database is in `users`; the page asks for the state of the program when the window opens and after each of its changes, and shows the setup, the login or the frame:

```text
SetupForm    ──setup_form_install(name, password)────────► installation/: made, logged in ─┐
LoginForm    ──login_form_log_in(name, password)─────────► installation/: logged in ───────┤
UserMenu     ──user_menu_log_out()───────────────────────► installation/: the session ends ┤
PasswordForm ──password_form_change(old, new, new)──► installation/: the key of the database encrypted again
                                                                                            ▼
App.svelte ──program_get()──► ProgramView: installed, who is logged in, the pages, the warnings
```

**The way of the key in every operation of `users`** (step 47; the functions named are those the commands of `views/users/users.rs` call; the keys: section 5.2):

```text
setup_form_install(name, password)
  lifecycle::install ──► system.db made (not encrypted: the installation, the account)
    accounts::create ──► keys::check_password (at least 12 characters)
                     ──► generate_database_key: 32 random bytes ─────────────────── the key of the database (RAM)
                     ──► keys::wrap: password ──Argon2id──► HKDF ──► unlocking key
                                     key of the database ──AES-256-GCM──► WrappedKey ──► system.db (accounts)
                     ──► user_db::create: users/<id>.db made by SQLCipher with the key
  then the login below; the key of the setup is dropped (wiped)

login_form_log_in(name, password)
  accounts::log_in ──► the WrappedKey of the account from system.db
                   ──► keys::unwrap: password ──Argon2id──► HKDF ──► unlocking key ──AES-256-GCM──► the key
                       (a wrong password fails here: GCM detects it; the same error as an unknown name)
  Session::open ──► SQLCipher opens users/<id>.db with the key (its memory locked and wiped)
                ──► the key of the program dropped (wiped); SQLCipher keeps its own inside the connection
  AppState::log_in ──► the session, with the open database, until its end

password_form_change(current, new, new)
  accounts::change_password ──► keys::unwrap with the current password ──► the key (RAM, briefly)
                            ──► keys::wrap with the new password ──► the new WrappedKey ──► system.db
                            (the database itself is not touched; the key is wiped)

user_menu_log_out()                    the end of the session
user_menu_close()                      the same, then the end of the program
the cross of the window, the end       RunEvent::Exit (main.rs) ──► the same
  AppState::log_out ──► the reply and the long work stopped
                    ──► the session dropped; when its last holder lets it go, the connection
                        closes and SQLCipher wipes its key and its pages
```

What stays on the disk after any of them, or after a cut of the power: `system.db` (no secret: the keys only wrapped) and `users/<id>.db`, encrypted page by page. What is in RAM during the session: the open connection with its key and at most 2 MiB of decrypted pages, and what a command reads for the page.

**`chat`** (entry `views/chat/chat.rs`, page `ui/src/views/chat/`).

```text
the view opens
  ├─► ConnectionLine ──connection_line_get()──► the connection of the role consultation (ai/api/)
  └─► ChatList ──chat_list_get()──► db/ai/chats/: the chats, the newest first
ChatList ──"chosen"(id)──► the view ──► Conversation ──conversation_open(id)──► the chat from db/
Conversation ──conversation_send(chat, text)──► db: the message ──► ai/api/: the reply, piece by piece ──► db: the reply
     ▲                                                                   │
     └──────────── events (ReplyEvent): the text as it comes, the end ◄──┘
Conversation ──conversation_stop() / conversation_delete(id)
Conversation ──"changed"──► the view ──► ChatList ──chat_list_get()──► the list again
```

**`evaluation_plan`** (entry `views/evaluation_plan/evaluation_plan.rs`, page `ui/src/views/evaluation_plan/`). The documents of standards (PDF), the documents of OSCAL, and the editor of a catalog:

```text
the view opens ──► OscalDocuments ──oscal_documents_get()──► the documents of OSCAL by model and title
the tab Documents of standards ──► StandardDocuments ──standard_documents_get()──► the PDFs by title and part
StandardDocuments ──standard_documents_load()──► the dialog ──► pdf/: the text, part by part ──► db/standards/
                     (events LoadEvent: file, pages, end; standard_documents_stop())
StandardDocuments ──standard_documents_update / _delete──► db/standards/ (refused while a catalog quotes it)
OscalDocuments ──oscal_documents_import / _export / _delete──► the dialog ──► checked against the schema of NIST ──► db/oscal/
NewCatalog ──new_catalog_sources()──► the PDFs it can come from; ──new_catalog_create(title, sources)──► db/oscal/
     └── "made" ──► the view opens it in the editor
DocumentSearch ──document_search_run(text)──► the hits ──► the view ──► the viewer (component) ──► the PDF, the quote on its page

the editor of a catalog (entry views/evaluation_plan/catalog_editor/catalog_editor.rs):
the editor opens ──► CatalogTree ──catalog_tree_get(uuid)──► the rows of the tree (no recursion in the page)
                 ├─► Proposals ──proposals_get(uuid)──► what the AI proposed, waiting
                 └─► AiBuilder ──ai_builder_sources()──► the documents the AI can read
CatalogTree ──"chosen"──► the editor ──► ObjectForm ──object_form_get(uuid, id)──►; ObjectQuotes ──object_quotes_get──►
ObjectForm, NewObject, ObjectQuotes, Proposals ──a change──► the catalog in db/oscal/ (checked against the schema of NIST)
  └── "changed" ──► the editor ──► every element reads again
AiBuilder ──ai_builder_groups / ai_builder_controls(pages)──► ai/api/: the model, part by part ──► proposals in db/
  └── BuildEvent (part, writing, proposed, failed, end) while it works; ai_builder_stop(); "changed" at the end
Proposals ──proposals_accept(id, corrected)──► the object in the catalog, its quotes kept (P1)
```

Every question of the AI of the editor follows one pipeline (`ai_builder/`: `groups.rs` from a table of contents, `controls.rs` from the pages of a group, `steps.rs` what both share, `parts.rs` the pages cut into parts that fit the context of a model); the database is held for each step only, never while the model writes:

```text
the catalog and the pages (db/) ──prompt (ai/tasks/catalog_*.rs)──► the question
  ──ai/api/completions::ask──► the model, and the journal of the AI
  ──read (ai/tasks/catalog_*.rs)──► groups or controls of OSCAL
  ──quotes checked on their pages, proposals stored (db/evaluation_support/)
```

**`settings`** (entry `views/settings/settings.rs`, page `ui/src/views/settings/`): the settings of the program; those of the user are in `users`.

```text
the tab AI opens
  ├─► ConnectionList ──connection_list_get()──► the connections by name, the active one
  └─► ConnectionForm ──connection_form_new()──► the template of a new connection, the presets
ConnectionList ──"chosen"(id)──► the view ──► ConnectionForm ──connection_form_get(id)──► the connection (never the key)
                                           ├─► ApiKey ──api_key_get(id)──► whether a key is stored
                                           └─► ConnectionCheck (emptied)
ConnectionForm ──connection_form_save / _delete / _activate──► db/ai/connection/ (through ai/api/connections)
  └── "changed" ──► the view ──► ConnectionList ──connection_list_get()──►; ApiKey ──api_key_get(id)──►
ApiKey ──api_key_store(id, key) / api_key_delete(id)──► the key in the encrypted database, bound to the address
ConnectionCheck ──connection_check_run(id)──► ai/api/checks: one short question, written into the journal ──► the report
```

**`evaluation_lab` (planned, step 41.5; open).** Its flow is drawn when its concept is settled; the tiles of the evidence are still open (section 10.2). What stands: the plan is handed over from `evaluation_plan`, what the AI proposes becomes a result of the assessment (OSCAL) only when the evaluator accepts it (P1), and every question to the AI is in the journal (P3).
