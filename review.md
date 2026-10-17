# Code review (step 12 of docs/todo.md)

The record of what the user has reviewed and approved. The review is
done by the user, file by file; Claude answers questions and changes
code only on request (CLAUDE.md, section "Code review").

Two levels of approval:

- **A function or a fragment:** the comment `// ok kk` above it in the
  code. Every line added or removed in it needs the user's approval
  first. Its tests (`<module>_tests.rs`) may be changed.
- **A whole file:** the user says so in the chat; Claude writes it in
  the table below with the date. Every function of an approved file is
  treated as marked `// ok kk`, even where the comment is missing.

A file that changes after its approval goes back to "to review"
(with a note of what changed) once the user has approved the change.

The review started anew on 2026-10-16 with the code of v0.4 (the
desktop window, Tauri 2 + Svelte); approvals made on v0.3 do not
count.

## Approved files

| File | Approved | Notes |
|---|---|---|
| `io-master/src/main.rs` | 2026-10-16 | Whole file, approved again after the module lists of steps 15–22 (`mod ai`, `mod evaluation`, `mod tauri_ui`); the commented-out block `mod web_ui` removed by the user. **Changed after the approval (2026-10-16, decided by the user), waiting for the user's look:** `tauri_startup` moved here from `tauri_ui/startup.rs` (file removed), its handler list grouped by area with the `*_actions.rs` file of each group; in `main` only the comment and the call of `tauri_startup` changed. **Changed again (steps 24 to 27, 2026-10-16):** the module lists (`evaluation::contents`, `evaluation::projects`, `tauri_ui::model_run`, `tauri_ui::project_actions`, `serializer::project_view`) and the handler list of `tauri_startup` (the commands of the test projects); the function `main` is unchanged. **Changed again (step 29, 2026-10-17):** the module list `mod product_evaluation` added; nothing else. **Changed again (step 37, 2026-10-17):** the module list `mod db` rebuilt for the new folders of `db/`; nothing else. **Changed again (step 38, 2026-10-17):** the module lists `mod ai` and `mod db` (`db/ai/` subfolders), `mod tauri_ui` without `model_run`; nothing else. **Changed again (step 39, 2026-10-17):** the module lists `mod standards`, `mod test_projects`, `mod evaluation` (was `product_evaluation`); nothing else. |

## Reviewed later by decision

| File | Why |
|---|---|
| `io-master/src/installation/verify.rs` | Rebuilt in step 13 (keyring, secret of the application). |
| `io-master/src/installation/uninstall.rs` | Not compiled; "DO NOT USE: TO BE REWORKED" (uninstall moves to the window). |

## To review

Everything else in `io-master/src/` and `io-master/ui/`.
