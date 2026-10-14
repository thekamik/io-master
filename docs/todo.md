# IO Master v0.3: TODO

The steps that take the program from where it is now to the design in `architecture.md`. They are done **in order, one at a time**. The workstation is the core and is completed first; the master (step 9) is the last stage. When the last step is done, the next functionality is planned anew.

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
- [ ] SQLCipher bundled into the program or the system library.
- [ ] One database per evaluator, or one per workstation with separate data per account.
- [ ] Parameters of Argon2id (memory, time) and the cipher that wraps the key of the database.

**Tasks**
- [ ] System database: encrypted with a key sealed in the TPM only, opened at program start without a password (`architecture.md`, section 5.3).
- [ ] The role and the users move from `installation.json` into the system database; the file goes away.
- [ ] Creating an account: password rules, Argon2id, wrapping of the key of the database with the password and the secret of the device.
- [ ] Login unlocks the database of the evaluator; logout wipes the key from memory. This replaces the mock-up login of step 1.
- [ ] Change of password: re-wraps only the key of the database.
- [ ] Management of users: on a workstation a logged-in user adds a further user (equal rights); a master keeps its one user.
- [ ] Settings are kept in the database of the evaluator.

**Done when:** without the password the data cannot be read, also with the TPM present; tests cover creating, opening, changing the password, a wrong password, and adding a user.

---

## Step 4: The TLS server

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

**Decisions**
- [ ] Shape of the JSON API, for example `GET /api/ports?type=ttyACM`.

**Tasks**
- [ ] `web_ui/serializer/`: ports (`SerialPortInfo`) to JSON.
- [ ] API endpoint for the scan, with the optional filter by port type.
- [ ] Table of ports on the page, a filter by type, a button to scan again.
- [ ] Settings page: choice of the columns of the table among all properties of a port; default columns: port, type, device, serial number, access.
- [ ] Settings kept in the database of the evaluator (step 3).
- [ ] Protection of requests that change something (CSRF token, in addition to `SameSite=Strict`).

**Done when:** the Arduino appears in the table; the chosen columns survive a restart of the program; tests cover the serializer, the endpoint and the settings file.

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

## Step 7: AI and chat

**Decisions**
- [ ] HTTP client with `rustls` and TLS 1.3 only (proposed: `reqwest`).
- [ ] Providers to start with: Anthropic and Gemini, as in v0.2.
- [ ] Replies shown when complete, or streamed while they are written.

**Tasks**
- [ ] API keys of the evaluator stored in the database; a settings page to enter, replace and delete them (the key is never shown back).
- [ ] Provider interface and the two providers; errors told apart (key rejected, limit reached, no connection, refusal).
- [ ] Chat page: list of chats, new chat, continue, delete; history in the database.
- [ ] Cancelling a request that is waiting for a reply.
- [ ] Tests against a fake AI service, without a real key.

**Done when:** a conversation with a real model works from the browser; the key is never written to a file or shown in a page; the tests pass without network.

---

## Step 8: Before release of the workstation

- [ ] **Replace the TPM 2.0 emulator `swtpm` with a real TPM 2.0 (`/dev/tpmrm0`, the user in the group `tss`); the release refuses to run on an emulated TPM.** Until then the program uses only the emulator (step 2).
- [ ] Final login: limits on failed attempts, session expiry, logout everywhere.
- [ ] Security review of the whole program.
- [ ] Installation package: the program and its start as a service; the role is chosen at the first start (step 1).

---

## Step 9: Master and password reset (last stage)

The workstation is the core of the program and is completed first (steps 0–8). The master comes last.

**Needs:** a second machine for the master; a virtual machine with its own virtual TPM (`swtpm`) is enough. Master and workstation never run on the same computer (`architecture.md`, section 3.1).

**Decisions**
- [ ] How a workstation and the master recognize each other (mutual TLS with certificates exchanged at enrollment, proposed).
- [x] The administrator of the master is the user created at its first start (step 1).

**Tasks**
- [ ] Master: API that receives reset requests, a panel where the administrator approves or rejects them, shares sealed in its TPM.
- [ ] Workstation: settings page to configure the reset service; creates the recovery copy of the key of the database and hands the master its share.
- [ ] Reset flow: request, approval, share handed out, new password set on the workstation.
- [ ] The red bar disappears once a reset service is configured.
- [ ] Settings of a workstation: connect it to a master after installation, change the master, or disconnect it.
- [ ] Every request and decision recorded on the master.

**Done when:** a forgotten password is reset with the master on a second machine; neither side alone can decrypt the data (shown by tests).

---

---

## After this list

Not planned in detail yet; to be designed once the steps above are done:

- Backup of the databases with its own encryption, independent of the TPM; also the way to move to a new computer (`architecture.md`, section 5.5).
- Git of the laboratory over a secure connection, with checks of its configuration (`architecture.md`, section 6).
- Filling the LaTeX report templates of the laboratory and committing the result.
- Git panel in the web UI.
- The other functions of v0.2 (`v0.2_functionality.md`): UART read and write with the confirmation of every command (P2), Bluetooth.
- A build variant without the code of the master (`architecture.md`, section 3.2).
- Common Criteria support: norm index, document library, the two pipelines, guided execution.
