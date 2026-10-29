# IO Master: the open work

A heap of reminders, not a plan in order: the user chooses what is done next. Every step: a concept approved by the user, the code with its tests, a check, the approval, a commit. What is done leaves this list (its history is in git).

## The rebuild: the parts to come back

- **The chat** with an agent: the answer streamed into the page, "Stop" (the receiver of the pieces dropped).
- **The evaluation**: a plan copied for one product (`create_evaluation` exists), the documents and proofs given, each work unit assessed by its agent in the order of the work, the quotes checked, the verdict, the report.
- **The login with the recovery code**, the change of the password and a new recovery code.
- **`Settings::active_integration`**: has no use since every agent names its integration; to be decided with the user (removed, or used by a chat without an agent).

## Before the release

- The review of the code by the user, file by file (`review.md`); the functional hardening: a limit of failed logins, a full disk, a damaged database, the limits of the inputs measured on real documents.
- `[lints]` in `Cargo.toml` for the rules of `CLAUDE.md` (clippy `pedantic` and the restriction lints).
- `cargo-audit` and `cargo-deny`; the package for Debian 13.
- Version 1: from then on the data is kept and every change of a table is a migration.

## Reminders

- An optional second factor of the login.
- An export of a plan and of the results as OSCAL documents (profile, assessment plan, assessment results), when a laboratory asks for it.
- The program measured on a virtual machine of 8 GB RAM with the CEM (813 work units) and a large PDF.
- Embeddings beside the full-text search, if the search proves too weak on real knowledge.
