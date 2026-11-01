# Code review

The record of what the user has reviewed and approved. The review is done by the user, file by file; Claude answers questions and changes code only on request (`CLAUDE.md`, "Code review").

- **A function or a fragment:** the comment `// ok kk` above it. Every line added or removed in it needs the user's approval first; its tests may be changed.
- **A whole file:** the user says so in the chat; Claude writes it below with the date. Every function of an approved file counts as marked `// ok kk`.

A file that changes after its approval goes back to "to review", with a note of what changed, once the user has approved the change.

## Approved files

| File | Approved | Notes |
|---|---|---|
| `io-master/src/infrastructure/storage/lists.rs` | 2026-10-31 | The pilot of the pattern of the comments (`CLAUDE.md`, "Comments"). Moved from `db/models/` on 2026-11-01: its functions `pub` instead of `pub(in crate::db)`, its `Called by:` naming the full paths. |
| `io-master/src/model/` (every file) | 2026-10-31 | But `model/session/progress.rs`, new on 2026-11-01 (agreed field by field in the chat), to review. |
| `io-master/src/infrastructure/storage/error.rs` | 2026-10-31 | Moved from `db/` on 2026-11-01, its first sentence naming the new folders. |
| `io-master/src/infrastructure/storage/user_database.rs`, `system_database.rs` | 2026-10-31 | `encryption/` apart, reviewed later. Moved from `db/storage/` on 2026-11-01; **changed, to review**: `create`, `open` and `from` take the list of the tables (`tables`) instead of reading `db/models/schema.rs`; `connection()` is `pub`, its guard moved to `architecture_tests.rs` (`the_sql_is_only_in_the_store`). |
| `io-master/src/db/models/` (every file) | 2026-10-31 | Only the comments `Called by:` changed on 2026-11-01, to name the new places of the callers; the change approved. |

## Reviewed later by decision

| File | Why |
|---|---|
| `io-master/src/infrastructure/storage/encryption/` | Moved unchanged from the earlier program; audited and redesigned by the user later. |

## To review

Everything else in `io-master/src/` and `io-master/ui/src/`, from the bottom of the layers up (`docs/code_structure.md`): `infrastructure/` (`security/`, `pdf/`, `storage/connection.rs`, `clock.rs`, `data_folder.rs`), `ai/`, `session/`, `panels/logic/`, `panels/views/`, `main.rs`, `ui/`.
