# IO Master v0.3: TODO

The steps that take the program from where it is now to the design in `architecture.md`. They are done **one at a time, in the order of work below**. The workstation is the core and is completed first; the TLS server and the master are the last stage. When the last step is done, the next functionality is planned anew.

The number of a step is its name and never changes, so that references in the code (`TODO (step 7 of docs/todo.md)`) stay valid. The order of work is kept here:

| Order | Step | State |
|---|---|---|
| 1 | 0: Put the current state in order | done |
| 2 | 1: Web server skeleton | done |
| 3 | 2: Secrets in the TPM (now an optional factor, see step 8) | done |
| 4 | 3: Databases, accounts and the management of users | done |
| 5 | 5: Scan page | done |
| 6 | **7: AI chat with Anthropic** | **next** |
| 7 | 6: Port permissions | |
| 8 | 8: The TPM as an optional factor | |
| 9 | 9: Passkeys as an optional factor | |
| 10 | 10: Before release of the workstation | |
| 11 | 4: The TLS server | before the master |
| 12 | 11: Master and password reset | last |

## How every step is done

1. **Concept.** Claude proposes how the step is built (files, dependencies, open decisions). The user approves or corrects it. No code before that.
2. **Implementation** in small parts, each with its unit tests (`<module>_tests.rs`).
3. **Check.** Builds without warnings, all tests and `clippy` pass; the user tries the result.
4. **Approval** by the user, then a commit to the branch `v0.3`.

Each step lists its **decisions** (to settle in the concept), its **tasks**, and when it is **done**. Prerequisites the user has to provide (packages, permissions) are marked **needs**.

Legend: `[ ]` to do, `[x]` done.

---

## Step 0: Put the current state in order

- [x] Commit what exists today to `v0.3`: the new folder layout (`cli_ui/`, `web_ui/`, `db/`, `keyring/`, `permission/`), `keyring.rs`, `database_key.rs`, `db/generate_key.rs`, `docs/`.
- [x] Remove the console interface: `cli_ui/`, the `--cli` parameter, its tests.
- [x] Remove empty files that no step needs yet, or keep them with a comment saying which step fills them.

**Done when:** the working tree is clean, `v0.3` on GitHub matches it, and the program builds without the console.

---

## Step 1: Web server skeleton

**Decisions**
- [x] Web framework: `axum` on `tokio`.
- [x] No TLS for the web UI: plain HTTP on 127.0.0.1 only. TLS 1.3 only where data leaves the computer (master, AI, git).
- [x] How the server is started: flags. `--port N` (default 8080); `--address IP` always means the TLS server (step 4); `--data-dir DIR` (default `~/.local/share/io-master`).
- [x] The role is chosen at the first start, in the web interface, together with the first user, unlocked by a one-time setup code printed in the terminal (`architecture.md`, section 3.2).
- [x] Pages: plain HTML and CSS built into the program, no separate frontend build.

**Tasks**
- [x] First start: the setup page (setup code, first user, password twice, role, confirmation); afterwards the page is gone and the first user is logged in. **TODO: until steps 2–3 the state is kept in `installation.json` in the data directory; the password is checked on the form but not kept.**
- [x] A damaged installation file stops the program.
- [x] Flags `--port`, `--address` and `--data-dir`; `--address` is recognized and refused with an explanation until step 4.
- [x] HTTP server listening only on `127.0.0.1`.
- [x] Protection of a local server: only the `Host` names of the program are answered (DNS rebinding); requests that change something need the `Origin` of the program (cross-site requests).
- [x] Layout: navigation on the left, the page on the right.
- [x] Pages of the workstation: Chat (placeholder), Scan and permissions (placeholder), Settings (placeholder).
- [x] Page of the master: password reset panel (placeholder). Each role refuses the pages of the other.
- [x] Red bar on every page while no password reset is configured (always, for now).
- [x] Mock-up login: a login page and a session cookie (`HttpOnly`, `SameSite=Strict`); only users of the installation are accepted, the password is not checked yet. **TODO: real login in step 5.**
- [x] Security headers: Content-Security-Policy, no framing, no MIME sniffing, no referrer.
- [x] `main.rs` reads the configuration and starts the server; Ctrl+C stops it cleanly.
- [x] The random bytes of the kernel moved to `random/random.rs`, shared by the database key and the session tokens.

**Done when:** the browser opens the pages on `http://127.0.0.1:PORT/`; tests cover the configuration, routes, role blocking, the Host and Origin checks and the headers.

---

## Step 2: Secrets in the TPM

> **Changed after step 3:** the TPM is an **optional, further factor**, not a requirement, because the program must also run on machines without a TPM (many virtual machines). By default the program uses passwords only. The TPM modules built here stay and are switched on as a factor in step 8.

**Needs:** packages `libtss2-dev` and `swtpm` (installed). Until the beta version the program uses only the emulator; access to the real TPM (group `tss`) is needed from step 8.

**Decisions**
- [x] Library: `tss-esapi`.
- [x] The secret of the device: 32 random bytes sealed under the primary key of the TPM (derived by the TPM on every use, never stored). The sealed envelope is a file in the data directory, readable only by the user; it is useless without the TPM of this computer.
- [x] A marker in the non-volatile memory of the TPM (one small index, written once at installation) holds the identifier of the installation; it tells the states of `architecture.md`, section 3.2 and refuses a second installation.
- [x] No binding to the state of the machine (PCR) for now.
- [x] The owner password of the TPM is assumed empty; the program never clears the TPM or changes its passwords.
- [x] Until the beta version the program uses only the emulator `swtpm`, which it starts itself in the data directory; the real TPM comes in step 8. The tests start their own emulators.

**Tasks**
- [x] The TPM emulator (`tpm/emulator.rs`): started and stopped by the program, its state in `<data directory>/tpm-emulator/`; an emulator left running after a crash is stopped at the next start.
- [x] Access to the TPM (`tpm/tpm.rs`): encrypted sessions, sealing and unsealing, the marker in the non-volatile memory.
- [x] The secret of the device (`tpm/device_secret.rs`, `architecture.md` section 5.2) created and sealed at the first start; unsealed at every start to prove the installation.
- [x] The states of `architecture.md`, section 3.2 (`installation/lifecycle.rs`): a marker without a record, a record without the marker, a record of another installation, or an interrupted first start stops the program.
- [x] One installation per TPM: a second installation is refused. With the emulator every data directory has its own emulated TPM, so this protects a computer only with the real TPM (step 8).
- [x] `io-master --uninstall` (asks for a typed confirmation): removes the marker from the TPM, the sealed secret, the record and the emulator state; works on a damaged installation too.
- [x] Tests against `swtpm`, which never touch the real TPM.
- [x] Remove the Secret Service keyring (`keyring/`); the type of the database key moves to `db/database_key.rs`.

**Done when:** the secret survives a restart, cannot be read without the TPM, and the tests pass on `swtpm`.

---

## Step 3: Databases, accounts and the management of users

**Needs:** SQLCipher (installed; bundled or system library decided below).

**Decisions**
- [x] Database: SQLite encrypted with SQLCipher (`architecture.md`, section 5.3).
- [x] Two levels: the system database (key sealed in the TPM only) and the databases of the evaluators (password + secret of the device).
- [x] SQLCipher bundled into the program (`rusqlite`, feature `bundled-sqlcipher`), on the OpenSSL of the system.
- [x] One database per evaluator (`users/<id>.db`) next to the system database (`system.db`).
- [x] Argon2id with 64 MiB and 3 passes (OWASP); the key of a database is wrapped with AES-256-GCM under a key derived (HKDF-SHA256) from Argon2id(password) and the secret of the device, and bound to the account name.
- [x] Password rules: 12 to 1024 characters, nothing else (NIST).

**Tasks**
- [x] Encrypted databases (`db/db.rs`): files readable only by the user; every new database holds its schema version at once, so even an empty one opens only with its key.
- [x] System database (`db/system_db.rs`): key derived from the secret of the device, opened at program start without a password (`architecture.md`, section 5.3). It is the record of the installation; `installation.json` is gone.
- [x] Creating an account (`accounts/accounts.rs`): password rules, a random key for its own database, wrapped with the password and the secret of the device (`db/keys.rs`).
- [x] Login unlocks the key of the database of the evaluator; the session keeps it and logout wipes it. A wrong name and a wrong password get the same answer. This replaces the mock-up login of step 1.
- [x] Change of password (Settings): needs the current password, re-wraps only the key of the database.
- [x] Management of users (Settings): on a workstation a logged-in user adds a further user (equal rights); a master keeps its one user.
- [x] Uninstall also removes the databases.
- [ ] Settings kept in the database of the evaluator: moved to step 5, the first setting is the choice of columns.
- [ ] Changing the password on a master: comes with the master (step 11).

**Done when:** without the password the data cannot be read, also with the TPM present; tests cover creating, opening, changing the password, a wrong password, and adding a user.

---

## Step 4: The TLS server

Moved to the last stage: done right before the master (step 11), which is reached over the network. Until then the program serves only the local interface on 127.0.0.1.

**Decisions**
- [x] Who may change the settings of the installation: on a workstation every user (equal rights); on a master its only user.
- [ ] Whether the TLS settings can be changed after they are saved (for example to replace an expiring certificate) or only by installing again.
- [ ] TLS library: `rustls` with TLS 1.3 only (proposed).
- [ ] Which names the TLS server answers to (`Host` check): the address, and the names in the certificate.

**Tasks**
- [ ] Settings page: the certificate and private key of the TLS server, checked (they belong together, the certificate is valid) and stored in the system database.
- [ ] `--address IP` starts HTTPS with TLS 1.3 only, with the stored settings; refuses with an explanation when none are stored.
- [ ] In TLS mode: the session cookie is `Secure`, the answers carry HSTS.
- [ ] Tests: a TLS 1.2 client is refused; a changed system database is detected.

**Done when:** the TLS server is configured in the local interface, starts with `--address`, and a browser reaches it over TLS 1.3 only.

---

## Step 5: Scan page

Done before step 4, at the request of the user.

**Decisions**
- [x] The page is built by the server (HTML forms, no JavaScript); a JSON API and `web_ui/serializer/` come when a page needs live data.
- [x] The settings of the scanner are on the scan page itself, kept per evaluator in their encrypted database.
- [x] No CSRF token: the check of `Origin` and the `SameSite=Strict` cookie already refuse forms of other websites (step 1).

**Tasks**
- [x] The scan filters by several kinds of port at once, or all (`devices::PortFilter`).
- [x] The properties of a port that can be shown (`devices/port_property.rs`), each with a stable key for storing the choice; default columns: port, type, device, serial number, access.
- [x] The settings of the scanner (`devices/scan_settings.rs`) in the settings table of the evaluator's database (`db/user_db.rs`); forgiving when reading.
- [x] Scan page: kinds of port (with "All kinds") and columns as boxes, "Save and scan", "Scan again", the table of ports with the chosen columns.

**Done when:** the Arduino appears in the table; the chosen settings survive a new login and a restart of the program, and each evaluator has their own; tests cover the filter, the properties, the settings and the page.

---

## Step 6: Port permissions

**Decisions**
- [ ] How access to a port is granted from the browser: (a) a button that runs `pkexec` (system password dialog), or (b) the command shown for the user to copy and run. Proposed: (b) first.
- [ ] Which advice is shown: membership in the group of the port (usually `dialout`) as the lasting solution, `chmod` until reconnect as the quick one.

**Tasks**
- [ ] Access of the current user shown for every port (read, write), with the owner, group and permissions.
- [ ] For a port without access: the explanation and the command(s), per the decision above.
- [ ] Fill `permission/permission.rs` with this logic, independent of the UI.

**Done when:** a port without access is explained on the page with a working command; tests cover the advice for each case.

---

## Step 7: AI chat with Anthropic

The first AI function: a chat with a Claude model of Anthropic, with the API key of the evaluator. Other providers (Gemini) and the use of AI on the data of devices come later.

**Decisions**
- [x] HTTP client: `reqwest` with `rustls`, TLS 1.3 only, certificates verified, redirects never followed, as in v0.2, so the key never reaches another address.
- [x] Settings of the chat on the chat page itself, folded like those of the scanner (step 5), kept per evaluator in their database: the model, chosen from a list of the current Claude models (Opus, Sonnet, Haiku), with a default.
- [x] Replies are shown while they are written (streamed): friendlier for the user. This needs a small script on the chat page, served from the program itself (the Content-Security-Policy allows no other), and server-sent events from the server.
- [x] The key and the chosen model are checked when they are stored, as in v0.2: one short request; a failure is explained in plain words (key rejected, unknown model, no connection) and the key is not stored.
- [x] What the page shows after the check: only that the model is configured correctly and can be used in the chat or to support the evaluation; nothing of the key itself, not even its last characters.

**Tasks: the API key (Settings)**
- [ ] A section "AI" in Settings: entering, replacing and deleting the Anthropic API key of the logged-in evaluator. The field is a password field; the key is **never shown back** in a page, a log or an error message.
- [ ] The key is stored only in the evaluator's database, which is encrypted with a key opened by their password (and by every further factor, steps 8 and 9): **at least the password protects it**. It is never written to a file or to the system database.
- [ ] Transport of the key from the browser: only to the local interface on 127.0.0.1 (later over TLS 1.3, step 4); the form is protected like every form (Origin check, `SameSite=Strict`).
- [ ] In memory: the key is held in a type that wipes it and never prints it (as `DatabaseKey`), read from the database only for a request and dropped right after it.

**Tasks: the chat**
- [ ] AI module (`ai/`): sending a conversation to the Anthropic Messages API, reading the reply and the token counts; errors told apart (key rejected, limit reached, no connection, refusal, overloaded service).
- [ ] Chat page: list of the evaluator's chats, new chat, continue, delete; every message and reply kept in the evaluator's database with its time and model.
- [ ] A reply is shown while it is written; it can be stopped while it is being written.
- [ ] Tests against a fake Anthropic service, without a real key and without network.

**Done when:** an evaluator stores an API key in Settings, the key cannot be read from the files without the password, a conversation with a real Claude model works from the browser, and the tests pass without network.

---

## Step 8: The TPM as an optional factor

Comes after the functions of the workstation (steps 4–7). The program authenticates with passwords only by default (`architecture.md`, section 5). The TPM, built in step 2, becomes a further factor that can be switched on where a TPM exists, for the whole installation.

**Decisions**
- [ ] Where the factor is switched on: at the first start, in Settings afterwards, or both.
- [ ] Switching it on later: every account must log in once to have its key wrapped again with the new factor; what happens to accounts that have not.
- [ ] Whether switching it off is allowed, and what it asks for.

**Tasks**
- [ ] The setting in the system database; the program starts the TPM only when the factor is on.
- [ ] The secret of the device added to the unlocking keys (`db/keys.rs` already takes it as a factor; `with_device_secret` is stored per account).
- [ ] With the factor on: the system database encrypted with a key from the secret of the device, the marker of the installation compared at every start, one installation per computer.
- [ ] `--uninstall` also removes the marker from the TPM.
- [ ] Tests on the emulator `swtpm`.
- [ ] The red bar "No TPM is configured" (shown on every page since step 3) disappears once the factor is on.

**Done when:** the factor can be switched on, an account then needs both the password and the TPM, and the program still works fully without it.

---

## Step 9: Passkeys as an optional factor

A further factor next to the TPM, chosen by each user for their own account (`architecture.md`, section 5.1). The TPM binds the data to the computer, a passkey to the person; either, both or neither can be used.

**Decisions**
- [ ] Library: `webauthn-rs` (proposed); check its support of the PRF extension.
- [ ] What a browser or key without PRF gets: a passkey only for logging in (does not protect the files), or no passkey at all.
- [ ] How many passkeys an account may have (proposed: several, so that a spare key in a drawer prevents the loss of the data).

**Tasks**
- [ ] Registering and removing passkeys in the Settings of an account.
- [ ] Login with password and passkey; the secret from the PRF extension becomes part of the unlocking key (`db/keys.rs`), the key of the database is wrapped once per registered passkey.
- [ ] Tests with a software authenticator.

**Done when:** an account with a passkey opens only with the password and the passkey, and losing one of two registered passkeys loses no data.

---

## Step 10: Before release of the workstation

- [ ] **The TPM factor (step 8) uses a real TPM 2.0 (`/dev/tpmrm0`, the user in the group `tss`) instead of the emulator `swtpm`; the release refuses an emulated TPM.**
- [ ] Without the TPM factor the system database is not encrypted and the private key of the TLS server (step 4) has no key to protect it at program start: decide how it is protected without a TPM.
- [ ] Final login: limits on failed attempts, session expiry, logout everywhere.
- [ ] Security review of the whole program.
- [ ] Installation package: the program and its start as a service; the role is chosen at the first start (step 1).

---

## Step 11: Master and password reset (last stage)

The workstation is the core of the program and is completed first (steps 0–10). The master comes last.

**Needs:** a second machine for the master; a virtual machine with its own virtual TPM (`swtpm`) is enough. Master and workstation never run on the same computer (`architecture.md`, section 3.1).

**Decisions**
- [ ] How a workstation and the master recognize each other (mutual TLS with certificates exchanged at enrollment, proposed).
- [x] The administrator of the master is the user created at its first start (step 1).

**Tasks**
- [ ] Master: API that receives reset requests, a panel where the administrator approves or rejects them, shares sealed in its TPM.
- [ ] Workstation: settings page to configure the reset service; creates the recovery copy of the key of the database and hands the master its share.
- [ ] Reset flow: request, approval, share handed out, new password set on the workstation.
- [ ] The orange bar disappears once a reset service is configured.
- [ ] Settings of a workstation: connect it to a master after installation, change the master, or disconnect it.
- [ ] Every request and decision recorded on the master.

**Done when:** a forgotten password is reset with the master on a second machine; neither side alone can decrypt the data (shown by tests).

---

---

## After this list

Not planned in detail yet; to be designed once the steps above are done:

- Further AI providers (Gemini, local models) and AI on the data of the devices.
- Backup of the databases with its own encryption, independent of the TPM; also the way to move to a new computer (`architecture.md`, section 5.5).
- Git of the laboratory over a secure connection, with checks of its configuration (`architecture.md`, section 6).
- Filling the LaTeX report templates of the laboratory and committing the result.
- Git panel in the web UI.
- The other functions of v0.2 (`v0.2_functionality.md`): UART read and write with the confirmation of every command (P2), Bluetooth.
- A build variant without the code of the master (`architecture.md`, section 3.2).
- Common Criteria support: norm index, document library, the two pipelines, guided execution.
