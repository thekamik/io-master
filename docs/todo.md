# IO Master: the open work

A heap of reminders, not a plan in order: the user chooses what is done next. Every step: a concept approved by the user, the code with its tests, a check, the approval, a commit. What is done leaves this list (its history is in git).

## The rebuild: the parts to come back

- **The evaluation**: the state Report when the last work unit is assessed (and what the evaluator may still change then); the report itself.

## Before the release

- The review of the code by the user, file by file (`review.md`); the functional hardening: a limit of failed logins, a full disk, a damaged database, the limits of the inputs measured on real documents.
- The restriction lints of clippy in `[lints]` of `Cargo.toml` for the rules of `CLAUDE.md` (`unwrap_used`, `panic`, `indexing_slicing` and the like; `pedantic` is on).
- **The checks with recovery** (rule 5, 2026-11-01): 200 `ensure!` in 503 functions (0.40 on average, the rule asks two): added in the review file by file, for what the types do not guarantee (the limits of an input, the state a function expects).
- `cargo-audit` and `cargo-deny`; the package for Debian 13.
- Version 1: from then on the data is kept and every change of a table is a migration. The names of the database files keep their `.v2` until then (user, 2026-10-31: the databases of the tests are not broken for a name).

## Reminders

- **The parts of a control beyond its statement and guidance** (2026-11-01): the catalog of ETSI TS 103 701 (`testing_documents/`) has no part `guidance`; its parts `test-units`, `assessment-method` and `assignment-of-verdict` are kept in the stored catalog but reach no work unit (`objects/evaluation/oscal_standard_document/oscal_standard_document_rules.rs`: `control_of` takes only `statement` and `guidance`). To decide whether and how a work unit shows them.

- **The reading of a whole plan, measured** (2026-10-31): `read_work_units` reads each work unit apart, five queries each (about 4000 for the CEM), every query parsed anew (`prepare`). To measure first; then `prepare_cached`, or each list read once for the whole plan.
- **`IdList`** (2026-10-31): a constant per list of ids (its table, its columns, its limit) instead of three statements of SQL at its callers (`infrastructure/storage/lists.rs`); put off while the comments read well.

- An optional second factor of the login.
- An export of a plan and of the results as OSCAL documents (profile, assessment plan, assessment results), when a laboratory asks for it.
- The program measured on a virtual machine of 8 GB RAM with the CEM (813 work units) and a large PDF.
- Embeddings beside the full-text search, if the search proves too weak on real knowledge.
