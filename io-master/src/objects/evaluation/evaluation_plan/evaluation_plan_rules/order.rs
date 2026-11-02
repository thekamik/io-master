//! The order of the work of a plan (`EvaluationPlan`) and its cross
//! references (user, 2026-10-28): a work unit comes after all its
//! dependencies; a work unit that depends on itself, directly or through
//! others, is a cross reference, a mistake of the planner, and its plan is
//! blocked until it is corrected.
//!
//! ```text
//! work units ──Kahn's algorithm (forward)──► the order; what is left depends on a cycle
//!            ──the same backward on what is left──► the work units of the cycles themselves
//! check_order ──► cross_reference of every work unit, blocked of the plan (written when changed)
//! ```
//!
//! Without recursion: each pass takes every work unit at most once, and each
//! taking looks once at every work unit of the plan (rule 1 and 2).

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::EvaluationPlan;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::{
    read_evaluation_plan, update_evaluation_plan,
};
use crate::objects::evaluation::work_unit::work_unit_model::WorkUnit;
use crate::objects::evaluation::work_unit::work_unit_store::{read_work_units, update_work_unit};

/// The order of the work of a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    /// The positions in the plan of the work units that can be carried out,
    /// in the order they can be: each after all its dependencies, otherwise
    /// in the order of the plan.
    pub order: Vec<usize>,
    /// The positions of the work units in a cycle of dependencies: the cross
    /// references.
    pub crossed: Vec<usize>,
}

/// Kahn's algorithm over the work units at `positions`: takes, again and
/// again, a work unit whose counted dependencies are all taken. With
/// `backward`, a dependency is counted the other way: a work unit is taken
/// when no work unit left depends on it.
///
/// Input: `units`, the work units of a plan; `positions`, positions in
/// `units`, each at most once; `backward`, the direction of the counting.
/// Output: the positions taken, in their order (`[1, 2, 0]`); an error when
/// a position is outside `units`, which [`work_order`] never gives.
///
/// Called by: [`work_order`].
/// Calls: [`links`], [`unit_at`].
fn taken_in_order(units: &[WorkUnit], positions: &[usize], backward: bool) -> Result<Vec<usize>, DomainError> {
    ensure!(positions.len() <= units.len(), outside(positions.len()));

    // How many of the work units at `positions` each one waits for.
    let mut waiting = Vec::with_capacity(positions.len());

    for index in 0..positions.len() {
        let unit = unit_at(units, positions, index)?;
        let mut count = 0;

        for other in 0..positions.len() {
            if links(unit, unit_at(units, positions, other)?, backward) {
                count += 1;
            }
        }

        waiting.push(count);
    }

    let mut taken: Vec<usize> = Vec::with_capacity(positions.len());

    for (index, count) in waiting.iter().enumerate() {
        if *count == 0 {
            taken.push(index);
        }
    }

    // Each turn takes the next work unit of `taken` and frees those that
    // waited for it; a work unit is pushed once, when its count reaches 0,
    // so `taken` never outgrows `positions`.
    for next in 0..positions.len() {
        let Some(&index) = taken.get(next) else { break };
        let done = unit_at(units, positions, index)?;

        for (other, count) in waiting.iter_mut().enumerate() {
            if *count > 0 && links(unit_at(units, positions, other)?, done, backward) {
                *count -= 1;

                if *count == 0 {
                    taken.push(other);
                }
            }
        }
    }

    let mut result = Vec::with_capacity(taken.len());

    for index in taken {
        let position = positions.get(index).copied().ok_or_else(|| outside(index))?;

        result.push(position);
    }

    Ok(result)
}

/// The work unit at the `index`-th of `positions`; an error when either is
/// outside its list.
///
/// Called by: [`taken_in_order`].
/// Calls: [`outside`].
fn unit_at<'a>(units: &'a [WorkUnit], positions: &[usize], index: usize) -> Result<&'a WorkUnit, DomainError> {
    let position = positions.get(index).copied().ok_or_else(|| outside(index))?;

    units.get(position).ok_or_else(|| outside(position))
}

/// The error of a position outside the work units of the plan, which the
/// order of the work never gives.
///
/// Called by: [`taken_in_order`], [`unit_at`].
fn outside(position: usize) -> DomainError {
    DomainError::Refused(format!("the order of the work: the position {position} is outside the plan"))
}

/// Whether `unit` waits for `other`: forward, when `unit` depends on
/// `other`; backward, when `other` depends on `unit`.
///
/// Called by: [`taken_in_order`].
fn links(unit: &WorkUnit, other: &WorkUnit, backward: bool) -> bool {
    if backward { other.dependencies.contains(&unit.id) } else { unit.dependencies.contains(&other.id) }
}

/// The order of the work of the work units `units` (a whole plan) and its
/// cross references: forward, the work units that can be carried out;
/// backward on those left, the ones that only depend on a cycle are taken
/// away, and the work units of the cycles themselves remain.
///
/// Input: `units`, the work units of a plan, in the order of the plan.
/// Output: the order and the cross references, as positions in `units`
/// (`Order { order: [1, 2, 0], crossed: [] }`); an error only if
/// [`taken_in_order`] finds a position outside `units`.
///
/// Called by: [`check_order`]; `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`:
/// `units_in_order`.
/// Calls: [`taken_in_order`].
pub fn work_order(units: &[WorkUnit]) -> Result<Order, DomainError> {
    let mut all = Vec::with_capacity(units.len());

    for position in 0..units.len() {
        all.push(position);
    }

    let order = taken_in_order(units, &all, false)?;
    let mut left = Vec::with_capacity(units.len().saturating_sub(order.len()));

    for position in all {
        if !order.contains(&position) {
            left.push(position);
        }
    }

    let behind = taken_in_order(units, &left, true)?;
    let mut crossed = Vec::with_capacity(left.len());

    for position in left {
        if !behind.contains(&position) {
            crossed.push(position);
        }
    }

    ensure!(order.len() + crossed.len() <= units.len(), outside(units.len()));

    Ok(Order { order, crossed })
}

/// Marks the cross references of the plan `plan`: `cross_reference` of every
/// work unit, and `blocked` of the plan when there is one; writes only what
/// changed.
///
/// Input: `plan`, the id of a stored plan.
/// Output: whether the plan is blocked (`true`); an error of the database,
/// or of [`work_order`].
///
/// Called by: `objects/evaluation/evaluation_plan/evaluation_plan_rules/work_units.rs`:
/// `remove_work_unit`; `objects/evaluation/work_unit/work_unit_rules/parameters.rs`:
/// `update_work_unit_parameters`.
/// Calls: `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`, `update_work_unit`; [`work_order`];
/// `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`, `update_evaluation_plan`.
pub fn check_order(db: &UserDatabase, plan: i64) -> Result<bool, DomainError> {
    let units = read_work_units(db, plan)?;
    let crossed = work_order(&units)?.crossed;

    for (position, unit) in units.iter().enumerate() {
        let is_crossed = crossed.contains(&position);

        if unit.cross_reference != is_crossed {
            update_work_unit(db, &WorkUnit { cross_reference: is_crossed, ..unit.clone() })?;
        }
    }

    let stored = read_evaluation_plan(db, plan)?;
    let blocked = !crossed.is_empty();

    if stored.blocked != blocked {
        update_evaluation_plan(db, &EvaluationPlan { blocked, ..stored })?;
    }

    Ok(blocked)
}

#[cfg(test)]
mod tests {
    //! Tests of the order of the work: a chain, a dependency on a later
    //! work unit, a cycle with a work unit behind it; the marks written.

    use super::*;

    use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::create_evaluation_plan;
    use crate::objects::evaluation::work_unit::work_unit_store::{create_work_unit, read_work_unit};
    use crate::test_support::database::{database, node, plan, unit};

    #[test]
    fn a_work_unit_comes_after_its_dependencies_otherwise_in_the_order_of_the_plan() {
        let units = [node(1, &[3]), node(2, &[]), node(3, &[])];
        let order = work_order(&units).unwrap();

        assert_eq!(order.order, vec![1, 2, 0]);
        assert!(order.crossed.is_empty());
    }

    #[test]
    fn only_the_work_units_of_a_cycle_are_cross_references() {
        // 1 ⇄ 2 is a cycle; 3 depends on it; 4 is free.
        let units = [node(1, &[2]), node(2, &[1]), node(3, &[1]), node(4, &[])];
        let order = work_order(&units).unwrap();

        assert_eq!(order.order, vec![3]);
        assert_eq!(order.crossed, vec![0, 1]);
    }

    #[test]
    fn the_marks_are_written_and_taken_back_when_the_cycle_is_broken() {
        let (_directory, db) = database();
        let routers = plan(&db);
        let id = create_evaluation_plan(&db, &routers).unwrap();
        let first = create_work_unit(&db, id, &unit(&routers, "ASE_INT.1-1")).unwrap();
        let second =
            create_work_unit(&db, id, &WorkUnit { dependencies: vec![first], ..unit(&routers, "ASE_INT.1-2") })
                .unwrap();
        let mut back = read_work_unit(&db, first).unwrap();

        back.dependencies = vec![second];
        update_work_unit(&db, &back).unwrap();

        assert!(check_order(&db, id).unwrap());
        assert!(read_work_unit(&db, first).unwrap().cross_reference);
        assert!(read_evaluation_plan(&db, id).unwrap().blocked);

        back = read_work_unit(&db, first).unwrap();
        back.dependencies.clear();
        update_work_unit(&db, &back).unwrap();

        assert!(!check_order(&db, id).unwrap());
        assert!(!read_work_unit(&db, second).unwrap().cross_reference);
        assert!(!read_evaluation_plan(&db, id).unwrap().blocked);
    }
}
