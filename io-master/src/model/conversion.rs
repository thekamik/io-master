//! The conversion of the documents of a standard (PDF) into a catalog of
//! OSCAL, led step by step (step 50.2 of docs/todo.md): where a conversion
//! is, the pages of its table of contents, and the pages and the state of
//! every group whose controls the AI reads. Plain data and its rules; it is
//! stored by `db/conversion/`, linked to its catalog by the UUID, and never
//! written into the document of OSCAL.
//!
//! ```text
//! stage      Contents ─► Groups ─► Controls ─► Review ─► Finished
//! a group    Todo ─► Proposed ─► Done
//! ```
//!
//! Only a finished catalog is used by the view Evaluation plan; a catalog
//! that was imported, not converted, has no conversion and is finished.

use std::fmt;

/// The most groups whose pages a conversion keeps: a standard of a few
/// thousand pages has some hundreds.
pub const MAX_GROUPS: usize = 2000;

/// The stage of a conversion, in the order the evaluator goes through them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The pages of the table of contents (found by the AI, confirmed).
    Contents,
    /// The groups the AI proposed from the table of contents, reviewed.
    Groups,
    /// The controls, group by group.
    Controls,
    /// The last review: what is left to check.
    Review,
    /// Done: the catalog is used by the view Evaluation plan.
    Finished,
}

impl Stage {
    pub const ALL: [Stage; 5] = [Stage::Contents, Stage::Groups, Stage::Controls, Stage::Review, Stage::Finished];

    /// The name kept in the database and sent to the page.
    pub fn name(self) -> &'static str {
        match self {
            Stage::Contents => "contents",
            Stage::Groups => "groups",
            Stage::Controls => "controls",
            Stage::Review => "review",
            Stage::Finished => "finished",
        }
    }

    pub fn from_name(name: &str) -> Option<Stage> {
        Stage::ALL.into_iter().find(|stage| stage.name() == name)
    }
}

/// How far the controls of a group are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupState {
    /// Nothing asked yet.
    Todo,
    /// The AI proposed its controls; the evaluator reviews them.
    Proposed,
    /// The evaluator said the group is done.
    Done,
}

impl GroupState {
    pub const ALL: [GroupState; 3] = [GroupState::Todo, GroupState::Proposed, GroupState::Done];

    /// The name kept in the database and sent to the page.
    pub fn name(self) -> &'static str {
        match self {
            GroupState::Todo => "todo",
            GroupState::Proposed => "proposed",
            GroupState::Done => "done",
        }
    }

    pub fn from_name(name: &str) -> Option<GroupState> {
        GroupState::ALL.into_iter().find(|state| state.name() == name)
    }
}

/// Pages of one document of a standard, from `first` to `last`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pages {
    pub document_id: i64,
    pub first: u32,
    pub last: u32,
}

/// A conversion: its stage and the pages of its table of contents, if they
/// are known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversion {
    pub stage: Stage,
    pub contents: Option<Pages>,
}

/// The pages and the state of one group of a conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupPages {
    pub group_id: String,
    pub pages: Option<Pages>,
    pub state: GroupState,
}

/// Why pages are refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionError {
    /// The pages are not in the document, or the first is after the last.
    Pages { first: u32, last: u32, page_count: u32 },
}

impl fmt::Display for ConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionError::Pages { first, last, page_count } => {
                write!(formatter, "the pages {first} to {last} are not in a document of {page_count} pages")
            }
        }
    }
}

impl std::error::Error for ConversionError {}

/// Checks that `pages` are in a document of `page_count` pages.
///
/// Called by: the commands of the wizard (`views/converter/catalog_editor/
/// catalog_editor.rs`), before pages from the page or from the AI are kept.
pub fn check_pages(pages: Pages, page_count: u32) -> Result<(), ConversionError> {
    let error = ConversionError::Pages { first: pages.first, last: pages.last, page_count };

    ensure!(pages.first >= 1, error);
    ensure!(pages.first <= pages.last, error);
    ensure!(pages.last <= page_count, error);

    Ok(())
}

/// The pages of every group from the page where each one starts: a group
/// goes from its start to the page before the next start found, the last
/// one to the end of the document. A group whose start was not found gets
/// no pages (the evaluator gives them).
///
/// `starts` are the groups in the order of the document, each with the page
/// it starts on, if found.
///
/// Called by: `conversion_wizard/locate.rs`.
pub fn ranges(starts: &[(String, Option<u32>)], document_id: i64, page_count: u32) -> Vec<GroupPages> {
    let mut groups = Vec::with_capacity(starts.len());

    for (index, (group_id, start)) in starts.iter().enumerate() {
        // A short closure: Option::map, which clippy asks for here.
        let pages = start.map(|first| Pages { document_id, first, last: end(starts, index, first, page_count) });

        groups.push(GroupPages { group_id: group_id.clone(), pages, state: GroupState::Todo });
    }

    groups
}

/// The last page of the group at `index` that starts on `first`: the page
/// before the next start found after it, else the end of the document.
///
/// Called by: [`ranges`].
fn end(starts: &[(String, Option<u32>)], index: usize, first: u32, page_count: u32) -> u32 {
    for (_, next) in starts.iter().skip(index + 1) {
        if let Some(next) = next
            && *next > first
        {
            return next - 1;
        }
    }

    page_count.max(first)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `conversion.rs`.

    use super::*;

    #[test]
    fn the_names_of_the_stages_and_states_read_back() {
        for stage in Stage::ALL {
            assert_eq!(Stage::from_name(stage.name()), Some(stage));
        }

        for state in GroupState::ALL {
            assert_eq!(GroupState::from_name(state.name()), Some(state));
        }

        assert_eq!(Stage::from_name("done"), None);
    }

    #[test]
    fn pages_must_be_in_the_document() {
        let pages = |first, last| Pages { document_id: 1, first, last };

        assert!(check_pages(pages(1, 10), 10).is_ok());
        assert!(check_pages(pages(0, 3), 10).is_err());
        assert!(check_pages(pages(5, 4), 10).is_err());
        assert!(check_pages(pages(5, 11), 10).is_err());
    }

    #[test]
    fn a_group_goes_to_the_page_before_the_next_start() {
        let starts = [
            ("adv".to_owned(), Some(10)),
            ("agd".to_owned(), None),
            ("alc".to_owned(), Some(25)),
            ("ate".to_owned(), Some(40)),
        ];

        let groups = ranges(&starts, 7, 60);

        assert_eq!(groups[0].pages, Some(Pages { document_id: 7, first: 10, last: 24 }));
        assert_eq!(groups[1].pages, None);
        assert_eq!(groups[2].pages, Some(Pages { document_id: 7, first: 25, last: 39 }));
        assert_eq!(groups[3].pages, Some(Pages { document_id: 7, first: 40, last: 60 }));
        assert!(groups.iter().all(|group| group.state == GroupState::Todo));
    }

    #[test]
    fn two_groups_on_one_page_share_it() {
        let starts = [("a".to_owned(), Some(5)), ("b".to_owned(), Some(5)), ("c".to_owned(), Some(9))];

        let groups = ranges(&starts, 1, 20);

        assert_eq!(groups[0].pages, Some(Pages { document_id: 1, first: 5, last: 8 }));
        assert_eq!(groups[1].pages, Some(Pages { document_id: 1, first: 5, last: 8 }));
    }
}
