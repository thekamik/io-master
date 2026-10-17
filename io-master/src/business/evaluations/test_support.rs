//! Help for the tests of the evaluation projects: a ready test project
//! on the small standard of `business/evaluations/test_support.rs`, and the
//! fields of an evaluation.

use crate::business::test_projects::test_support::{self as standard, PAGES, opened};
use crate::business::test_projects::{links, projects as test_projects, tiles};
use crate::db::user::database::UserDb;
use crate::model::test_projects::{Source, TileFields, TileKind};

use crate::model::evaluations::EvaluationFields;

/// The tiles of [`ready_project`].
pub struct ReadyProject {
    pub document: i64,
    pub project: i64,
    pub scheme: i64,
    pub input: i64,
    pub requirement: i64,
    pub output: i64,
}

/// A ready test project "P" on the small standard: one scheme with an
/// input, a requirement quoting page 2 and an output, linked
/// input → requirement → output.
pub fn ready_project(db: &UserDb) -> ReadyProject {
    let document = standard::load(db, b"standard");
    let (project, scheme) = standard::project_with_scheme(db, document);

    let mut open = opened(db, project);
    let input =
        tiles::create(db, &mut open, scheme, &TileFields::new(TileKind::Input, "Functional specification")).unwrap();

    let mut fields = TileFields::new(TileKind::Requirement, "Complete functional specification");
    fields.identifier = "ADV_FSP.4.1E".to_owned();
    fields.sources = vec![Source {
        document_id: document,
        page: 2,
        quote: "The evaluator shall confirm that the information provided meets all requirements.".to_owned(),
        found: false,
    }];
    let requirement = tiles::create(db, &mut open, scheme, &fields).unwrap();

    let output = tiles::create(db, &mut open, scheme, &TileFields::new(TileKind::Output, "ADV report")).unwrap();

    links::create(db, &mut open, input, requirement).unwrap();
    links::create(db, &mut open, requirement, output).unwrap();
    test_projects::set_ready(db, &mut open, true).unwrap();

    assert!(PAGES[1].contains("ADV_FSP.4.1E"));

    ReadyProject { document, project, scheme, input, requirement, output }
}

/// The fields of an evaluation named `name`.
pub fn fields(name: &str) -> EvaluationFields {
    EvaluationFields {
        name: name.to_owned(),
        product: "Smart meter X".to_owned(),
        product_version: "2.1".to_owned(),
        manufacturer: "ACME".to_owned(),
        evaluator: "Kamil".to_owned(),
    }
}

/// The CRUD of an open evaluation as single calls on the database, for
/// the tests of the rules: each call opens the evaluation of the tile
/// (or proposal) from the database, makes one change on it, and checks
/// that memory and the database are still the same.
pub mod single {
    use crate::business::evaluations::error::EvaluationError;
    use crate::business::evaluations::results;
    use crate::business::evaluations::workspace::OpenEvaluation;
    use crate::db::evaluations as db_evaluations;
    use crate::db::user::database::UserDb;
    use crate::model::evaluations::{Proposal, RequirementResult, ResultField};
    use crate::model::text::Entity;

    /// The results and the proposals of `open`, in one order, the times
    /// of the proposals left out (memory keeps the time of the
    /// computer, the database its own, a moment apart).
    fn content(open: &OpenEvaluation) -> (Vec<RequirementResult>, Vec<Proposal>) {
        let mut results = open.results().to_vec();
        results.sort_by_key(|result| (result.tile, result.field.name()));

        let proposals = open.proposals().iter().map(|proposal| Proposal { created: 0, ..proposal.clone() }).collect();

        (results, proposals)
    }

    /// Runs `work` on evaluation `evaluation` opened from the database,
    /// and checks afterwards that memory and the database agree.
    pub fn on<T>(db: &UserDb, evaluation: i64, work: impl FnOnce(&mut OpenEvaluation) -> T) -> T {
        let mut open = OpenEvaluation::read(db, evaluation).unwrap().expect("the evaluation exists");
        let result = work(&mut open);
        let again = OpenEvaluation::read(db, evaluation).unwrap().expect("the evaluation exists");

        assert_eq!(content(&open), content(&again), "memory and database differ");

        result
    }

    /// The evaluation of tile `tile`; 0 when there is no such tile.
    fn of_tile(db: &UserDb, tile: i64) -> i64 {
        db_evaluations::read::tile(db, tile).unwrap().map_or(0, |(evaluation, _)| evaluation)
    }

    /// The evaluation of proposal `id`; 0 when there is no such.
    fn of_proposal(db: &UserDb, id: i64) -> i64 {
        db_evaluations::read::proposal_tile(db, id).unwrap().map_or(0, |tile| of_tile(db, tile))
    }

    /// Runs `work` on the evaluation `evaluation`, or gives the error
    /// `missing` when there is none.
    fn on_or<T>(
        db: &UserDb,
        evaluation: i64,
        missing: Entity,
        work: impl FnOnce(&mut OpenEvaluation) -> Result<T, EvaluationError>,
    ) -> Result<T, EvaluationError> {
        if evaluation == 0 {
            return Err(EvaluationError::NotFound(missing));
        }

        on(db, evaluation, work)
    }

    pub fn of(db: &UserDb, tile: i64) -> Result<Vec<RequirementResult>, EvaluationError> {
        on_or(db, of_tile(db, tile), Entity::Tile, |open| Ok(results::list(open, tile)))
    }

    pub fn proposals(db: &UserDb, tile: i64) -> Result<Vec<Proposal>, EvaluationError> {
        on_or(db, of_tile(db, tile), Entity::Tile, |open| Ok(results::proposals(open, tile)))
    }

    pub fn propose(
        db: &UserDb,
        tile: i64,
        field: ResultField,
        text: &str,
        model: &str,
    ) -> Result<i64, EvaluationError> {
        on_or(db, of_tile(db, tile), Entity::Tile, |open| results::propose(db, open, tile, field, text, model))
    }

    pub fn accept(db: &UserDb, id: i64) -> Result<(), EvaluationError> {
        on_or(db, of_proposal(db, id), Entity::Proposal, |open| results::accept(db, open, id))
    }

    pub fn correct(db: &UserDb, id: i64, text: &str) -> Result<(), EvaluationError> {
        on_or(db, of_proposal(db, id), Entity::Proposal, |open| results::correct(db, open, id, text))
    }

    pub fn reject(db: &UserDb, id: i64) -> Result<(), EvaluationError> {
        on_or(db, of_proposal(db, id), Entity::Proposal, |open| results::reject(db, open, id))
    }

    pub fn write(db: &UserDb, tile: i64, field: ResultField, text: &str) -> Result<(), EvaluationError> {
        on_or(db, of_tile(db, tile), Entity::Tile, |open| results::write(db, open, tile, field, text))
    }
}
