//! The screen of a task of an evaluation read for the page (step 52 of
//! docs/todo.md): the frozen scheme and related elements, the requirement
//! from its catalog, and what the evaluator added (`db/evaluation/`). Every
//! command that changes a task answers with it, so the page always shows
//! what the program keeps.

use crate::ai::tasks::candidates::requirement_text;
use crate::app::error::AppError;
use crate::components::part_rows::view as part_rows;
use crate::db::evaluation as db_evaluation;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::model::evaluation::{LabTask, output_applies};
use crate::model::oscal::catalog::Control;
use crate::model::task_scheme::{BlockKind, ResultText};

use super::view::{
    EvidenceView, FragmentView, InputView, LabBlockView, LabConnectionView, LabRelatedView, LabRequirementView,
    LabTaskScreenView, MessageView, OutputView, ProposalView, ResultsForm,
};

/// Task `task` of evaluation `evaluation`; refused when it does not exist.
///
/// Called by: the commands of a task (`evaluation_lab.rs`), [`screen`].
pub fn task(db: &UserDb, evaluation: &str, task: &str) -> Result<LabTask, AppError> {
    match db_evaluation::read::task(db, evaluation, task)? {
        Some(found) => Ok(found),
        None => Err(AppError::Invalid("the task does not exist".to_owned())),
    }
}

/// The requirement of `task` from its catalog, and its text for a question
/// to the AI (`candidates::requirement_text`); refused when the catalog has
/// it no more.
///
/// Called by: [`screen`], the works of the AI of a task.
pub fn requirement(db: &UserDb, task: &LabTask) -> Result<(Control, String, String), AppError> {
    let catalog = db_oscal::read::catalog(db, &task.catalog_uuid)?;

    let control = match catalog.locate(&task.control_id)? {
        Some(location) => catalog.control_at(&location),
        None => None,
    };
    let Some(control) = control else {
        return Err(AppError::Invalid("the catalog has the requirement no more".to_owned()));
    };

    let text = requirement_text(&task.label, control).map_err(AppError::Invalid)?;

    Ok((control.clone(), text, catalog.metadata.title.clone()))
}

/// The screen of task `task` of evaluation `evaluation`.
///
/// Called by: `lab_task_get` and every command that changes a task (`evaluation_lab.rs`).
/// Calls: [`task`], [`requirement`], `part_rows::rows`, [`inputs`], [`outputs`], `db_evaluation::read::*`.
pub fn screen(db: &UserDb, evaluation: &str, uuid: &str) -> Result<LabTaskScreenView, AppError> {
    let found = task(db, evaluation, uuid)?;
    let (control, _, catalog_title) = requirement(db, &found)?;
    let blocks = db_evaluation::read::blocks(db, evaluation, uuid)?;
    let results = db_evaluation::read::results(db, evaluation, uuid)?;
    let mut related = Vec::new();

    for element in db_evaluation::read::related(db, evaluation, uuid)? {
        related.push(LabRelatedView { label: element.label, title: element.title });
    }

    let mut connections = Vec::new();

    for connection in db_evaluation::read::connections(db, evaluation, uuid)? {
        connections.push(LabConnectionView { from: connection.from, to: connection.to });
    }

    // A short closure: Option::map, which clippy asks for here.
    let proposal = db_evaluation::read::results_proposal(db, evaluation, uuid)?
        .map(|(proposed, reason)| ProposalView { results: ResultsForm::new(proposed), reason });
    let inputs = inputs(db, evaluation, &blocks)?;
    let outputs = outputs(db, evaluation, &blocks, &results)?;
    let mut block_views = Vec::with_capacity(blocks.len());

    for block in blocks {
        block_views.push(LabBlockView::new(block));
    }

    Ok(LabTaskScreenView {
        uuid: found.uuid,
        state: found.state.name(),
        requirement: LabRequirementView {
            label: found.label,
            title: found.title,
            catalog_title,
            parts: part_rows::rows(&control.parts)?,
        },
        related,
        blocks: block_views,
        connections,
        inputs,
        results: ResultsForm::new(results),
        proposal,
        outputs,
        result_titles: [
            ResultText::Findings.title(),
            ResultText::Analysis.title(),
            ResultText::Assessment.title(),
            ResultText::Vulnerabilities.title(),
        ],
    })
}

/// Every input of `blocks` with its evidence, chat and fragments.
///
/// Called by: [`screen`].
fn inputs(
    db: &UserDb,
    evaluation: &str,
    blocks: &[crate::model::task_scheme::Block],
) -> Result<Vec<InputView>, AppError> {
    let mut found = Vec::new();

    for block in blocks {
        if block.kind != BlockKind::Input {
            continue;
        }

        let mut evidence = Vec::new();
        let mut messages = Vec::new();
        let mut fragments = Vec::new();

        for piece in db_evaluation::read::evidence_of(db, evaluation, &block.uuid)? {
            evidence.push(EvidenceView::new(&piece));
        }

        for turn in db_evaluation::read::messages(db, evaluation, &block.uuid)? {
            messages.push(MessageView::new(turn));
        }

        for fragment in db_evaluation::read::fragments(db, evaluation, &block.uuid)? {
            fragments.push(FragmentView::new(fragment));
        }

        found.push(InputView { block: block.uuid.clone(), evidence, messages, fragments });
    }

    Ok(found)
}

/// Every output of `blocks` with its text and whether its condition holds
/// for the verdict of `results`.
///
/// Called by: [`screen`].
/// Calls: `output_applies`, `db_evaluation::read::output`.
fn outputs(
    db: &UserDb,
    evaluation: &str,
    blocks: &[crate::model::task_scheme::Block],
    results: &crate::model::evaluation::Results,
) -> Result<Vec<OutputView>, AppError> {
    let mut found = Vec::new();

    for block in blocks {
        if block.kind != BlockKind::Output {
            continue;
        }

        let text = db_evaluation::read::output(db, evaluation, &block.uuid)?;
        let (text, proposal) = match text {
            Some(output) => (output.text, output.proposal),
            None => (String::new(), None),
        };

        found.push(OutputView {
            block: block.uuid.clone(),
            text,
            proposal,
            applies: output_applies(block.fields.condition, results.verdict),
        });
    }

    Ok(found)
}
