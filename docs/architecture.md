# IO Master v0.3: architecture

This document records the concept of the new version, as agreed on 2026-10-14 and changed on 2026-10-15 (one user per installation, the keyring, the secret of the application; sections 3, 5.1, 5.6, 9). It describes *what* the program is and *why* it is built this way. Implementation details belong in the code; the functions of v0.2 that are being re-created are listed in `v0.2_functionality.md`.

Status markers: **agreed** is decided; **open** still needs a decision; **TODO** is known work that must not be forgotten.

---

## 1. Purpose

IO Master is a tool for evaluators who work under Common Criteria (ISO/IEC 15408, CEM ISO/IEC 18045). It brings together the work on the device under evaluation (serial ports, Bluetooth), AI assistance, and the record of everything that was done.

The program is installed **locally, on the computer of the evaluator**, in their own Linux account: **one Linux account holds one installation with one user of the program** (section 3). It is used through a **desktop window** (section 4). There is no console interface.

## 2. Principles

Carried over from the design of v0.2; they are requirements, not preferences.

| # | Principle | Consequence |
|---|---|---|
| P1 | **The evaluator decides.** | The program never issues a verdict. It offers an estimate with its reasons. |
| P2 | **No command reaches a device without an action of the evaluator for that command.** | The program proposes; the evaluator executes. Acting on a device carries legal responsibility. |
| P3 | **Everything is traceable.** | Every passage used, every proposal of the model, every approval, every command and every answer of the device is recorded. |
| P4 | **The norm is data, not code.** | CC and CEM are loaded from replaceable index files. |
| P5 | **The model reads; it does not search the norm.** | Passages of the norm are found by their identifiers. The model works on the documents of the product. |

## 3. One program, two roles (agreed)

There is one program. Its **role is chosen once, at the first start, in the window**, together with its user (section 3.2).

**One Linux account, one installation, one user (agreed 2026-10-15).** An installation belongs to the Linux account that installed it, and it has exactly one user of the program, on a workstation and on a master alike. Users cannot be added. The data of a laboratory is critical, so the installation is bound as tightly as possible to one person and one account; evaluators work together only through the git of the laboratory (section 6). Afterwards the choice cannot be reached again: a role that could be switched later would let a compromised installation turn itself into the other role.

| Role | Where | What it can do | What is blocked |
|---|---|---|---|
| **Workstation** | the Linux account of an evaluator | everything: web UI, AI, chats, device scan and tests, evaluations. **One user only**: the evaluator who installed it | the master panel; adding users |
| **Master** | a separate, dedicated machine | only the password reset service: an API for workstations and a panel where its administrator approves requests. **One user only**: the one who installed it | everything else; adding users |

```text
 workstation of evaluator A          workstation of evaluator B
┌──────────────────────────┐        ┌──────────────────────────┐
│ browser ── web UI        │        │ browser ── web UI        │
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

The first start is the real moment of installation. **It is told by the keyring of the Linux account** (section 5.6): two entries, the location of the installation and the secret of the application, are written there by the installation. The checks run in `startup/` before the server starts; `main` only calls them.

| Entries in the keyring | Installation at the recorded location | State | What the program does |
|---|---|---|---|
| missing | (not known) | first start, or the keyring was lost | the start page offers two ways: a **new installation**, or the **restoring of an existing one** |
| present | valid | installed | normal work; the role cannot be chosen |
| present | missing, damaged, wrong permissions | damaged | **refuses to start** (critical error) |
| damaged or only one present | – | damaged | **refuses to start** (critical error) |

- **A new installation** is made only in an empty or new folder, so it can never overwrite an existing installation. The location is chosen by the user; how it is chosen in the window is decided when the interface is reviewed.
- **Restoring an installation** asks for its location and its recovery code (the secret of the application, section 5.6). The program checks both against the system database and writes them back into the keyring. This is the way back after a lost keyring: a reset of the Linux password by an administrator, keyring entries removed by hand, a new system.
- With the optional TPM factor (section 5.1) the marker in the TPM is compared as well; a marker without an installation, or an installation without its marker, is a damaged installation and stops the program.
- At the first start the program prints a **one-time setup code** in the terminal. The setup page asks for it, so only the person who started the program can install it, not another user or program of the computer.
- The setup is possible only in the local interface (without `--address`).
- The person who installs the program is its **only user**.
- **Only the role is final.** The connection to a master is set, changed or removed later in Settings; a workstation works fully without a master (section 5.4).
- The data of an installation lives in its folder (the system database and the encrypted database of the user). All settings are in the databases; only the two entries of section 5.6 are kept in the keyring.
- A build without the master code is possible later as a separate build variant.
- **Removing the installation** is done in the window, after a login and a confirmation; it removes the databases, the keyring entries and, with the TPM factor, the marker in the TPM. There is no console command for it: whoever can type in a terminal of the account must not be able to remove the work of the evaluator. TODO (step 12): `installation/uninstall.rs` is a console function marked "do not use" until it is reworked.

## 4. User interface (agreed, changed 2026-10-16)

- **A desktop window** (Tauri 2 with a page in Svelte, since v0.4; step 14 of `todo.md`). The page is built into the program and talks to it through a fixed list of commands; **the program opens no network port for its interface**. The earlier web interface on 127.0.0.1 (v0.3, with its Host and Origin checks against DNS rebinding and cross-site requests) was removed in v0.4; it is in the git history (branch `v0.3`).
- **The program is started without parameters.** A parameter given anyway is refused. All configuration is made in the window.
- **Network functions are started from the window, never from the command line:** on a master, the server of the password reset service (an API, HTTPS with TLS 1.3 only); on a workstation, its client (section 5.4, step 11). A remote access to the interface itself does not exist: the evaluator works at the computer (section 5.6).
- TLS 1.3 is used wherever data leaves the computer: the reset service between a workstation and the master, the AI services, git.
- **AI connections (agreed 2026-10-16, step 15 of `todo.md`).** The program knows no AI provider. It is meant for a local model of the laboratory; Anthropic serves only for the tests and is set up the same way. The evaluator describes each service completely on the tab AI of Settings: endpoint, model, optional API key and CA certificate, the headers and the JSON template of the request, and the paths to the text, the stop reason, an error, the model and the token counts in the answer (a stream of events or one JSON). Rules:
  - `https://` with TLS 1.3 and a verified certificate (the system's, or only the CA certificate of the connection); **plain `http://` only to the loopback** (`127.0.0.0/8`, `::1`, written as an IP address), because that traffic does not leave the computer; no redirects, no proxy of the environment.
  - The API key only where `{{api_key}}` stands in a header, never in the URL or the body; never shown back; masked in the report of a check; bound to the scheme, host and port it was entered for (a changed address removes it).
  - Placeholders of the body are whole JSON values, replaced as values: a message can never change the structure of the request.
  - **The AI layer (step 38, 2026-10-17):** the rest of the program asks the AI only through `ai/api/`, in the form of REST (roles, tasks, completions, checks); the communication with the provider (`ai/api/provider/`) is reachable only from there. Every question is asked in a task and a role (analysis of a text and its fragments, summary, consultation, formal analysis) and written into the journal of the AI, which is never changed. The connections, the roles, the instructions, the memory of the chat and the journal are stored by `db/ai/`.
- Minimal layout: **navigation on the left**, the selected page on the right.
- Pages of the workstation, in this order of work:
  1. **Chat**: the AI assistant, over the active AI connection.
  2. **Scan and permissions**: the serial ports, a filter by port type, and the access rights to the ports.
  3. **Settings**, in tabs: **User** (the password, later the factors of the person), **AI** (the connections to AI services and their API keys, the active connection, the check of a connection; changed 2026-10-16: settings, not a tool of its own), **Program** (the version, the role and the folder of the installation, its protection; later the TPM factor, the secret of the application (section 5.6) and the removal of the installation).
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

- The key of the database is random (`db/generate_key.rs`) and is never stored in plain form; it is stored encrypted with the unlocking key. Each stored key records which factors it needs.
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

**The code (step 37 of `todo.md`, 2026-10-17):** all SQL is in `src/db/`, one folder per area (`system/`, `user/`, `ai/`, `standards/`, `test_projects/`, `evaluations/`), each with its tables and its files of create, read, update and delete; all code of encryption is in `src/db/encryption/`, and `sqlcipher.rs` is the only place where a database file is opened or created. The connection of the database of an evaluator is visible only inside `db/`, so SQL elsewhere does not compile. **Objects in memory (step 39):** a test project open for editing (and an evaluation open for work) is an object in memory in the core (`test_projects/workspace.rs`, `evaluation/workspace.rs`), changed by the CRUD of its domain and written through to the database at once; the versions of a test project for "Undo" are only in memory.

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
- **It can be shown again** in Settings, only after the password is entered again, so that an open, logged-in browser tab is not enough to read it.
- **It can be replaced** when it may have been stolen: a new secret is made, the key of the database is encrypted again with the password and the new secret, and the keyring entry is replaced. Limit, stated in the page: a copy of the files taken *together with* the old secret still opens with the old secret and the password; the replacement protects the files on the computer, not copies already taken.
- **What the keyring does not prove.** It needs a desktop session, which makes work over SSH or without a desktop impossible: wanted, because the evaluator is to work at the computer. It is not a proof of presence: while the session is unlocked (also behind a locked screen) the keyring is open to every program of the account. A proof of presence is the passkey with user verification (section 5.1, optional, step 9).

## 6. Collaboration through git (agreed)

The program does not share data between evaluators itself. The laboratory uses its own git server; merges and conflicts are the matter of the laboratory.

- The workstation can **connect to the git server of the laboratory securely**: HTTPS with TLS 1.3 and a verified certificate, or SSH with current algorithms.
- The program **checks the configuration and warns clearly** when it is not secure: plain `http://` or `git://`, TLS below 1.3, a certificate that cannot be verified, weak SSH algorithms.
- The program answers for its own security and for the transport. What happens on the git server (for example disk encryption) is the responsibility of the laboratory.
- Credentials for git are secrets, protected like all others (section 5.1).
- **What goes to git (agreed): only the filled-in report template of the laboratory.** The laboratory writes its own report template in LaTeX with markers; the program fills the markers with parts of the evaluation and the result is committed. Everything else (API keys, chats, working results) stays in the encrypted database of the workstation. This comes in a later stage (see section 8).
- Planned later as well: a small git panel in the web UI.

## 7. Work on devices

Devices are reached from the workstation only. The first function is the scan of serial ports (`devices/devices.rs`); it reads only the description of the ports and never opens one. Sending to a device follows P2: proposed by the program, executed by an action of the evaluator.

### 7.1 Test projects and evaluation schemes (agreed and built 2026-10-16, steps 16 to 22 and 24 to 27 of `todo.md`)

The program works against any **standard** (not only Common Criteria): its documents are loaded as PDFs, read page by page (`pdftotext` of Poppler, the PDF on its standard input), and kept with their text in the encrypted database of the evaluator. From them a **test project** is made: the map of the standard (only the map: providing documents, tests and reports belong to the tool *Evaluation*). A project holds a flat list of **schemes**, one per chapter of the standard; each scheme is drawn as tiles on a canvas, the user picks a chapter from the list and sees its graph. AI creates, the person corrects and approves.

- **Three kinds of tile:** *input* (what the evaluation needs: documentation, a sample, access, the result of another requirement, a part of another document of the standard), *requirement* (met when all its inputs are), *output* (the result in the standard form of a report: requirements reported, inputs, activities and tests, evidence, findings, verdict and rationale).
- **Links** follow from the kinds they join: input → requirement (needed by), requirement → output (reported in), requirement → input (provides), requirement → requirement (precedes: must be met first); no other pair and no circle. Links belong to the project: their two tiles may be in two schemes, and a tile of another scheme is drawn as a read-only shadow. Tiles can be moved between the schemes of a project.
- **Sources:** every tile keeps the passages it comes from (document, page, quote); the program checks that a quote is on its page and keeps the result; the viewer (pdf.js, built in) shows it highlighted in the original page.
- **AI** reads the table of contents and proposes the schemes (the pages of each chapter found by its heading in the text), then analyses the pages of each chapter part by part over the active AI connection and proposes tiles and links (also to requirements of other chapters); every proposal enters the scheme as *proposed*, its quotes checked; the user accepts, corrects or deletes (P1). Inputs described in a document that is not loaded become inputs to be completed later.
- **Checks** name the gaps of a scheme (requirements without inputs or not reported, unused inputs, inputs to be completed, proposals, quotes not found, untraced requirements); a project is marked ready by the user.
- **Export and import** of a whole project as a JSON file (format `io-master-project`, version 1), the documents named by title, part, version and SHA-256, the PDFs carried only if the user chooses; a head of the laboratory gives projects to the evaluators this way. The laboratory is trusted: no signatures.
- **Where the text of a standard goes is the decision of the laboratory**: the program sends it over the connection the user chose and does not warn or block. Recommended: a local model for licensed standards (a connection to `http://127.0.0.1`, which never leaves the computer); a cloud service only where the licence of the standard and the rules of the laboratory allow it.
- **Later (step 23, built as steps 29 to 36, agreed in part 2026-10-17):** the tool *Evaluation* carries a project out for one product with the help of AI; verdicts and final decisions stay with the evaluator. The evaluator works on **one requirement at a time** (project → scheme → requirement). The documentation of the product is used in **two phases**: the AI first finds the fragments that matter for the requirement (each an exact quote checked on its page, with the explanation of the model in a separate *AI note*, never presented as a quote); then it assesses the requirement on **only the fragments the evaluator has accepted**, a methodological requirement that keeps the evaluation defensible in an audit. An evaluation starts when a test project is imported: it is transformed into an **evaluation project** (a copy). The program imposes **no list of verdicts** (many standards): each requirement gets four text results, *Findings*, *Analysis*, *Assessment and verdict proposition*, *Vulnerabilities and notes*; the AI proposes them as text marked AI, and they become results only when the evaluator accepts them. A journal records every step. **Step 30:** the documentation of the manufacturer (Security Target, guidance, design) is analysed in a tool of its own; the input tiles of an evaluation read its projects. **Step 28:** the results are written into the LaTeX template of the laboratory, automatically by default.

## 8. TODO

The work is planned step by step in `todo.md`. Two items must not be forgotten whatever the order:

- **The TPM factor uses a real TPM 2.0 in the release, not the emulator `swtpm`** (section 5.1).
- **Replace the mock-up login with the real login** (section 4).

## 9. System requirements (agreed 2026-10-15)

- The package **`poppler-utils`** (`pdfinfo`, `pdftotext`) for loading the documents of standards (section 7.1).
- Linux with a **graphical desktop session and a keyring through the Secret Service interface** (the freedesktop.org standard on D-Bus, `org.freedesktop.secrets`). The program does not start without it, also not over SSH or as a system service (section 5.6).
- **Supported and tested desktops: GNOME and XFCE**, both with GNOME Keyring as the provider of the Secret Service. On XFCE the keyring is unlocked at login only with its PAM module (`libpam-gnome-keyring`) and a display manager that calls it (LightDM does), and with "Launch GNOME services on startup" switched on; otherwise it asks for its password at the first use. The installation guide describes this. Other desktops (KDE with KWallet, KeePassXC as the provider) may work but are not supported until they are tested.
- A virtual machine needs a desktop and a logged-in session as well.
- The optional TPM factor needs a TPM 2.0 (section 5.1).
