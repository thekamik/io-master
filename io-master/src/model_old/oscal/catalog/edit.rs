//! The changes of a catalog in memory: a group or a control added,
//! changed, removed. Every change keeps the rules of OSCAL that the
//! program answers for (an id once in the catalog, a group only in the
//! catalog or a group); the schema of NIST is checked when the catalog is
//! stored (`db/oscal/`).

use std::collections::HashSet;

use super::tree::Location;
use super::{Catalog, CatalogError, Control, Group};

/// Where a new object goes: the top of the catalog, a group, or a
/// control (for an enhancement).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Top,
    Group(String),
    Control(String),
}

impl Catalog {
    /// The ids of every group and control.
    pub fn ids(&self) -> Result<HashSet<String>, CatalogError> {
        let mut ids = HashSet::new();

        for location in self.locations()? {
            if let Some(id) = self.id_at(&location) {
                ids.insert(id.to_owned());
            }
        }

        Ok(ids)
    }

    /// The groups (id, title), in the order of the document.
    pub fn group_titles(&self) -> Result<Vec<(String, String)>, CatalogError> {
        let mut titles = Vec::new();

        for location in self.locations()? {
            if !location.is_group() {
                continue;
            }

            if let Some(Group { id: Some(id), title, .. }) = self.group_at(&location.groups) {
                titles.push((id.clone(), title.clone()));
            }
        }

        Ok(titles)
    }

    /// The groups that hold no group (those whose controls are read from
    /// their pages), in the order of the document: id, title, and label (the
    /// identifier the standard writes, `ADV_FSP`), if it has one.
    ///
    /// Called by: `views/converter/catalog_editor/conversion_wizard/locate.rs`.
    pub fn leaf_groups(&self) -> Result<Vec<(String, String, Option<String>)>, CatalogError> {
        let mut leaves = Vec::new();

        for location in self.locations()? {
            if !location.is_group() {
                continue;
            }

            if let Some(Group { id: Some(id), title, groups, props, .. }) = self.group_at(&location.groups)
                && groups.is_empty()
            {
                let mut label = None;

                for prop in props {
                    if prop.name == "label" {
                        label = Some(prop.value.clone());
                    }
                }

                leaves.push((id.clone(), title.clone(), label));
            }
        }

        Ok(leaves)
    }

    /// Every control, in the order of the document, with the titles of the
    /// groups it is in.
    pub fn all_controls(&self) -> Result<Vec<(Vec<&str>, &Control)>, CatalogError> {
        let mut controls = Vec::new();

        for location in self.locations()? {
            let Some(control) = self.control_at(&location) else {
                continue;
            };

            let mut path = Vec::with_capacity(location.groups.len());

            for depth in 1..=location.groups.len() {
                if let Some(group) = self.group_at(&location.groups[..depth]) {
                    path.push(group.title.as_str());
                }
            }

            controls.push((path, control));
        }

        Ok(controls)
    }

    /// Adds `group` at `place` (the top or a group), without what it holds:
    /// that is added later.
    pub fn add_group(&mut self, place: &Place, mut group: Group) -> Result<(), CatalogError> {
        let Some(id) = group.id.clone() else {
            return Err(CatalogError::Invalid("the group needs an id".to_owned()));
        };

        self.check_new_id(&id)?;
        group.groups.clear();
        group.controls.clear();

        match place {
            Place::Top => self.groups.push(group),
            Place::Group(parent) => {
                let at = self.located_group(parent)?;
                self.group_at_mut(&at.groups).ok_or(CatalogError::NoSuchObject(parent.clone()))?.groups.push(group);
            }
            Place::Control(_) => {
                return Err(CatalogError::Invalid("a group goes into the catalog or a group".to_owned()));
            }
        }

        Ok(())
    }

    /// Changes the own fields of group `id` (its id, title, class, props,
    /// parts, params); the groups and controls it holds stay.
    pub fn update_group(&mut self, id: &str, mut group: Group) -> Result<(), CatalogError> {
        let Some(new) = group.id.clone() else {
            return Err(CatalogError::Invalid("the group needs an id".to_owned()));
        };

        if new != id {
            self.check_new_id(&new)?;
        }

        let at = self.located_group(id)?;
        let stored = self.group_at_mut(&at.groups).ok_or(CatalogError::NoSuchObject(id.to_owned()))?;

        group.groups = std::mem::take(&mut stored.groups);
        group.controls = std::mem::take(&mut stored.controls);
        *stored = group;

        Ok(())
    }

    /// Adds `control` at `place` (the top, a group, or a control for an
    /// enhancement), without its enhancements.
    pub fn add_control(&mut self, place: &Place, mut control: Control) -> Result<(), CatalogError> {
        self.check_new_id(&control.id)?;
        control.controls.clear();

        let into = match place {
            Place::Top => &mut self.controls,
            Place::Group(parent) => {
                let at = self.located_group(parent)?;
                &mut self.group_at_mut(&at.groups).ok_or(CatalogError::NoSuchObject(parent.clone()))?.controls
            }
            Place::Control(parent) => {
                let at = self.located_control(parent)?;
                &mut self.control_at_mut(&at).ok_or(CatalogError::NoSuchObject(parent.clone()))?.controls
            }
        };

        into.push(control);

        Ok(())
    }

    /// Changes the own fields of control `id`; its enhancements stay.
    pub fn update_control(&mut self, id: &str, mut control: Control) -> Result<(), CatalogError> {
        if control.id != id {
            self.check_new_id(&control.id)?;
        }

        let at = self.located_control(id)?;
        let stored = self.control_at_mut(&at).ok_or(CatalogError::NoSuchObject(id.to_owned()))?;

        control.controls = std::mem::take(&mut stored.controls);
        *stored = control;

        Ok(())
    }

    /// Removes the group or control `id` with everything it holds; returns
    /// the ids of everything removed.
    pub fn remove(&mut self, id: &str) -> Result<Vec<String>, CatalogError> {
        let locations = self.locations()?;
        let Some(target) = self.locate(id)? else {
            return Err(CatalogError::NoSuchObject(id.to_owned()));
        };

        let mut gone = Vec::new();

        for location in &locations {
            if !target.holds(location) {
                continue;
            }

            if let Some(inner) = self.id_at(location) {
                gone.push(inner.to_owned());
            }
        }

        self.detach(&target)?;

        Ok(gone)
    }

    /// Takes the object at `at` out of the list that holds it.
    fn detach(&mut self, at: &Location) -> Result<(), CatalogError> {
        if at.is_group() {
            let (last, parent) = at.groups.split_last().ok_or(lost())?;
            let groups =
                if parent.is_empty() { &mut self.groups } else { &mut self.group_at_mut(parent).ok_or(lost())?.groups };

            ensure!(*last < groups.len(), lost());
            groups.remove(*last);

            return Ok(());
        }

        let (last, parent) = at.controls.split_last().ok_or(lost())?;
        let holder = Location { groups: at.groups.clone(), controls: parent.to_vec() };
        let controls = if parent.is_empty() {
            self.controls_of_mut(&at.groups).ok_or(lost())?
        } else {
            &mut self.control_at_mut(&holder).ok_or(lost())?.controls
        };

        ensure!(*last < controls.len(), lost());
        controls.remove(*last);

        Ok(())
    }

    /// The id of a new object, checked: not empty, not used yet.
    fn check_new_id(&self, id: &str) -> Result<(), CatalogError> {
        ensure!(!id.is_empty(), CatalogError::Invalid("the object needs an id".to_owned()));
        ensure!(!self.ids()?.contains(id), CatalogError::IdTaken(id.to_owned()));

        Ok(())
    }

    /// The location of group `id`; an error when it is not a group.
    fn located_group(&self, id: &str) -> Result<Location, CatalogError> {
        match self.locate(id)? {
            Some(location) if location.is_group() => Ok(location),
            _ => Err(CatalogError::NoSuchObject(id.to_owned())),
        }
    }

    /// The location of control `id`; an error when it is not a control.
    fn located_control(&self, id: &str) -> Result<Location, CatalogError> {
        match self.locate(id)? {
            Some(location) if !location.is_group() => Ok(location),
            _ => Err(CatalogError::NoSuchObject(id.to_owned())),
        }
    }
}

/// The error of an object that is no longer where the walk found it (a
/// fault of the program, never of the evaluator).
fn lost() -> CatalogError {
    CatalogError::Invalid("the object is not where it was found".to_owned())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `edit.rs` and `tree.rs`: the changes of a catalog in
    //! memory, and the walk through it.

    use super::*;

    use crate::model_old::oscal::OscalDocument;

    /// The catalog of NIST in the examples, without groups and controls of
    /// its own.
    fn empty() -> Catalog {
        let OscalDocument::Catalog(mut catalog) =
            OscalDocument::from_json(include_str!("../../../../oscal/examples/basic-catalog.json")).unwrap()
        else {
            panic!("the example is a catalog");
        };

        catalog.groups.clear();
        catalog.controls.clear();
        catalog
    }

    fn group(id: &str) -> Group {
        serde_json::from_value(serde_json::json!({ "id": id, "title": format!("Group {id}") })).unwrap()
    }

    fn control(id: &str) -> Control {
        serde_json::from_value(serde_json::json!({ "id": id, "title": format!("Control {id}") })).unwrap()
    }

    /// ADV with ADV_FSP (two enhancements) and the group ADV_X; ATE at the top.
    fn sample() -> Catalog {
        let mut catalog = empty();

        catalog.add_group(&Place::Top, group("ADV")).unwrap();
        catalog.add_control(&Place::Group("ADV".into()), control("ADV_FSP.1")).unwrap();
        catalog.add_control(&Place::Control("ADV_FSP.1".into()), control("ADV_FSP.1.a")).unwrap();
        catalog.add_control(&Place::Control("ADV_FSP.1.a".into()), control("ADV_FSP.1.a.i")).unwrap();
        catalog.add_group(&Place::Group("ADV".into()), group("ADV_X")).unwrap();
        catalog.add_control(&Place::Top, control("ATE")).unwrap();

        catalog
    }

    #[test]
    fn the_walk_gives_the_objects_in_the_order_of_the_document() {
        let catalog = sample();
        let mut ids = Vec::new();

        for location in catalog.locations().unwrap() {
            ids.push(catalog.id_at(&location).unwrap().to_owned());
        }

        assert_eq!(ids, ["ATE", "ADV", "ADV_FSP.1", "ADV_FSP.1.a", "ADV_FSP.1.a.i", "ADV_X"]);
    }

    #[test]
    fn every_control_is_listed_with_the_titles_of_its_groups() {
        let catalog = sample();
        let all = catalog.all_controls().unwrap();

        assert_eq!(all.len(), 4);
        assert_eq!(all[0].0, Vec::<&str>::new());
        assert_eq!(all[2].0, ["Group ADV"]);
        assert_eq!(all[3].1.id, "ADV_FSP.1.a.i");
        assert_eq!(
            catalog.group_titles().unwrap(),
            [("ADV".to_owned(), "Group ADV".to_owned()), ("ADV_X".to_owned(), "Group ADV_X".to_owned())]
        );
    }

    #[test]
    fn an_id_is_used_once_and_a_group_goes_only_into_a_group() {
        let mut catalog = sample();

        assert_eq!(catalog.add_control(&Place::Top, control("ADV")), Err(CatalogError::IdTaken("ADV".into())));
        assert_eq!(
            catalog.add_group(&Place::Top, group("")),
            Err(CatalogError::Invalid("the object needs an id".into()))
        );
        assert!(matches!(catalog.add_group(&Place::Control("ATE".into()), group("G")), Err(CatalogError::Invalid(_))));
        assert_eq!(
            catalog.add_control(&Place::Group("ATE".into()), control("C")),
            Err(CatalogError::NoSuchObject("ATE".into()))
        );
    }

    #[test]
    fn a_change_keeps_what_the_object_holds() {
        let mut catalog = sample();

        catalog.update_group("ADV", group("ADV2")).unwrap();
        catalog.update_control("ADV_FSP.1", control("ADV_FSP.2")).unwrap();

        let ids = catalog.ids().unwrap();
        assert!(ids.contains("ADV2") && ids.contains("ADV_FSP.2") && ids.contains("ADV_FSP.1.a.i"));
        assert!(!ids.contains("ADV") && !ids.contains("ADV_FSP.1"));
        assert_eq!(catalog.update_control("ADV_FSP.2", control("ATE")), Err(CatalogError::IdTaken("ATE".into())));
    }

    #[test]
    fn an_object_is_removed_with_everything_it_holds() {
        let mut catalog = sample();

        let gone = catalog.remove("ADV_FSP.1.a").unwrap();
        assert_eq!(gone, ["ADV_FSP.1.a", "ADV_FSP.1.a.i"]);

        let gone = catalog.remove("ADV").unwrap();
        assert_eq!(gone, ["ADV", "ADV_FSP.1", "ADV_X"]);

        assert_eq!(catalog.ids().unwrap().into_iter().collect::<Vec<_>>(), ["ATE"]);
        assert_eq!(catalog.remove("ADV"), Err(CatalogError::NoSuchObject("ADV".into())));
    }

    #[test]
    fn a_catalog_deeper_than_the_limit_is_refused() {
        let mut catalog = empty();
        let mut place = Place::Top;

        for level in 0..=crate::model_old::oscal::catalog::tree::MAX_DEPTH {
            let id = format!("C{level}");
            catalog.add_control(&place, control(&id)).unwrap_or(());
            place = Place::Control(id);
        }

        assert_eq!(catalog.locations(), Err(CatalogError::TooDeep));
    }
}
