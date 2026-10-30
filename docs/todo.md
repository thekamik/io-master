# IO Master: the open work

A heap of reminders, not a plan in order: the user chooses what is done next. Every step: a concept approved by the user, the code with its tests, a check, the approval, a commit. What is done leaves this list (its history is in git).

## The rebuild: the parts to come back

- **The evaluation** (concept agreed with the user, 2026-10-30; built in steps: a new evaluation and its inputs; the question of one work unit and its answer; the loop; the page; the corrections of the evaluator and the state Report):
  - A plan copied for one product (`create_evaluation` exists); the documents of the product (`.pdf`, `.txt`, `.md`) and the proofs of the laboratory given.
  - The work units assessed by the AI in the order of the work (Kahn: first those without dependencies, then those whose dependencies are assessed), without the evaluator; the progress shown, "Stop" after the present work unit; before the start, how many questions go to which integrations.
  - The question of a work unit: the instructions of its agent, the work unit whole as JSON but `id`, `author`, `ai_agent` and `cross_reference` (the ids written as names), the label, verdict and findings of each dependency, the text of its documents and proofs (whole when they fit, otherwise the passages that fit the work unit best, indexed when the document is given), the knowledge of the agent.
  - The answer: findings, analysis, vulnerability notes, notes, quotes, verdict (one of the plan). A quote is only a fragment of a document, word for word; one the program does not find in its document is not kept as a quote but written into `notes`. The verdict is the worst of its own and those of the dependencies; a work unit is assessed even when a dependency failed.
  - A work unit with a verdict is passed over; "Assess again" assesses anew those of `author = Ai`; a work unit of `author = User` is never written by the AI.
  - A work unit without its own agent is assessed by the agent of the evaluation (`EvaluationPlan::ai_agent`), chosen by the evaluator when the evaluation begins. Every document and proof the work units need is checked before the start: the assessment does not start while one is missing, so that it runs through the night without the evaluator.
  - Documents are never removed. A quote is only of a document of the product (user, 2026-10-30: the standard is always at hand, the contribution of the manufacturer is assessed). 
- **The login with the recovery code**, the change of the password and a new recovery code.
- **`Settings::active_integration`**: has no use since every agent names its integration; to be decided with the user (removed, or kept for something else: a chat is always held with an agent, user 2026-10-30).

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
