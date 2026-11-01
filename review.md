# Code review

The record of what the user has reviewed and approved. The review is done by the user, file by file; Claude answers questions and changes code only on request (`CLAUDE.md`, "Code review").

- **A function or a fragment:** the comment `// ok kk` above it. Every line added or removed in it needs the user's approval first; its tests may be changed.
- **A whole file:** the user says so in the chat; Claude writes it below with the date. Every function of an approved file counts as marked `// ok kk`.

A file that changes after its approval goes back to "to review", with a note of what changed, once the user has approved the change.

## Approved files

| File | Approved | Notes |
|---|---|---|
| `io-master/src/infrastructure/storage/lists.rs` | 2026-10-31 | The pilot of the pattern of the comments (`CLAUDE.md`, "Comments"). Moved from `db/models/` on 2026-11-01: its functions `pub` instead of `pub(in crate::db)`, its `Called by:` naming the full paths. |
| `io-master/src/objects/*/*/<object>_model.rs` (every model) | 2026-10-31 | The files of `model/`, moved on 2026-11-01 with their content, checked item by item against `7f07b15` (the paths in the heads of `user_keys_model.rs`, `integration_model.rs` and `agent_model.rs` mended). In `integration_model.rs` an example address and `OpenAI` put in backticks (2026-11-01). But `objects/session/progress/progress_model.rs`, new on 2026-11-01 (agreed field by field in the chat), to review. |
| `io-master/src/objects/*/*/<object>_store.rs` (every store) | 2026-10-31 | The files of `db/models/`, moved on 2026-11-01; only their paths of modules and in the comments mended; the functions they share keep a visibility bounded to their folder (`pub(in crate::objects::evaluation)`, `pub(in crate::objects)` instead of `pub(in crate::db)`). In `api_key_store.rs` and `integration_store.rs` the example addresses and names of a column put in backticks, the callers of the settings named anew (2026-11-01). The functions no panel calls removed with their tests, approved (2026-11-01): `create_api_key`, `update_oscal_standard_document`, `read_installation`, `read_user`, `update_user`; the heads of those files and their `Called by:` mended. |
| `io-master/src/objects/schema.rs` | 2026-10-31 | Moved from `db/models/` on 2026-11-01; its head says the caller gives the list to `infrastructure/storage/`, the names of the user stores mended. |
| `io-master/src/infrastructure/storage/error.rs` | 2026-10-31 | Moved from `db/` on 2026-11-01, its first sentence naming the new folders. |
| `io-master/src/infrastructure/storage/user_database.rs`, `system_database.rs` | 2026-10-31 | `encryption/` apart, reviewed later. Moved from `db/storage/` on 2026-11-01; `create`, `open` and `from` take the list of the tables (`tables`) instead of reading `schema.rs`; `connection()` is `pub`, its guard moved to `architecture_tests.rs` (`the_sql_is_only_in_the_store`): the change approved (2026-11-01). `SQLCipher` put in backticks in the head of `user_database.rs` (clippy `pedantic`, approved 2026-11-01). |

## Reviewed later by decision

| File | Why |
|---|---|
| `io-master/src/infrastructure/storage/encryption/` | Moved unchanged from the earlier program; audited and redesigned by the user later. For the audit (2026-11-01): `DatabaseKey::fill_with` takes a closure as a parameter (rule 9); `generate_key.rs`, `database_key.rs` and `sqlcipher.rs` lack `Called by:` at their functions. Names in their comments put in backticks (clippy `pedantic`, 2026-11-01), nothing else changed. |

## To review

Everything else in `io-master/src/` and `io-master/ui/src/`, from the bottom of the groups up (`docs/code_structure.md`): `infrastructure/` (`security/`, `pdf/`, `storage/connection.rs`, `storage/encryption/generate_key.rs`, `clock.rs`, `data_folder.rs`); in `objects/` the rules and the AI of every object (`<object>_rules.rs`, `<object>_ai.rs` and their folders), `error.rs`, `ai_error.rs`, `text.rs`; `panels/session.rs`, `panels/logic/`, `panels/views/`, `main.rs`, `architecture_tests.rs`, `ui/`.
