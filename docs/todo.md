# IO Master v0.3: TODO

The steps that take the program from where it is now to the design in `architecture.md`. They are done **in order, one at a time**. When the last step is done, the next functionality is planned anew.

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
- [ ] Web framework: `axum` on `tokio` (proposed), TLS with `rustls` (TLS 1.3 only).
- [ ] TLS certificate of the workstation: a local certificate authority created at first start, whose certificate the user adds to the browser once (proposed).
- [ ] Where the configuration lives (role, address, port), for example `~/.config/io-master/config.toml`, and its format.
- [ ] How the page is built: plain HTML, CSS and a little JavaScript embedded in the program (proposed), no separate frontend build.

**Tasks**
- [ ] Configuration file with the role (`workstation` or `master`), fixed at installation; the program refuses to start without it.
- [ ] HTTPS server listening on `127.0.0.1` by default; TLS 1.2 and older refused.
- [ ] Creation of the local certificate authority and of the server certificate. The private keys are files readable only by the user for now. **TODO: move them into the TPM in step 4.**
- [ ] Layout: navigation on the left, the page on the right.
- [ ] Pages of the workstation: Chat (placeholder), Scan and permissions (placeholder), Settings (placeholder).
- [ ] Page of the master: administration panel (placeholder). Each role refuses the pages of the other.
- [ ] Red bar on every page while no password reset is configured (always, for now).
- [ ] Warning bar in single-host mode.
- [ ] Mock-up login: a login page and a session cookie (`Secure`, `HttpOnly`, `SameSite=Strict`); any name is accepted. **TODO: real login in step 5.**
- [ ] Security headers: Content-Security-Policy, HSTS, no framing, no MIME sniffing.
- [ ] `main.rs` starts the server; `--cli` is gone.

**Done when:** the browser opens the pages over HTTPS without a certificate warning (after the authority was added once); a TLS 1.2 client is refused; tests cover routes, role blocking and headers.

---

## Step 2: Scan page

**Decisions**
- [ ] Shape of the JSON API, for example `GET /api/ports?type=ttyACM`.
- [ ] Place and format of the session file, for example `~/.config/io-master/session.json`.

**Tasks**
- [ ] `web_ui/serializer/`: ports (`SerialPortInfo`) to JSON.
- [ ] API endpoint for the scan, with the optional filter by port type.
- [ ] Table of ports on the page, a filter by type, a button to scan again.
- [ ] Settings page: choice of the columns of the table among all properties of a port; default columns: port, type, device, serial number, access.
- [ ] Settings kept in the session file until the database exists (step 5).
- [ ] Protection of requests that change something (CSRF token, in addition to `SameSite=Strict`).

**Done when:** the Arduino appears in the table; the chosen columns survive a restart of the program; tests cover the serializer, the endpoint and the settings file.

---

## Step 3: Port permissions

**Decisions**
- [ ] How access to a port is granted from the browser: (a) a button that runs `pkexec` (system password dialog), or (b) the command shown for the user to copy and run. Proposed: (b) first.
- [ ] Which advice is shown: membership in the group of the port (usually `dialout`) as the lasting solution, `chmod` until reconnect as the quick one.

**Tasks**
- [ ] Access of the current user shown for every port (read, write), with the owner, group and permissions.
- [ ] For a port without access: the explanation and the command(s), per the decision above.
- [ ] Fill `permission/permission.rs` with this logic, independent of the UI.

**Done when:** a port without access is explained on the page with a working command; tests cover the advice for each case.

---

## Step 4: Secrets in the TPM

**Needs:** the user in the group `tss` (access to `/dev/tpmrm0`); packages `libtss2-dev`, `tpm2-tools`, `swtpm`.

**Decisions**
- [ ] Library: `tss-esapi` (proposed).
- [ ] How secrets are sealed: under a primary key of the TPM, with the sealed objects stored as files in the data directory.
- [ ] Whether a secret is bound to the state of the machine (PCR values). Binding protects against a modified system but makes updates harder.

**Tasks**
- [ ] `SecretStore` implemented on the TPM: store, load, delete, contains.
- [ ] The secret of the device (`architecture.md`, section 5.2) created and sealed at first start.
- [ ] Private keys of the TLS certificates (step 1) moved into the TPM.
- [ ] Tests against `swtpm`, so they run without touching the real TPM.
- [ ] Remove the Secret Service keyring (`keyring/` on `secret-service`).

**Done when:** secrets survive a restart, cannot be read without the TPM, and the tests pass on `swtpm`; the program works with the real TPM of the development laptop.

---

## Step 5: Accounts and encrypted database

**Needs:** SQLCipher (packaged or compiled into the program; decided below).

**Decisions**
- [ ] Database: SQLCipher through `rusqlite` (proposed); bundled or system library.
- [ ] One database per evaluator, or one per workstation with separate data per account.
- [ ] Parameters of Argon2id (memory, time) and the cipher that wraps the key of the database.

**Tasks**
- [ ] Creating an account: password rules, Argon2id, wrapping of the key of the database with the password and the device secret.
- [ ] Login unlocks the database; logout wipes the key from memory. This replaces the mock-up login of step 1.
- [ ] Change of password: re-wraps only the key of the database.
- [ ] Settings move from the session file into the database.
- [ ] Several evaluators on one workstation.

**Done when:** without the password the data cannot be read, also with the TPM present; tests cover creating, opening, changing the password and a wrong password.

---

## Step 6: AI and chat

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

## Step 7: Master and password reset

**Needs:** for development, `swtpm` for the master on the same laptop (single-host mode).

**Decisions**
- [ ] How a workstation and the master recognize each other (mutual TLS with certificates exchanged at enrollment, proposed).
- [ ] How the administrator account of the master is created at installation.

**Tasks**
- [ ] Master: API that receives reset requests, a panel where the administrator approves or rejects them, shares sealed in its TPM.
- [ ] Workstation: settings page to configure the reset service; creates the recovery copy of the key of the database and hands the master its share.
- [ ] Reset flow: request, approval, share handed out, new password set on the workstation.
- [ ] The red bar disappears once a reset service is configured.
- [ ] Every request and decision recorded on the master.

**Done when:** a forgotten password is reset in single-host mode with `swtpm`; neither side alone can decrypt the data (shown by tests).

---

## Step 8: Before release

- [ ] **Replace the TPM 2.0 emulator `swtpm` with a real TPM 2.0; the release refuses to run on an emulated TPM.**
- [ ] Final login: limits on failed attempts, session expiry, logout everywhere.
- [ ] Security review of the whole program.
- [ ] Installation: role chosen at installation, configuration created, certificate authority shown to the user.
- [ ] Rewrite `README.md` for v0.3.

---

## After this list

Not planned in detail yet; to be designed once the steps above are done:

- Git of the laboratory over a secure connection, with checks of its configuration (`architecture.md`, section 6).
- Filling the LaTeX report templates of the laboratory and committing the result.
- Git panel in the web UI.
- The other functions of v0.2 (`v0.2_functionality.md`): UART read and write with the confirmation of every command (P2), Bluetooth.
- Common Criteria support: norm index, document library, the two pipelines, guided execution.
