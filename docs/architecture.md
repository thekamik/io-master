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

- A computer holds one installation of IO Master. With the TPM factor this is enforced: the TPM refuses a second installation on the same computer. Without it, it is a rule of operation.
- Developing the master needs a second machine; a virtual machine with its own virtual TPM counts as one.

### 3.2 The first start (agreed)

The first start is the real moment of installation. **It is told by the system database alone**: without it the program shows the setup page, with it the login. A system database that cannot be read stops the program.

With the optional TPM factor (section 5.1) the marker in the TPM is compared as well:

| Database | Marker in the TPM | State | What the program does |
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
- The program can be removed with `io-master --uninstall`, which removes its databases (and, with the TPM factor, its marker in the TPM). It works on a damaged installation too.

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

### 5.1 Factors (agreed)

**By default the program authenticates with passwords only**, and only the password opens the database of an evaluator. The program must also run on machines without a TPM, such as many virtual machines.

**Every further factor adds protection** on top of the password; none replaces it. Planned factors, after the functions of the workstation:

| Factor | Binds the data to | Chosen | Step of `todo.md` |
|---|---|---|---|
| **TPM 2.0** | the computer: a secret of the device, sealed by the TPM | for the whole installation | 8 |
| **Passkey** (FIDO2, WebAuthn with the PRF extension) | the person: a secret computed inside their security key | by each user for their account | 9 |

Either, both or neither can be used. Each adds a secret to the unlocking key, so the files alone are useless without it, even with the password. Until a factor is configured, every page shows that the databases are protected by the passwords alone.

- Each evaluator has **their own API keys**, stored in the encrypted database of their workstation.
- The program has its **own accounts and passwords**, independent of the Linux accounts.
- Several evaluators may use one workstation, each with an own account.
- Without the TPM factor, someone who copies the files can try passwords offline. Argon2id (64 MiB per try) and the minimum of 12 characters make that slow; a strong password is the protection.

> **With the TPM factor, the TPM must stay as it is for the life of the installation.** The program never clears the TPM or changes its passwords. But the secret of the device, and with it all data, is lost if the TPM is cleared from outside the program: "Clear TPM" in the BIOS or in another operating system, some BIOS updates that reset a TPM built into the processor (fTPM), or a new motherboard. Only a backup with its own encryption (section 5.5) protects against that.

> **TODO (before release): development of the TPM factor uses the emulator `swtpm`. The release must use a real TPM 2.0 and refuse an emulated one.**

### 5.2 Key of the database

```text
password of the evaluator ──► Argon2id ──┐
                                          ├─► unlocking key ──► key of the database ──► database (SQLCipher)
[secret of the device (TPM), optional] ───┘
```

- The key of the database is random (`db/generate_key.rs`) and is never stored in plain form; it is stored encrypted with the unlocking key. Each stored key records which factors it needs.
- Opening the database needs the password, and with the TPM factor also the device.
- Changing the password re-encrypts only the 32-byte key of the database, not the database.
- Limit: while an evaluator is logged in, the key is in the memory of the program. The scheme protects the data when the program is closed or logged out.

### 5.3 Two levels of the database (agreed)

| Level | Protection | Opened | Holds |
|---|---|---|---|
| **System database** (of the installation) | not encrypted, readable only by the user; with the TPM factor: encrypted with a key from the secret of the device | at program start, without a password | the installation (identifier, role), the accounts with their keys in encrypted form; later the settings of the installation |
| **Database of an evaluator** | encrypted; key opened by the password (+ TPM factor) | at login | API keys, chats, working results |

The system database is opened before anyone gives a password, so without the TPM there is no key that could protect it: it therefore holds **nothing secret**. The keys of the evaluators' databases it holds are encrypted and useless without the passwords.

**Open (step 4):** the private key of the TLS server is needed at program start, before any login. With the TPM factor it is protected by the TPM; without it, how it is protected is still to be decided.

Both databases are **SQLite files** (agreed, implemented in step 3), the databases of the evaluators encrypted with SQLCipher: no separate database service, one file per database in the data directory (`--data-dir`, default `~/.local/share/io-master`), transactions that survive an interrupted write, and a built-in way to copy a running database.

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

- **The TPM factor uses a real TPM 2.0 in the release, not the emulator `swtpm`** (section 5.1).
- **Replace the mock-up login with the real login** (section 4).
