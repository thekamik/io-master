//! Where the objects of a catalog are, and the walk through all of them,
//! without recursion (rule 1 of CLAUDE.md): one explicit stack, bounded by
//! [`MAX_DEPTH`] levels and [`MAX_OBJECTS`] objects.
//!
//! ```text
//! Location { groups: [2, 0], controls: [] }       the first group in the third group
//! Location { groups: [2],    controls: [1, 0] }   the first enhancement of the second control of the third group
//! Location { groups: [],     controls: [4] }      the fifth control of the top of the catalog
//! ```

use super::super::common::Part;
use super::{Catalog, CatalogError, Control, Group};

/// The most levels of groups and enhancements an object may be in.
pub const MAX_DEPTH: usize = 32;

/// The most groups and controls a catalog may have.
pub const MAX_OBJECTS: usize = 100_000;

/// Where an object of a catalog is: the index of a group at every level,
/// then, for a control, the index of a control at every level (a control,
/// its enhancement, ...). A location without controls is a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub groups: Vec<usize>,
    pub controls: Vec<usize>,
}

impl Location {
    /// Whether the location is of a group.
    pub fn is_group(&self) -> bool {
        self.controls.is_empty()
    }

    /// The levels the object is down.
    pub fn depth(&self) -> usize {
        self.groups.len() + self.controls.len()
    }

    /// Whether `other` is this object or inside it.
    pub fn holds(&self, other: &Location) -> bool {
        if self.is_group() {
            return other.groups.starts_with(&self.groups);
        }

        other.groups == self.groups && other.controls.starts_with(&self.controls)
    }

    fn with_group(&self, index: usize) -> Location {
        let mut groups = self.groups.clone();
        groups.push(index);

        Location { groups, controls: Vec::new() }
    }

    fn with_control(&self, index: usize) -> Location {
        let mut controls = self.controls.clone();
        controls.push(index);

        Location { groups: self.groups.clone(), controls }
    }
}

impl Catalog {
    /// Every group and control, in the order of the document: an object,
    /// then its controls, then its groups. Refused when the catalog is
    /// deeper than [`MAX_DEPTH`] or larger than [`MAX_OBJECTS`].
    pub fn locations(&self) -> Result<Vec<Location>, CatalogError> {
        let top = Location { groups: Vec::new(), controls: Vec::new() };
        let mut pending = Vec::new();
        let mut found = Vec::new();

        // Pushed in reverse, so that they are taken in order.
        for index in (0..self.groups.len()).rev() {
            pending.push(top.with_group(index));
        }

        for index in (0..self.controls.len()).rev() {
            pending.push(top.with_control(index));
        }

        for _ in 0..MAX_OBJECTS {
            let Some(location) = pending.pop() else {
                return Ok(found);
            };

            ensure!(location.depth() <= MAX_DEPTH, CatalogError::TooDeep);
            self.push_inner(&location, &mut pending);
            found.push(location);
        }

        Err(CatalogError::TooLarge)
    }

    /// Puts the objects inside the object at `location` on `pending`, in
    /// reverse: its groups, then its controls.
    fn push_inner(&self, location: &Location, pending: &mut Vec<Location>) {
        if location.is_group() {
            let Some(group) = self.group_at(&location.groups) else {
                return;
            };

            for index in (0..group.groups.len()).rev() {
                pending.push(location.with_group(index));
            }

            for index in (0..group.controls.len()).rev() {
                pending.push(location.with_control(index));
            }
        } else if let Some(control) = self.control_at(location) {
            for index in (0..control.controls.len()).rev() {
                pending.push(location.with_control(index));
            }
        }
    }

    /// The location of the group or control `id`, if there is one.
    pub fn locate(&self, id: &str) -> Result<Option<Location>, CatalogError> {
        for location in self.locations()? {
            if self.id_at(&location) == Some(id) {
                return Ok(Some(location));
            }
        }

        Ok(None)
    }

    /// The id of the object at `location` (a group may have none).
    pub fn id_at(&self, location: &Location) -> Option<&str> {
        if location.is_group() {
            return self.group_at(&location.groups)?.id.as_deref();
        }

        Some(&self.control_at(location)?.id)
    }

    /// The group at the indices `groups`.
    pub fn group_at(&self, groups: &[usize]) -> Option<&Group> {
        let (first, rest) = groups.split_first()?;
        let mut group = self.groups.get(*first)?;

        for index in rest {
            group = group.groups.get(*index)?;
        }

        Some(group)
    }

    /// The group at the indices `groups`, to be changed.
    pub fn group_at_mut(&mut self, groups: &[usize]) -> Option<&mut Group> {
        let (first, rest) = groups.split_first()?;
        let mut group = self.groups.get_mut(*first)?;

        for index in rest {
            group = group.groups.get_mut(*index)?;
        }

        Some(group)
    }

    /// The controls of the top of the catalog (`groups` empty) or of a
    /// group.
    fn controls_of(&self, groups: &[usize]) -> Option<&Vec<Control>> {
        if groups.is_empty() {
            return Some(&self.controls);
        }

        Some(&self.group_at(groups)?.controls)
    }

    /// The controls of the top or of a group, to be changed.
    pub fn controls_of_mut(&mut self, groups: &[usize]) -> Option<&mut Vec<Control>> {
        if groups.is_empty() {
            return Some(&mut self.controls);
        }

        Some(&mut self.group_at_mut(groups)?.controls)
    }

    /// The control at `location`.
    pub fn control_at(&self, location: &Location) -> Option<&Control> {
        let (first, rest) = location.controls.split_first()?;
        let mut control = self.controls_of(&location.groups)?.get(*first)?;

        for index in rest {
            control = control.controls.get(*index)?;
        }

        Some(control)
    }

    /// The titles of the groups and controls that hold the object at
    /// `location`, from the top of the catalog down; a control with its
    /// label (`FCS_CKM.1: Cryptographic key generation`). Refused when the
    /// location is deeper than [`MAX_DEPTH`].
    ///
    /// Called by: `views/evaluation_lab/lab_task/read.rs` (the requirement for the AI).
    pub fn titles_above(&self, location: &Location) -> Result<Vec<String>, CatalogError> {
        ensure!(location.depth() <= MAX_DEPTH, CatalogError::TooDeep);
        let mut titles = Vec::with_capacity(location.depth());

        for end in 1..=location.groups.len() {
            let Some(groups) = location.groups.get(..end) else {
                continue;
            };
            let Some(group) = self.group_at(groups) else {
                continue;
            };

            titles.push(group.title.clone());
        }

        // The controls above it: every one but itself (the last index).
        for end in 1..location.controls.len() {
            let Some(controls) = location.controls.get(..end) else {
                continue;
            };
            let above = Location { groups: location.groups.clone(), controls: controls.to_vec() };
            let Some(control) = self.control_at(&above) else {
                continue;
            };

            titles.push(match control.label() {
                Some(label) => format!("{label}: {}", control.title),
                None => control.title.clone(),
            });
        }

        Ok(titles)
    }

    /// The control at `location`, to be changed.
    pub fn control_at_mut(&mut self, location: &Location) -> Option<&mut Control> {
        let (first, rest) = location.controls.split_first()?;
        let mut control = self.controls_of_mut(&location.groups)?.get_mut(*first)?;

        for index in rest {
            control = control.controls.get_mut(*index)?;
        }

        Some(control)
    }
}

/// The parts of an object and the parts in them, in the order of the
/// document, each with its level (0: a part of the object); for the page,
/// which shows them without recursion.
pub fn flat_parts(parts: &[Part]) -> Result<Vec<(usize, &Part)>, CatalogError> {
    let mut pending: Vec<(usize, &Part)> = Vec::new();
    let mut found = Vec::new();

    for part in parts.iter().rev() {
        pending.push((0, part));
    }

    for _ in 0..MAX_OBJECTS {
        let Some((level, part)) = pending.pop() else {
            return Ok(found);
        };

        ensure!(level <= MAX_DEPTH, CatalogError::TooDeep);

        for inner in part.parts.iter().rev() {
            pending.push((level + 1, inner));
        }

        found.push((level, part));
    }

    Err(CatalogError::TooLarge)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `tree.rs`.

    use super::*;
    use crate::model::oscal::OscalDocument;

    /// The basic catalog of the examples of NIST.
    fn catalog() -> Catalog {
        match OscalDocument::from_json(include_str!("../../../../oscal/examples/basic-catalog.json")).unwrap() {
            OscalDocument::Catalog(catalog) => catalog,
            _ => panic!("the basic catalog is a catalog"),
        }
    }

    #[test]
    fn the_titles_above_a_control_go_from_the_top_down() {
        let catalog = catalog();
        let location = catalog.locate("s1.1.1").unwrap().unwrap();

        let titles = catalog.titles_above(&location).unwrap();

        assert_eq!(titles, ["Organization of Information Security", "Internal Organization"]);
    }

    #[test]
    fn a_location_too_deep_is_refused() {
        let location = Location { groups: vec![0; MAX_DEPTH + 1], controls: Vec::new() };

        assert!(matches!(catalog().titles_above(&location), Err(CatalogError::TooDeep)));
    }
}
