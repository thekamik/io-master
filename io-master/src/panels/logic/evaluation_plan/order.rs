//! The order of the work of a plan and its cross references (user,
//! 2026-10-28): a work unit comes after all its dependencies; a work unit
//! that depends on itself, directly or through others, is a cross
//! reference, a mistake of the planner, and its plan is blocked until it
//! is corrected.
//!
//! ```text
//! work units ──Kahn's algorithm (forward)──► the order; what is left depends on a cycle
//!            ──the same backward on what is left──► the work units of the cycles themselves
//! check_order ──► cross_reference of every work unit, blocked of the plan (written when changed)
//! ```
//!
//! Without recursion: each pass takes every work unit at most once, and each
//! taking looks once at every work unit of the plan (rule 1 and 2).

use crate::db::models::evaluation::evaluation_plan::{read_evaluation_plan, update_evaluation_plan};
use crate::db::models::evaluation::work_unit::{read_work_units, update_work_unit};
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::evaluation_plan::EvaluationPlan;
use crate::model::evaluation::work_unit::WorkUnit;
use crate::panels::logic::evaluation_plan::error::PlanError;

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
/// when no work unit left depends on it. Gives the positions taken, in their
/// order.
///
/// Called by: [`work_order`].
fn taken_in_order(units: &[WorkUnit], positions: &[usize], backward: bool) -> Vec<usize> {
    let mut waiting = Vec::with_capacity(positions.len());

    for &position in positions {
        let mut count = 0;

        for &other in positions {
            if let (Some(unit), Some(other_unit)) = (units.get(position), units.get(other))
                && links(unit, other_unit, backward)
            {
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

    for next in 0..positions.len() {
        let Some(&index) = taken.get(next) else { break };
        let Some(done) = unit_at(units, positions, index) else { break };

        for (other, count) in waiting.iter_mut().enumerate() {
            let Some(other_unit) = unit_at(units, positions, other) else { continue };

            if *count > 0 && links(other_unit, done, backward) {
                *count -= 1;

                if *count == 0 {
                    taken.push(other);
                }
            }
        }
    }

    let mut result = Vec::with_capacity(taken.len());

    for index in taken {
        if let Some(&position) = positions.get(index) {
            result.push(position);
        }
    }

    result
}

/// The work unit at the `index`-th of `positions`.
///
/// Called by: [`taken_in_order`].
fn unit_at<'a>(units: &'a [WorkUnit], positions: &[usize], index: usize) -> Option<&'a WorkUnit> {
    units.get(*positions.get(index)?)
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
/// Called by: [`check_order`]; the view of the evaluation, which checks a
/// plan again before it is copied.
/// Calls: [`taken_in_order`].
pub fn work_order(units: &[WorkUnit]) -> Order {
    let mut all = Vec::with_capacity(units.len());

    for position in 0..units.len() {
        all.push(position);
    }

    let order = taken_in_order(units, &all, false);
    let mut left = Vec::with_capacity(units.len().saturating_sub(order.len()));

    for position in all {
        if !order.contains(&position) {
            left.push(position);
        }
    }

    let behind = taken_in_order(units, &left, true);
    let mut crossed = Vec::with_capacity(left.len());

    for position in left {
        if !behind.contains(&position) {
            crossed.push(position);
        }
    }

    Order { order, crossed }
}

/// Marks the cross references of the plan `plan`: `cross_reference` of every
/// work unit, and `blocked` of the plan when there is one; writes only what
/// changed. Gives whether the plan is blocked.
///
/// Called by: `work_units.rs`, after every change of the work units of a
/// plan in planning (a dependency set, a work unit removed).
/// Calls: `read_work_units`, [`work_order`], `update_work_unit`,
/// `read_evaluation_plan`, `update_evaluation_plan`.
pub fn check_order(db: &UserDatabase, plan: i64) -> Result<bool, PlanError> {
    let units = read_work_units(db, plan)?;
    let crossed = work_order(&units).crossed;

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

    use crate::db::models::evaluation::evaluation_plan::create_evaluation_plan;
    use crate::db::models::evaluation::work_unit::{create_work_unit, read_work_unit};
    use crate::test_support::database::{database, node, plan, unit};

    #[test]
    fn a_work_unit_comes_after_its_dependencies_otherwise_in_the_order_of_the_plan() {
        let units = [node(1, &[3]), node(2, &[]), node(3, &[])];
        let order = work_order(&units);

        assert_eq!(order.order, vec![1, 2, 0]);
        assert!(order.crossed.is_empty());
    }

    #[test]
    fn only_the_work_units_of_a_cycle_are_cross_references() {
        // 1 ⇄ 2 is a cycle; 3 depends on it; 4 is free.
        let units = [node(1, &[2]), node(2, &[1]), node(3, &[1]), node(4, &[])];
        let order = work_order(&units);

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
