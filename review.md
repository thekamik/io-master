# Code review

The record of what the user has reviewed and approved. The review is done by the user, file by file; Claude answers questions and changes code only on request (`CLAUDE.md`, "Code review").

- **A function or a fragment:** the comment `// ok kk` above it. Every line added or removed in it needs the user's approval first; its tests may be changed.
- **A whole file:** the user says so in the chat; Claude writes it below with the date. Every function of an approved file counts as marked `// ok kk`.

A file that changes after its approval goes back to "to review", with a note of what changed, once the user has approved the change.

## Approved files

| File | Approved | Notes |
|---|---|---|
| `io-master/src/db/models/lists.rs` | 2026-10-31 | The pilot of the pattern of the comments (`CLAUDE.md`, "Comments"). |
| `io-master/src/model/` (every file) | 2026-10-31 | |
| `io-master/src/db/error.rs` | 2026-10-31 | |
| `io-master/src/db/storage/user_database.rs`, `system_database.rs` | 2026-10-31 | `encryption/` apart, reviewed later. |
| `io-master/src/db/models/` (every file) | 2026-10-31 | |

## Reviewed later by decision

| File | Why |
|---|---|
| `io-master/src/db/storage/encryption/` | Moved unchanged from the earlier program; audited and redesigned by the user later. |

## To review

Everything else in `io-master/src/` and `io-master/ui/src/`, from the bottom of the layers up (`docs/code_structure.md`): `security/`, `pdf/`, `ai/`, `session/`, `panels/logic/`, `panels/views/`, `main.rs`, `ui/`.
