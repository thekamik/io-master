# IO Master: the open work

A heap of reminders, not a plan in order: the user chooses what is done next. Every step: a concept approved by the user, the code with its tests, a check, the approval, a commit. What is done leaves this list (its history is in git).

## Functions still open

- **The evaluation**: the state Report when the last work unit is assessed (and what the evaluator may still change then); today an evaluation stays in the state Evaluation, and its report is written in that state.

## Before version 1

The beta (`V0.3`, 2026-11-04) has every function; version 1 comes after the full process of tests and verification.

- **The tests and verification of the beta** (user, 2026-11-04): every panel tried on real evaluations (a CEM and an ETSI catalog, real STs, local and cloud models), the reports checked against what the laboratory hands over, the faults found mended before version 1.
- The review of the code by the user, file by file (`review.md`); the functional hardening: a limit of failed logins, a full disk, a damaged database, the limits of the inputs measured on real documents.
- The restriction lints of clippy in `[lints]` of `Cargo.toml` for the rules of `CLAUDE.md` (`unwrap_used`, `panic`, `indexing_slicing` and the like; `pedantic` is on).
- **The checks with recovery** (rule 5): 212 `ensure!` in 517 functions (0.41 on average, the rule asks two), counted on 2026-11-02 after the silent defaults were replaced; added in the review file by file, only for what the types do not guarantee (the limits of an input, the state a function expects), never to fill the average.
- **The journal grows without a limit** (found 2026-11-10): every question of an assessment keeps its whole body, the documents of the work unit in it (hundreds of kilobytes for each work unit, again for each run). To decide after the tests on a real evaluation, when its size is known (user, 2026-11-10): what a journal keeps of a question that repeats a document, and for how long.
- `cargo-audit` and `cargo-deny`; the package for Debian 13.
- Version 1: from then on the data is kept and every change of a table is a migration. The names of the database files keep their `.v2` until then (user, 2026-10-31: the databases of the tests are not broken for a name).

## Reminders

- **The tokens counted by the service** (user, 2026-11-02): the program estimates the size of a question by `Integration::characters_per_token`, a guard against a local server that shortens a question in silence; after the answer it warns when the service counted fewer than half the estimate (`shortened_warning`). Where the service counts exactly (Anthropic: `/v1/messages/count_tokens`, free; llama.cpp: `/tokenize`), the count could replace the estimate; the chat completions of OpenAI have no such call.

- **The documents kept by the service between the questions** (2026-11-02): every question of an assessment sends the whole ST again (about 55 000 tokens; 30 work units are 1.6 million tokens, a whole CEM many times more). The messages of Anthropic can keep a fixed beginning of the question in their cache (prompt caching, `cache_control`), paid in part: the documents of a work unit put first and marked, the work unit after them. To measure the cost first.

- **The content fitted to a model of a small context** (user, 2026-11-02): a model whose context holds less than an ST of 100 pages (`objects/ai/integration/integration_ai/question.rs`: `needed_context`, 120 000 tokens at three characters of a token) gets the documents of a work unit only as the 20 passages that fit it best, and the AI settings warn of it. To make a way of fitting the content to such a model: the documents parted by their sections (the section a work unit asks about given whole), a summary of the rest, or the assessment of a work unit in several questions.

- **The reading of a whole plan** (measured 2026-11-10 on the CEM, 813 work units, a debug build): `read_work_units` 90 ms with its statements kept ready (212 ms before); all the work units of a standard added in one transaction (0.8 s); a new evaluation 0.8 s; the order of the work 0.1 s (`order.rs` compares every pair of work units, clear and bounded; an index of the dependencies only if a larger catalog needs it).
- **`IdList`** (2026-10-31): a constant per list of ids (its table, its columns, its limit) instead of three statements of SQL at its callers (`infrastructure/storage/lists.rs`); put off while the comments read well.

- An optional second factor of the login.
- An export of a plan and of the results as OSCAL documents (profile, assessment plan, assessment results), when a laboratory asks for it.
- The program measured on a virtual machine of 8 GB RAM with the CEM (813 work units) and a large PDF.
- Embeddings beside the full-text search, if the search proves too weak on real knowledge.
