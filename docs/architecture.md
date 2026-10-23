# IO Master: architecture

This document records the concept of the program, as agreed from 2026-10-14 on (one user per installation, the keyring and the secret of the application on 2026-10-15; the desktop window on 2026-10-16; the layers of the code on 2026-10-17; the standards in NIST OSCAL on 2026-10-18). It describes *what* the program is and *why* it is built this way. *Where* everything is in the code is in `code_structure.md`; the work with standards on NIST OSCAL in `oscal.md`; the functions of v0.2 that are being re-created are listed in `v0.2_functionality.md`.

Status markers: **agreed** is decided; **open** still needs a decision; **TODO** is known work that must not be forgotten.

**The vocabulary of the project is in section 10, the tasks of the folders in section 11, the flow of data in the views in section 12.** They are at the end so that the numbers of the earlier sections, named all over the code and the documents, stay as they are.

---

## 1. Purpose

IO Master is a tool for evaluators of security laboratories, who evaluate products against standards: first of all Common Criteria (ISO/IEC 15408, CEM ISO/IEC 18045) and ETSI EN 303 645, and any other standard in the same way. The requirements of a standard, the plan of their evaluation and its results are kept in **NIST OSCAL** (section 7.1). The program brings together the evaluation plan and the evaluation with AI assistance, the work on the device under evaluation (serial ports, Bluetooth), and the record of everything that was done.

The program is installed **locally, on the computer of the evaluator**, in their own Linux account: **one Linux account holds one installation with one user of the program** (section 3). It is used through a **desktop window** (section 4). There is no console interface.

## 2. Principles

Carried over from the design of v0.2; they are requirements, not preferences.

| # | Principle | Consequence |
|---|---|---|
| P1 | **The evaluator decides.** | The program never issues a verdict. It offers an estimate with its reasons. |
| P2 | **No command reaches a device without an action of the evaluator for that command.** | The program proposes; the evaluator executes. Acting on a device carries legal responsibility. |
| P3 | **Everything is traceable.** | Every passage used, every proposal of the model, every approval, every command and every answer of the device is recorded. |
| P4 | **The norm is data, not code.** | Every standard is kept as OSCAL documents (catalogs, profiles, mappings), made or loaded; nothing about a standard is written into the program (section 7.1). |
| P5 | **The model reads; it does not invent sources.** | Passages of the norm are found by the identifiers of their OSCAL objects and the quotes kept with them; every quote the AI proposes is checked on its page. The model works on the text it is given. |

## 3. One program, two roles (agreed)

There is one program. Its **role is chosen once, at the first start, in the window**, together with its user (section 3.2).

**One Linux account, one installation, one user (agreed 2026-10-15).** An installation belongs to the Linux account that installed it, and it has exactly one user of the program, on a workstation and on a master alike. Users cannot be added. The data of a laboratory is critical, so the installation is bound as tightly as possible to one person and one account; evaluators work together only through the git of the laboratory (section 6). Afterwards the choice cannot be reached again: a role that could be switched later would let a compromised installation turn itself into the other role.

| Role | Where | What it can do | What is blocked |
|---|---|---|---|
| **Workstation** | the Linux account of an evaluator | everything: the window, AI, chats, device scan and tests, evaluations. **One user only**: the evaluator who installed it | the master panel; adding users |
| **Master** | a separate, dedicated machine | only the password reset service: an API for workstations and a panel where its administrator approves requests. **One user only**: the one who installed it | everything else; adding users |

```text
 workstation of evaluator A          workstation of evaluator B
┌──────────────────────────┐        ┌──────────────────────────┐
│ the window (Tauri)       │        │ the window (Tauri)       │
│ io-master (workstation)  │        │ io-master (workstation)  │
│  data, AI, devices       │        │  data, AI, devices       │
└────────────┬─────────────┘        └────────────┬─────────────┘
             │  password reset only (optional)   │
             └──────────────► master ◄───────────┘
                          (admin approves)
```

All the work and all the data stay on the workstation. The master never commands a device and never sees the data of an evaluator.

### 3.1 Never on one computer (agreed)

A master and a workstation **never run on the same computer**, not even for development or tests: whoever took over that computer would get every secret, which is exactly what the split into two roles prevents. There is no mode that allows it.

- A Linux account holds one installation of IO Master. With the TPM factor, one computer holds one installation: the TPM refuses a second one. Without it, several evaluators of one computer each have their own account and installation; a master is never one of them.
- Developing the master needs a second machine; a virtual machine with its own virtual TPM counts as one.

### 3.2 The first start (agreed, changed 2026-10-15)

The first start is the real moment of installation. **It is told by the keyring of the Linux account** (section 5.6): two entries, the location of the installation and the secret of the application, are written there by the installation. The checks run in `installation/verify.rs` before the window opens; `main` only calls them.

| Entries in the keyring | Installation at the recorded location | State | What the program does |
|---|---|---|---|
| missing | (not known) | first start, or the keyring was lost | the start page offers two ways: a **new installation**, or the **restoring of an existing one** |
| present | valid | installed | normal work; the role cannot be chosen |
| present | missing, damaged, wrong permissions | damaged | **refuses to start** (critical error) |
| damaged or only one present | – | damaged | **refuses to start** (critical error) |

- **A new installation** is made only in an empty or new folder, so it can never overwrite an existing installation. The location is chosen by the user; how it is chosen in the window is decided when the interface is reviewed.
- **Restoring an installation** asks for its location and its recovery code (the secret of the application, section 5.6). The program checks both against the system database and writes them back into the keyring. This is the way back after a lost keyring: a reset of the Linux password by an administrator, keyring entries removed by hand, a new system.
- With the optional TPM factor (section 5.1) the marker in the TPM is compared as well; a marker without an installation, or an installation without its marker, is a damaged installation and stops the program.
- The setup is made in the window, by the person in front of it: the window opens no network port, so no other user or program of the computer can reach the setup (the one-time setup code of the earlier web interface is no longer needed).
- The person who installs the program is its **only user**.
- **Only the role is final.** The connection to a master is set, changed or removed later in Settings; a workstation works fully without a master (section 5.4).
- The data of an installation lives in its folder (the system database and the encrypted database of the user). All settings are in the databases; only the two entries of section 5.6 are kept in the keyring.
- A build without the master code is possible later as a separate build variant.
- **Removing the installation** is done in the window, after a login and a confirmation; it removes the databases, the keyring entries and, with the TPM factor, the marker in the TPM. There is no console command for it: whoever can type in a terminal of the account must not be able to remove the work of the evaluator. TODO (step 12): `installation/uninstall.rs` is a console function marked "do not use" (not compiled) until it is reworked.

## 4. User interface (agreed, changed 2026-10-16)

- **A desktop window** (Tauri 2 with a page in Svelte, since 2026-10-16; step 14 of `todo.md`). The page is built into the program and talks to it through a fixed list of commands; **the program opens no network port for its interface**. The earlier web interface on 127.0.0.1 (with its Host and Origin checks against DNS rebinding and cross-site requests) was removed with step 14; it is in the git history (commit `acedc8b`).
- **The program is started without parameters.** A parameter given anyway is refused. All configuration is made in the window.
- **Network functions are started from the window, never from the command line:** on a master, the server of the password reset service (an API, HTTPS with TLS 1.3 only); on a workstation, its client (section 5.4, step 11). A remote access to the interface itself does not exist: the evaluator works at the computer (section 5.6).
- TLS 1.3 is used wherever data leaves the computer: the reset service between a workstation and the master, the AI services, git.
- **AI connections (agreed 2026-10-16, step 15 of `todo.md`).** The program knows no AI provider. It is meant for a local model of the laboratory; Anthropic serves only for the tests and is set up the same way. The evaluator describes each service completely on the tab AI of Settings: endpoint, model, optional API key and CA certificate, the headers and the JSON template of the request, and the paths to the text, the stop reason, an error, the model and the token counts in the answer (a stream of events or one JSON). Rules:
  - `https://` with TLS 1.3 and a verified certificate (the system's, or only the CA certificate of the connection); **plain `http://` only to the loopback** (`127.0.0.0/8`, `::1`, written as an IP address), because that traffic does not leave the computer; no redirects, no proxy of the environment.
  - The API key only where `{{api_key}}` stands in a header, never in the URL or the body; never shown back; masked in the report of a check; bound to the scheme, host and port it was entered for (a changed address removes it).
  - Placeholders of the body are whole JSON values, replaced as values: a message can never change the structure of the request.
  - **The AI layer (steps 38 and 40, 2026-10-17):** the rest of the program asks the AI only through `ai/api/`, in the form of REST (connections, roles, tasks, completions, checks); the communication with the provider (`ai/api/provider/`) is reachable only from there, and the text of every question and the reading of every answer are in `ai/tasks/`. Every question is asked in a task and a role (analysis of a text and its fragments, summary, consultation, formal analysis) and written into the journal of the AI by `ai/api/` itself (`completions::ask`), which is never changed. The connections, the roles, the instructions, the chats and the journal are stored by `db/ai/`.
- Minimal layout: **navigation on the left**, the selected page on the right.
- Pages of the workstation, in this order in the navigation:
  1. **Chat**: the AI assistant, over the active AI connection.
  2. **Evaluation plan**: the OSCAL documents of the standards (catalogs made in the program or loaded, mappings, profiles, assessment plans), and the documents of standards (PDF) they quote (section 7.1).
  3. **Evaluation lab**: an evaluation plan carried out for one product, with what the manufacturer and the laboratory deliver (section 7.1; a placeholder until step 41.5, the documentation of the manufacturer with step 30).
  4. **Devices**: the serial ports, a filter by port type, later the access rights to the ports and the work on a device (UART, Bluetooth).
  5. **Settings**, in tabs: **User** (the password, later the factors of the person), **AI** (the connections to AI services and their API keys, the active connection, the check of a connection; changed 2026-10-16: settings, not a tool of its own), **Program** (the version, the role and the folder of the installation, its protection; later the TPM factor, the secret of the application (section 5.6) and the removal of the installation).
- The master shows only its administration panel.
- **Logging in** is built as a mock-up first; the real mechanism comes near the end of development. The design of the keys (section 5) is fixed now, because the encrypted database depends on it.

## 5. Secrets and encryption (agreed)

### 5.1 Factors (agreed)

**By default the program authenticates with the password**, combined with the **secret of the application** kept in the keyring of the Linux account (section 5.6). Both are needed to open the database of the evaluator. The program must also run on machines without a TPM, such as many virtual machines; it needs a desktop session with a keyring (section 9).

**Every further factor adds protection** on top of the password; none replaces it. Planned factors, after the functions of the workstation:

| Factor | Binds the data to | Chosen | Step of `todo.md` |
|---|---|---|---|
| **TPM 2.0** | the computer: a secret of the device, sealed by the TPM | for the whole installation | 8 |
| **Passkey** (FIDO2, WebAuthn with the PRF extension) | the person: a secret computed inside their security key | by each user for their account | 9 |

Either, both or neither can be used. Each adds a secret to the unlocking key, so the files alone are useless without it, even with the password. Until a factor is configured, every page shows that the databases are protected by the passwords alone.

- The evaluator has **their own API keys** (one per AI connection), stored in the encrypted database of their installation.
- The program has its **own password**, independent of the password of the Linux account.
- Someone who copies only the files cannot even try passwords: the secret of the application is missing. With the files *and* the keyring (which is encrypted with the Linux password) passwords can be tried offline; Argon2id (64 MiB per try) and the minimum of 12 characters make that slow.

> **With the TPM factor, the TPM must stay as it is for the life of the installation.** The program never clears the TPM or changes its passwords. But the secret of the device, and with it all data, is lost if the TPM is cleared from outside the program: "Clear TPM" in the BIOS or in another operating system, some BIOS updates that reset a TPM built into the processor (fTPM), or a new motherboard. Only a backup with its own encryption (section 5.5) protects against that.

> **TODO (before release): development of the TPM factor uses the emulator `swtpm`. The release must use a real TPM 2.0 and refuse an emulated one.**

### 5.2 Key of the database

```text
password of the evaluator ──► Argon2id ──┐
secret of the application (keyring) ──────┼─► HKDF ──► unlocking key ──► key of the database ──► database (SQLCipher)
[secret of the device (TPM), optional] ───┘
```

- The key of the database is random (`db/encryption/generate_key.rs`) and is never stored in plain form; it is stored encrypted with the unlocking key. Each stored key records which factors it needs.
- Opening the database needs the password and the secret of the application, and with the TPM factor also the device.
- Changing the password re-encrypts only the 32-byte key of the database, not the database.
- Limit: while an evaluator is logged in, the key is in the memory of the program. The scheme protects the data when the program is closed or logged out.

### 5.3 Two levels of the database (agreed)

| Level | Protection | Opened | Holds |
|---|---|---|---|
| **System database** (of the installation) | not encrypted, readable only by the user; with the TPM factor: encrypted with a key from the secret of the device | at program start, without a password | the installation (identifier, role), the accounts with their keys in encrypted form; later the settings of the installation |
| **Database of the evaluator** | encrypted; key opened by the password and the secret of the application (+ TPM factor) | at login | API keys, chats, working results |

The system database is opened before anyone gives a password, so without the TPM there is no key that could protect it: it therefore holds **nothing secret**. The keys of the evaluators' databases it holds are encrypted and useless without the passwords.

**Open (step 4):** the private key of the TLS server is needed at program start, before any login. With the TPM factor it is protected by the TPM; without it, how it is protected is still to be decided.

**The code (steps 37 and 40 of `todo.md`, 2026-10-17):** all SQL is in `src/db/`, one folder per area of data, each with its tables and its files of create, read, update and delete; all code of encryption is in `src/db/encryption/`, and `sqlcipher.rs` is the only place where a database file is opened or created. The connection of the database of an evaluator is visible only inside `db/`, so SQL elsewhere does not compile. The database of an evaluator is opened once at the login and kept in the session of the evaluator (`installation/session.rs`) until the logout; the key that opened it is wiped. The OSCAL documents are kept as their JSON, checked against the schema of NIST at every write, with tables of index for their UUIDs and ids (`db/oscal/`); the evaluation support has tables of its own (`db/evaluation_support/`). The commands of the views change them and write them through to the database at once. Details: `code_structure.md`.

Both databases are **SQLite files** (agreed, implemented in step 3), the database of the evaluator encrypted with SQLCipher: no separate database service, one file per database in the folder of the installation (section 3.2), transactions that survive an interrupted write, and a built-in way to copy a running database.

### 5.4 Password reset (agreed; optional)

The reset service is **optional and off by default**. It is configured after installation, on a running workstation.

- **Without a reset service** the program works fully. Every page shows an **orange bar at the top**: "No password reset is configured: a forgotten password means lost data." The risk belongs to the user of the system.
- **With a reset service**, the key of the database is additionally encrypted for recovery:

```text
secret of the device (TPM) ──┐
                              ├─► recovery key ──► key of the database
share from the master ────────┘
```

- When the reset service is configured, the workstation (with the evaluator logged in) creates the recovery copy and gives the master only its *share*. The master keeps it sealed in its TPM.
- Reset: the workstation sends a request; **an administrator of the master approves it**; the master hands out the share; the workstation combines it with its device secret, recovers the key of the database and sets the new password.
- Neither side can decrypt the data alone: a compromised master lacks the device secret, a stolen workstation lacks the share.

**Open (step 10):** this scheme combines the share of the master with the secret of the device. Without the TPM factor the workstation has no secret of the device, so the scheme has to be reworked for installations without a TPM.

### 5.5 Backup (agreed; later stage)

A backup has **its own encryption**, designed when the function is built; with the TPM factor it must not depend on the TPM, since the databases then open only on the computer whose TPM holds the secret of the device. The backup file is also the only way to move the work of an evaluator to a new computer.

### 5.6 The keyring and the secret of the application (agreed 2026-10-15)

The keyring of the Linux account (through the Secret Service interface; GNOME Keyring on GNOME and XFCE, section 9) holds exactly two entries of the installation:

| Entry | Holds | Why in the keyring |
|---|---|---|
| location | the folder of the installation | not a secret, but harder to remove, move or change by mistake than a configuration file |
| secret of the application | 32 random bytes, made when the databases are created | a further secret of the unlocking key (section 5.2), a "pepper": the files alone are useless |

- **Every damage is critical.** A missing, damaged or contradictory entry, or an installation that does not match them, stops the program (section 3.2), except when both entries are missing: then the start page offers a new installation or the restoring of an existing one.
- **The secret is also the recovery code.** It is shown as text that can be copied by hand: the alphabet without 0, O, 1 and I, groups of five characters, with a check sum against typing errors (about 52 characters). The evaluator keeps it outside the computer, for example in the safe of the laboratory.
- **It can be shown again** in Settings, only after the password is entered again, so that an open, logged-in window is not enough to read it.
- **It can be replaced** when it may have been stolen: a new secret is made, the key of the database is encrypted again with the password and the new secret, and the keyring entry is replaced. Limit, stated in the page: a copy of the files taken *together with* the old secret still opens with the old secret and the password; the replacement protects the files on the computer, not copies already taken.
- **What the keyring does not prove.** It needs a desktop session, which makes work over SSH or without a desktop impossible: wanted, because the evaluator is to work at the computer. It is not a proof of presence: while the session is unlocked (also behind a locked screen) the keyring is open to every program of the account. A proof of presence is the passkey with user verification (section 5.1, optional, step 9).

## 6. Collaboration through git (agreed)

The program does not share data between evaluators itself. The laboratory uses its own git server; merges and conflicts are the matter of the laboratory.

- The workstation can **connect to the git server of the laboratory securely**: HTTPS with TLS 1.3 and a verified certificate, or SSH with current algorithms.
- The program **checks the configuration and warns clearly** when it is not secure: plain `http://` or `git://`, TLS below 1.3, a certificate that cannot be verified, weak SSH algorithms.
- The program answers for its own security and for the transport. What happens on the git server (for example disk encryption) is the responsibility of the laboratory.
- Credentials for git are secrets, protected like all others (section 5.1).
- **What goes to git (agreed): only the filled-in report template of the laboratory.** The laboratory writes its own report template in LaTeX with markers; the program fills the markers with parts of the evaluation and the result is committed. Everything else (API keys, chats, working results) stays in the encrypted database of the workstation. This comes in a later stage (see section 8).
- Planned later as well: a small git panel in the window.

## 7. Work on devices and on standards

Devices are reached from the workstation only. The first function is the scan of serial ports (`hardware_tests/devices/serial_ports.rs`); it reads only the description of the ports and never opens one. Sending to a device follows P2: proposed by the program, executed by an action of the evaluator.

### 7.1 Standards: the evaluation plan and the evaluation on NIST OSCAL (agreed 2026-10-18, step 41 of `todo.md`)

The program works against any **standard**, not only Common Criteria. **The data of the standards is stored in NIST OSCAL 1.2.3** (Open Security Controls Assessment Language), the published, machine-readable format of NIST for requirements and their assessment; every OSCAL document is checked against the JSON schemas of NIST (kept in the repository, `io-master/oscal/`) whenever it is written. The whole concept, with the decisions of the user and the examples of how Common Criteria and ETSI EN 303 645 are represented, is in **`oscal.md`**; this section gives its frame.

- **Two functions.** The **evaluation plan** makes or loads the OSCAL scheme of a standard, shows it as a graph and hands it over; the **evaluation** takes a plan and, with the AI, provides the evidence, checks whether the requirements are met and makes the report. Besides them, the program integrates the tools with which the laboratory carries out the functional tests (section 7). It supports the laboratory; it is not the whole process of certification.
- **The models of OSCAL in use:** catalog (the requirements of a standard), mapping (relations between requirements, also of two standards), profile (the requirements chosen for one evaluation, their parameters set), assessment plan (the plan: tasks, activities, subjects, tools) and assessment results (observations, findings, risks, the log of the assessment). The system security plan and the component definition come with the documentation of the manufacturer (step 30).
- **No solution of its own for one standard.** Every document that defines the work is an OSCAL document and can be taken into a plan: the criteria (CC Parts 1 to 5, ETSI EN 303 645), the methodology with its work units (the CEM), the Protection Profiles, test specifications (ETSI TS 103 701). Catalogs published in OSCAL (NIST SP 800-53, ...) are imported as they are; the others are made in the program.
- **Documents of a standard.** Its PDFs are loaded, read page by page (`pdftotext` of Poppler, the PDF on its standard input) and kept with their text in the encrypted database of the evaluator; a catalog names the PDFs it comes from as resources of its back matter, by the SHA-256 of the file. The viewer (pdf.js, built in) shows a quoted passage highlighted on its page.
- **Catalogs made in the program.** The AI proposes the groups of a catalog from the table of contents and the controls of a group from its pages, part by part over the active AI connection; every proposal carries quotes, each checked on its page by the program. The evaluator accepts, corrects or rejects every proposal (P1) and edits the catalog by hand.
- **The evaluation support (our own model).** What the program adds to OSCAL (the quotes of an object with their pages and the result of their check, the AI notes, the proposals of the AI and their state, the instructions to the AI and the prompts, the layout of the graph) is kept in tables of its own and linked to OSCAL objects only by their UUIDs and ids; **nothing of ours is written into an OSCAL document**, so an export is valid OSCAL and the evaluation support travels beside it.
- **Where the text of a standard goes is the decision of the laboratory**: the program sends it over the connection the user chose and does not warn or block. Recommended: a local model for licensed standards (a connection to `http://127.0.0.1`, which never leaves the computer); a cloud service only where the licence of the standard and the rules of the laboratory allow it.
- **The evaluation (step 41.5, then steps 31 to 36)** records its work as OSCAL assessment results. The decisions of the user of 2026-10-17 stand: the evaluator works on **one requirement at a time**; the documentation of the product is used in **two phases**, first the fragments that matter for the requirement (each an exact quote checked on its page, the explanation of the model in a separate *AI note*, never presented as a quote), then the assessment on **only the fragments the evaluator has accepted**, a methodological requirement that keeps the evaluation defensible in an audit; the program imposes **no list of verdicts**, each requirement gets four text results (*Findings*, *Analysis*, *Assessment and verdict proposition*, *Vulnerabilities and notes*), which map onto the findings, observations and risks of OSCAL (`oscal.md`, section 5); what the AI proposes becomes a result only when the evaluator accepts it; every step is logged. **Step 30:** the documentation of the manufacturer (Security Target, guidance, design) is analysed in a tool of its own. **Step 28:** the results are written into the LaTeX template of the laboratory.
- **Before step 41** (steps 16 to 29, 2026-10-16 and 17) the program had a model of its own, the *test project*: schemes of tiles (inputs, requirements, outputs) and links per chapter, and an *evaluation project* copied from it. It was replaced by OSCAL and removed with its data in step 41.1 (`history.md`); the loading of the PDFs, the viewer, the check of quotes and the work of the AI part by part were kept.

## 8. TODO

The work is planned step by step in `todo.md`. Two items must not be forgotten whatever the order:

- **The TPM factor uses a real TPM 2.0 in the release, not the emulator `swtpm`** (section 5.1).
- **Replace the mock-up login with the real login** (section 4).

## 9. System requirements (agreed 2026-10-15)

- **Debian 13 (trixie) is the target system; Arch Linux is tested as well** (user, 2026-10-21), with the newest packages, so that the code stays future proof for the next versions of Debian. The packages for running, building and testing on both are listed in `README.md`, section "Requirements".
- **Few resources are the measure** (user, 2026-10-21): the virtual machines of a laboratory usually get little of the hardware, so the program must work well on a machine like a ThinkPad T480 (Intel Core i5 of 2018, 8 GB RAM) or weaker. Later also on a Raspberry Pi 5 (ARM64 architecture, no TPM): the code must not depend on x86_64.
- Poppler (**`poppler-utils`** on Debian, `poppler` on Arch: `pdfinfo`, `pdftotext`) for loading the documents of standards (section 7.1).
- Linux with a **graphical desktop session and a keyring through the Secret Service interface** (the freedesktop.org standard on D-Bus, `org.freedesktop.secrets`). The program does not start without it, also not over SSH or as a system service (section 5.6).
- **Supported and tested desktops: GNOME and XFCE**, both with GNOME Keyring as the provider of the Secret Service. On XFCE the keyring is unlocked at login only with its PAM module (`libpam-gnome-keyring`) and a display manager that calls it (LightDM does), and with "Launch GNOME services on startup" switched on; otherwise it asks for its password at the first use. The installation guide describes this. Other desktops (KDE with KWallet, KeePassXC as the provider) may work but are not supported until they are tested.
- A virtual machine needs a desktop and a logged-in session as well.
- The optional TPM factor needs a TPM 2.0 (section 5.1).

## 10. Vocabulary (agreed 2026-10-23)

The user prompts in Polish and the code is written in English, so every name below is used the same way in Rust, in the page, in the navigation of the window and in the documents.

### 10.1 The views

A **view** is a whole page of the window, the high-level display.

| Name | The user's words | What it is |
|---|---|---|
| `start` | ekran logowania | the login and the first installation, before anybody is logged in |
| `shell` | shell | the frame of the window around every view: the navigation, the user menu |
| `chat` | chat | the panel with the chatbot (the AI in the role *consultation*) |
| `evaluation_plan` | plan ewaluacji | everything from the PDFs of a standard to OSCAL: the documents of standards, catalogs made from them (the editor of a catalog), OSCAL documents loaded, the scheme of the evaluation (profile, assessment plan) |
| `evaluation_lab` | evaluation lab | the structure of a standard joined with what the client and the laboratory deliver (the documentation of the manufacturer, the Security Target, the results of the tests), and the evidence assessed with the AI |
| `devices` | urządzenia | the serial ports: the scan now, later UART, Bluetooth |
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
| `hardware_tests/` | everything that talks to hardware: the scan of the serial ports, later UART, Bluetooth, the permissions of the ports | `model/` |
| `pdf/` | reading PDFs with Poppler: the only place where the program starts another program on a document | nothing of the program |
| `db/` | every operation on a database, as CRUD, one folder per area of data; all SQL and all encryption of the databases | `model/`, `security/` |
| `security/` | the building blocks of protection: random numbers, the TPM | nothing of the program |
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
  └──► ai/api/, hardware_tests/, pdf/             the only ways to a model, a device, a PDF
  ▼
views/<view>/<element>/view.rs                    the data the element gets back (JSON)
  │  a long work: its events through a channel of Tauri (progress, Stop)
  ▼
the page: the element shows it; "changed" ──► the view ──► the other elements read again
```

The flow inside each view, as it is now (the head of each `views/<view>/mod.rs` names its commands):

**`start` and `shell`** (entries `views/start/start.rs` and `views/shell/shell.rs`). The page asks for the state of the program when the window opens and after each of its changes, and shows the setup, the login or the frame:

```text
SetupForm ──setup_form_install(name, password, role)──► installation/: made, logged in ─┐
LoginForm ──login_form_log_in(name, password)─────────► installation/: logged in ───────┤
UserMenu  ──user_menu_log_out()───────────────────────► installation/: the session ends ┤
                                                                                         ▼
App.svelte ──program_get()──► ProgramView: installed, the role, who is logged in, the pages, the warnings
```

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

**`devices`** (entry `views/devices/devices.rs`, page `ui/src/views/devices/`).

```text
the view opens
  ├─► ScannerSettings ──scanner_settings_get()──► db/scan_settings/: the texts ──► ScannerSettingsView
  └─► PortsTable ──ports_table_scan()──► db/scan_settings/: the texts ──► hardware_tests/: the descriptions
                                          of the ports (a port is never opened) ──► PortsTableView
ScannerSettings ──scanner_settings_save(all_types, types, columns)──► the rules of the settings
  │                    (hardware_tests/devices/scan_settings.rs) ──► db/scan_settings/ ──► ScannerSettingsView
  └── "saved" ──► the view ──► PortsTable ──ports_table_scan()──► the table again
```

**`settings`** (entry `views/settings/settings.rs`, page `ui/src/views/settings/`).

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
PasswordForm ──password_form_change(old, new, new)──► installation/: the key of the database encrypted again
```

**`evaluation_lab` (planned, step 41.5; open).** Its flow is drawn when its concept is settled; the tiles of the evidence are still open (section 10.2). What stands: the plan is handed over from `evaluation_plan`, what the AI proposes becomes a result of the assessment (OSCAL) only when the evaluator accepts it (P1), and every question to the AI is in the journal (P3).
