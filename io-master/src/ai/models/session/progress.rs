//! The progress of a long work of the AI as its steps write it (the model:
//! `model/session/progress.rs`, never kept in a database): each work unit
//! gone through is counted, and one not taken, or taken with notes, is kept
//! for the page among the newest problems.
//!
//! ```text
//! a step of the work ──► record_step ──► assessed / failed, label, last_verdict
//!                                    └─► problems (the newest first, at most MAX_PROBLEMS)
//! ```

use crate::model::evaluation::work_unit::Verdict;
use crate::model::session::progress::{Problem, Progress};

/// The most problems kept for the page: the newest.
const MAX_PROBLEMS: usize = 200;

/// Writes one step of the work into `progress`: the work unit `label` gone
/// through, with its verdict when the work gives one.
///
/// Input:
/// - `progress`: the progress of the work that runs, locked.
/// - `label`: the label of the work unit ("ADV_ARC.1-1").
/// - `done`: whether the work unit was done (assessed, filled); `false`
///   when an error or a refused answer left it as it was.
/// - `verdict`: its verdict now, for an assessment; `None` otherwise.
/// - `notes`: why it was not done, or what was not taken of the answer;
///   empty when all was taken.
///
/// Output: nothing; the counts, the last step and the problems of
/// `progress` changed.
///
/// Called by: `panels/views/evaluation/run.rs`: `evaluation_assess`;
/// `panels/views/evaluation_plan/fill.rs`: `fill`.
pub fn record_step(progress: &mut Progress, label: String, done: bool, verdict: Option<Verdict>, notes: Vec<String>) {
    if done {
        progress.assessed += 1;
    } else {
        progress.failed += 1;
    }

    // A work unit not done, or with notes, is what the user must look at.
    if !done || !notes.is_empty() {
        progress.problems.insert(0, Problem { label: label.clone(), notes });
        progress.problems.truncate(MAX_PROBLEMS);
    }

    progress.label = label;
    progress.last_verdict = verdict;
}

#[cfg(test)]
mod tests {
    //! Tests of the steps written into the progress.

    use super::*;

    #[test]
    fn steps_are_counted_and_only_the_newest_problems_kept() {
        let mut progress = Progress { running: true, total: 300, ..Progress::default() };

        record_step(&mut progress, "A-1".to_owned(), true, Some(Verdict::Pass), Vec::new());

        assert_eq!((progress.assessed, progress.failed, progress.problems.len()), (1, 0, 0));
        assert_eq!((progress.label.as_str(), progress.last_verdict), ("A-1", Some(Verdict::Pass)));

        for number in 0..250 {
            record_step(&mut progress, format!("B-{number}"), false, None, vec!["no answer".to_owned()]);
        }

        assert_eq!((progress.assessed, progress.failed), (1, 250));
        assert_eq!(progress.problems.len(), MAX_PROBLEMS);
        assert_eq!(progress.problems[0].label, "B-249");
        assert_eq!(progress.last_verdict, None);
    }
}
