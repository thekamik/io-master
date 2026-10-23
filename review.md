# Code review (step 12 of docs/todo.md)

The record of what the user has reviewed and approved. The review is
done by the user, file by file; Claude answers questions and changes
code only on request (CLAUDE.md, section "Code review").

Two levels of approval:

- **A function or a fragment:** the comment `// ok kk` above it in the
  code. Every line added or removed in it needs the user's approval
  first. Its tests (`<file>_tests.rs`) may be changed.
- **A whole file:** the user says so in the chat; Claude writes it in
  the table below with the date. Every function of an approved file is
  treated as marked `// ok kk`, even where the comment is missing.

A file that changes after its approval goes back to "to review"
(with a note of what changed) once the user has approved the change.

**The review started anew on 2026-10-17 with the code of step 40** (the
layout of `docs/code_structure.md`). No approval of the code before
step 40 moved with it (decision of the user, 2026-10-17); the approval of
that `main.rs` is kept in the history of git (commit `fbc3c47`). The
markers `// ok kk` still in the code of `main.rs` count again only when
the user confirms them.

## Approved files

| File | Approved | Notes |
|---|---|---|

## Reviewed later by decision

| File | Why |
|---|---|
| `io-master/src/installation/verify.rs` | Rebuilt in step 13 (keyring, secret of the application). |
| `io-master/src/installation/uninstall.rs` | Not compiled; "DO NOT USE: TO BE REWORKED" (uninstall moves to the window). |
| `io-master/src/db/encryption/` | Moved unchanged; audited and redesigned by the user later (CLAUDE.md, "Database code"). |

## To review

Everything in `io-master/src/` and `io-master/ui/`. A suggested order,
from the bottom of the layers up (`docs/code_structure.md`): `model/`,
`db/`, `security/`, `ai/`, `hardware_tests/`, `pdf/`, `installation/`,
`app/`, `views/`, `main.rs`, `ui/`.
