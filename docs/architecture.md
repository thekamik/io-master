# IO Master v0.3: architecture

This document records the concept of the new version, as agreed on 2026-10-14. It describes *what* the program is and *why* it is built this way. Implementation details belong in the code; the functions of v0.2 that are being re-created are listed in `v0.2_functionality.md`.

Status markers: **agreed** is decided; **open** still needs a decision; **TODO** is known work that must not be forgotten.

---

## 1. Purpose

IO Master is a tool for evaluators who work under Common Criteria (ISO/IEC 15408, CEM ISO/IEC 18045). It brings together the work on the device under evaluation (serial ports, Bluetooth), AI assistance, and the record of everything that was done.

The program is installed **locally, on the computer of the evaluator**. It is used through a **web user interface** in the browser. There is no console interface.

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

There is one program. Its **role is chosen once, at the first start, in the web interface**, together with the first user (section 3.2). Afterwards the choice cannot be reached again: a role that could be switched later would let a compromised installation turn itself into the other role.

| Role | Where | What it can do | What is blocked |
|---|---|---|---|
| **Workstation** | computer of an evaluator | everything: web UI, AI, chats, device scan and tests, evaluations. Any number of users with **equal rights**; a user can add further users | the master panel |
| **Master** | a separate, dedicated machine | only the password reset service: an API for workstations and a panel where its administrator approves requests. **One user only**: the one who installed it; adding users is not possible | everything else |

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

- A computer holds one installation of IO Master. The TPM, which holds the secret of the device, makes this enforceable: a second installation on the same computer is refused (step 4 of `todo.md`).
- Developing the master needs a second machine; a virtual machine with its own virtual TPM counts as one.

### 3.2 The first start (agreed)

The first start is the real moment of installation. The program tells it from the database and the secret of the device in the TPM:

| Database | Secret in the TPM | State | What the program does |
|---|---|---|---|
| missing | missing | first start | the setup page: first user and role |
| present | present | installed | normal work; the role cannot be chosen |
| missing | present | damaged: the database was removed | refuses to start |
| present | missing | damaged: another computer, or the TPM was cleared | refuses to start |

- At the first start the program prints a **one-time setup code** in the terminal. The setup page asks for it, so only the person who started the program can install it, not another user or program of the computer.
- The setup is possible only in the local interface (without `--address`).
- The person who installs the program is its **first user**. On a master it stays the only one.
- **Only the role is final.** The connection to a master is set, changed or removed later in Settings; a workstation works fully without a master (section 5.4). An evaluator can install a workstation alone and connect it to a master set up later.
- The data of an installation lives in its data directory: `--data-dir`, default `~/.local/share/io-master` (another directory for development and tests). Nothing is kept outside the database.
- A build without the master code is possible later as a separate build variant.

## 4. Web user interface (agreed)

- **How the server is started is chosen with flags** (agreed):
  - `io-master`: plain HTTP on 127.0.0.1, port 8080. The connection between the browser and the program never leaves the computer, so TLS would protect nothing there.
  - `--port N`: another port, in either mode.
  - `--address IP`: **always the secure server**, HTTPS with TLS 1.3 only, even for `--address 127.0.0.1`. The flag itself is the request for TLS.
- **The TLS server must be configured first**, in the Settings page of the program started locally (without `--address`). The certificate, its private key and the other TLS settings are kept in the system database (section 5.3). Without them `--address` refuses to start.
- TLS 1.3 is used wherever data leaves the computer: the TLS server above, between a workstation and the master, to the AI providers and to git.
- Two attacks reach a local server through the browser and are stopped: **DNS rebinding** (only the `Host` names `127.0.0.1:PORT` and `localhost:PORT` are answered) and **cross-site requests** (session cookie `SameSite=Strict`, and a request that changes something must carry the `Origin` of the program).
- Minimal layout: **navigation on the left**, the selected page on the right.
- Pages of the workstation, in this order of work:
  1. **Chat**: the AI assistant (empty page at first).
  2. **Scan and permissions**: the serial ports, a filter by port type, and the access rights to the ports.
  3. **Settings**: among others, which properties of a port the port table shows. Kept in a session file until the database exists.
- The master shows only its administration panel.
- **Logging in** is built as a mock-up first; the real mechanism comes near the end of development. The design of the keys (section 5) is fixed now, because the encrypted database depends on it.

## 5. Secrets and encryption (agreed)

### 5.1 Where secrets live

- **Every secret is protected by the TPM 2.0** of the machine, on workstations and on the master. A secret sealed in the TPM cannot be copied to another computer.
- Each evaluator has **their own API keys**, stored in the encrypted database of their workstation.
- The program has its **own accounts and passwords**, independent of the Linux accounts.
- Several evaluators may use one workstation, each with an own account.

> **TODO (before release): development uses the TPM 2.0 emulator `swtpm`. It must be replaced by a real TPM 2.0 before the end of development, and the release must refuse to run on an emulated TPM.**

### 5.2 Key of the database

```text
password of the evaluator ──► Argon2id ──┐
                                          ├─► unlocking key ──► key of the database ──► database (SQLCipher)
secret of the device (TPM) ───────────────┘
```

- The key of the database is random (`db/generate_key.rs`) and is never stored in plain form; it is stored encrypted with the unlocking key.
- Opening the database needs **both** the password and the device: the files alone are useless, and so is the device without the password.
- Changing the password re-encrypts only the 32-byte key of the database, not the database.
- Limit: while an evaluator is logged in, the key is in the memory of the program. The scheme protects the data when the program is closed or logged out.

### 5.3 Two levels of the database (agreed)

| Level | Key | Opened | Holds |
|---|---|---|---|
| **System database** (of the installation) | sealed in the TPM only (secret of the device) | at program start, without a password | settings of the installation: TLS certificate and private key, later the role and the password reset configuration |
| **Database of an evaluator** | password of the evaluator + secret of the device (section 5.2) | at login | API keys, chats, working results |

The system database exists because the TLS server needs its private key **before** anyone logs in (the TLS handshake comes before the login page). Keeping the key in a database protected by the TPM is much safer than a plain file: another program cannot read it, and a changed file is detected when it is opened.

Both databases are **SQLite files encrypted with SQLCipher** (agreed): no separate database service, one file per database in the data directory (`--data-dir`, default `~/.local/share/io-master`), transactions that survive an interrupted write, and a built-in way to copy a running database.

### 5.4 Password reset (agreed; optional)

The reset service is **optional and off by default**. It is configured after installation, on a running workstation.

- **Without a reset service** the program works fully. Every page shows a **red bar at the top**: "No password reset is configured: a forgotten password means lost data." The risk belongs to the user of the system.
- **With a reset service**, the key of the database is additionally encrypted for recovery:

```text
secret of the device (TPM) ──┐
                              ├─► recovery key ──► key of the database
share from the master ────────┘
```

- When the reset service is configured, the workstation (with the evaluator logged in) creates the recovery copy and gives the master only its *share*. The master keeps it sealed in its TPM.
- Reset: the workstation sends a request; **an administrator of the master approves it**; the master hands out the share; the workstation combines it with its device secret, recovers the key of the database and sets the new password.
- Neither side can decrypt the data alone: a compromised master lacks the device secret, a stolen workstation lacks the share.

### 5.5 Backup (agreed; later stage)

The databases can only be opened on the computer whose TPM holds the secret of the device. A backup therefore has **its own encryption, independent of the TPM**, designed when the function is built. The backup file is also the only way to move the work of an evaluator to a new computer.

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

## 8. TODO

The work is planned step by step in `todo.md`. Two items must not be forgotten whatever the order:

- **Replace the `swtpm` emulator with a real TPM 2.0 before release** (section 5.1).
- **Replace the mock-up login with the real login** (section 4).
