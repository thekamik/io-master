# IO Master: the open work

A heap of reminders, not a plan in order: the user chooses what is done next. Every step: a concept approved by the user, the code with its tests, a check, the approval, a commit. What is done leaves this list (its history is in git).

## The rebuild: the parts to come back

- **The evaluation**: the state Report when the last work unit is assessed (and what the evaluator may still change then); the report itself; an assessment that keeps running when the evaluator leaves the evaluation on the page (today the page loses its progress, the work goes on).
- **The login with the recovery code**, the change of the password and a new recovery code.

## Before the release

- The review of the code by the user, file by file (`review.md`); the functional hardening: a limit of failed logins, a full disk, a damaged database, the limits of the inputs measured on real documents.
- `[lints]` in `Cargo.toml` for the rules of `CLAUDE.md` (clippy `pedantic` and the restriction lints).
- `cargo-audit` and `cargo-deny`; the package for Debian 13.
- Version 1: from then on the data is kept and every change of a table is a migration. The names of the database files keep their `.v2` until then (user, 2026-10-31: the databases of the tests are not broken for a name).

## Reminders

- **The reading of a whole plan, measured** (2026-10-31): `read_work_units` reads each work unit apart, five queries each (about 4000 for the CEM), every query parsed anew (`prepare`). To measure first; then `prepare_cached`, or each list read once for the whole plan.
- **`IdList`** (2026-10-31): a constant per list of ids (its table, its columns, its limit) instead of three statements of SQL at its callers (`db/models/lists.rs`); put off while the comments read well.

- An optional second factor of the login.
- An export of a plan and of the results as OSCAL documents (profile, assessment plan, assessment results), when a laboratory asks for it.
- The program measured on a virtual machine of 8 GB RAM with the CEM (813 work units) and a large PDF.
- Embeddings beside the full-text search, if the search proves too weak on real knowledge.
