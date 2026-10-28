//! The screen of a task of an evaluation read for the page (step 52 of
//! docs/history.md): the frozen scheme and related elements, the requirement
//! from its catalog, and what the evaluator added (`db/evaluation/`). Every
//! command that changes a task answers with it, so the page always shows
//! what the program keeps.

use crate::ai::tasks::candidates::requirement_text;
use crate::ai::tasks::requirement_context::{self, Linked};
use crate::app::error::AppError;
use crate::components::part_rows::view as part_rows;
use crate::db::evaluation as db_evaluation;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::model_old::evaluation::{LabTask, TaskAgent, output_applies};
use crate::model_old::oscal::catalog::{Catalog, Control};
use crate::model_old::task_scheme::{BlockKind, ResultText};

use super::super::assessment::context;
use super::view::{
    ContextPassageView, EvidenceView, FragmentView, InputView, KnowledgeView, LabBlockView, LabConnectionView,
    LabRelatedView, LabRequirementView, LabTaskScreenView, MessageView, OutputView, ProposalView, ResultsForm,
};
use crate::components::knowledge_agent::view as agent_view;

/// Task `task` of evaluation `evaluation`; refused when it does not exist.
///
/// Called by: the commands of a task (`evaluation_lab.rs`), [`screen`].
pub fn task(db: &UserDb, evaluation: &str, task: &str) -> Result<LabTask, AppError> {
    match db_evaluation::read::task(db, evaluation, task)? {
        Some(found) => Ok(found),
        None => Err(AppError::Invalid("the task does not exist".to_owned())),
    }
}

/// The most links of a requirement named in a question.
const MAX_LINKED: usize = 50;

/// The requirement of `task` from its catalog, its whole text for a
/// question to the AI (step 57.1: where it stands, its parts, parameters
/// and links; `requirement_context::requirement`) and the title of the
/// catalog; refused when the catalog has it no more.
///
/// Called by: [`screen`], the works of the AI of a task.
/// Calls: `db_oscal::read::catalog`, `titles_above`, [`linked`], `requirement_context::requirement`.
pub fn requirement(db: &UserDb, task: &LabTask) -> Result<(Control, String, String), AppError> {
    let catalog = db_oscal::read::catalog(db, &task.catalog_uuid)?;

    let location = catalog.locate(&task.control_id)?;
    let Some(location) = location else {
        return Err(AppError::Invalid("the catalog has the requirement no more".to_owned()));
    };
    let Some(control) = catalog.control_at(&location) else {
        return Err(AppError::Invalid("the catalog has the requirement no more".to_owned()));
    };

    let above = catalog.titles_above(&location)?;
    let linked = linked(&catalog, control)?;
    let text = requirement_context::requirement(&catalog.metadata.title, &above, &task.label, control, &linked)
        .map_err(AppError::Invalid)?;

    Ok((control.clone(), text, catalog.metadata.title.clone()))
}

/// The requirements of `catalog` that `control` links to (`#id`), at most
/// [`MAX_LINKED`]; a link to a resource or to another document is left out.
///
/// Called by: [`requirement`].
fn linked(catalog: &Catalog, control: &Control) -> Result<Vec<Linked>, AppError> {
    let mut linked = Vec::new();

    for link in control.links.iter().take(MAX_LINKED) {
        let Some(id) = link.href.strip_prefix('#') else {
            continue;
        };
        let Some(location) = catalog.locate(id)? else {
            continue;
        };
        let Some(target) = catalog.control_at(&location) else {
            continue;
        };

        linked.push(Linked {
            relation: link.rel.as_deref().unwrap_or("related").to_owned(),
            text: format!("{}: {}", target.label().unwrap_or(&target.id), target.title),
        });
    }

    Ok(linked)
}

/// The text of the related elements of task `task` for the agent of the
/// requirement (step 57.1): each word for word from its catalog, up to the
/// limit of `requirement_context::related`; an element its catalog has no
/// more is left out. Every catalog is read once.
///
/// Called by: `assessment/propose.rs`.
/// Calls: `db_evaluation::read::related`, `db_oscal::read::catalog`, `requirement_text`, `requirement_context::related`.
pub fn related_text(db: &UserDb, evaluation: &str, task: &str) -> Result<String, AppError> {
    let related = db_evaluation::read::related(db, evaluation, task)?;
    let mut catalogs: Vec<(String, Catalog)> = Vec::new();
    let mut texts = Vec::with_capacity(related.len());

    for element in &related {
        if position_of(&catalogs, &element.catalog_uuid).is_none() {
            catalogs.push((element.catalog_uuid.clone(), db_oscal::read::catalog(db, &element.catalog_uuid)?));
        }

        let Some(at) = position_of(&catalogs, &element.catalog_uuid) else {
            continue;
        };
        let Some((_, catalog)) = catalogs.get(at) else {
            continue;
        };
        let Some(location) = catalog.locate(&element.control_id)? else {
            continue;
        };
        let Some(control) = catalog.control_at(&location) else {
            continue;
        };

        texts.push(requirement_text(&element.label, control).map_err(AppError::Invalid)?);
    }

    Ok(requirement_context::related(&texts))
}

/// Where the catalog `uuid` is in `catalogs`, if it was read.
///
/// Called by: [`related_text`].
fn position_of(catalogs: &[(String, Catalog)], uuid: &str) -> Option<usize> {
    for (at, (read, _)) in catalogs.iter().enumerate() {
        if read == uuid {
            return Some(at);
        }
    }

    None
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
    let knowledge = knowledge(db, evaluation, &found)?;
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
        knowledge,
    })
}

/// The agent with knowledge of `task` and its passages, for the panel of
/// the requirement (step 57.4).
///
/// Called by: [`screen`].
/// Calls: `context::context`, `agent_view::agents`.
fn knowledge(db: &UserDb, evaluation: &str, task: &LabTask) -> Result<KnowledgeView, AppError> {
    let found = context::context(db, evaluation, task)?;
    let (choice, agent) = match found.choice {
        TaskAgent::AsEvaluation => ("evaluation", None),
        TaskAgent::NoAgent => ("none", None),
        TaskAgent::Agent(id) => ("agent", Some(id)),
    };
    let mut passages = Vec::with_capacity(found.passages.len());

    for (hit, chosen) in found.passages {
        passages.push(ContextPassageView {
            id: hit.id,
            source: hit.source,
            page: hit.passage.page,
            label: hit.passage.label,
            text: hit.passage.text,
            chosen,
        });
    }

    let used = match found.agent {
        Some(used) => Some(used.fields.name),
        None => None,
    };

    Ok(KnowledgeView { choice, agent, used, agents: agent_view::agents(db)?, passages })
}

/// Every input of `blocks` with its evidence, chat and fragments.
///
/// Called by: [`screen`].
fn inputs(
    db: &UserDb,
    evaluation: &str,
    blocks: &[crate::model_old::task_scheme::Block],
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
    blocks: &[crate::model_old::task_scheme::Block],
    results: &crate::model_old::evaluation::Results,
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
