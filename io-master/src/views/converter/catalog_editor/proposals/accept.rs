//! A proposal of the AI accepted into its catalog.

use serde_json::Value;

use crate::db::evaluation_support as db_evaluation_support;
use crate::db::oscal as db_oscal;
use crate::db::oscal::error::OscalError;
use crate::db::user::database::{UserDb, in_transaction};
use crate::model::evaluation_support::{Proposal, ProposalKind};
use crate::model::oscal::catalog::edit::Place;
use crate::model::oscal::catalog::{Control, Group};

/// Accepts proposal `id` into its catalog: as it was proposed, or as
/// `corrected` (its JSON of OSCAL, changed by the evaluator). Returns the
/// id of the object.
///
/// One transaction (`in_transaction` takes the work as a closure: the
/// database keeps its connection to itself, so the transaction is opened
/// there): the object added, its quotes kept, the proposal removed; a
/// failure leaves all three as they were.
pub fn accept(db: &UserDb, id: i64, corrected: Option<Value>) -> Result<String, OscalError> {
    let Some(proposal) = db_evaluation_support::read::proposal(db, id)? else {
        return Err(OscalError::Invalid("the proposal does not exist".to_owned()));
    };

    let content = match corrected {
        Some(content) => content,
        None => proposal.content.clone(),
    };

    in_transaction(db, || {
        let object = add(db, &proposal, content)?;

        for quote in &proposal.quotes {
            db_evaluation_support::create::quote(db, &proposal.document_uuid, &object, quote)?;
        }

        db_evaluation_support::delete::proposal(db, id)?;

        Ok(object)
    })
}

/// Adds the object `content` of `proposal` to its catalog; returns its id.
fn add(db: &UserDb, proposal: &Proposal, content: Value) -> Result<String, OscalError> {
    let place = match &proposal.parent_id {
        Some(parent) => Place::Group(parent.clone()),
        None => Place::Top,
    };

    let mut catalog = db_oscal::read::catalog(db, &proposal.document_uuid)?;

    let object = match proposal.kind {
        ProposalKind::Group => {
            let group: Group = read(content)?;
            let object = group.id.clone().unwrap_or_default();
            catalog.add_group(&place, group)?;
            object
        }
        ProposalKind::Control => {
            let control: Control = read(content)?;
            let object = control.id.clone();
            catalog.add_control(&place, control)?;
            object
        }
    };

    db_oscal::update::catalog(db, catalog)?;

    Ok(object)
}

/// `content` read as an object of OSCAL.
fn read<T: serde::de::DeserializeOwned>(content: Value) -> Result<T, OscalError> {
    match serde_json::from_value(content) {
        Ok(object) => Ok(object),
        Err(error) => Err(OscalError::Invalid(format!("the object is not OSCAL: {error}"))),
    }
}

#[cfg(test)]
mod tests {
    //! Tests of `accept.rs`: proposals stored as the AI leaves them,
    //! accepted into a catalog made from the small standard.

    use super::*;

    use serde_json::json;

    use crate::db::evaluation_support::create::NewProposal;
    use crate::model::evaluation_support::QuoteDraft;
    use crate::views::converter::catalog_editor::test_support::{add_group, with_catalog};

    /// Stores a proposal of `kind` with `content`, under `parent`, with one
    /// quote of page 2 of `source`; returns its id.
    fn proposed(
        db: &UserDb,
        uuid: &str,
        kind: ProposalKind,
        parent: Option<&str>,
        content: &Value,
        source: i64,
    ) -> i64 {
        let quotes = [QuoteDraft { source_document_id: source, page: 2, text: "ADV_FSP.4".to_owned(), found: true }];
        let proposal = NewProposal {
            document_uuid: uuid,
            kind,
            parent_id: parent,
            content,
            quotes: &quotes,
            note: "",
            model: "m",
        };

        db_evaluation_support::create::proposal(db, &proposal).unwrap()
    }

    #[test]
    fn an_accepted_group_enters_the_catalog_with_its_quotes() {
        let (window, uuid, source) = with_catalog();
        let session = window.session();
        let db = session.db();
        let id = proposed(&db, &uuid, ProposalKind::Group, None, &json!({"id": "ADV", "title": "Development"}), source);

        assert_eq!(accept(&db, id, None).unwrap(), "ADV");

        assert_eq!(db_oscal::read::catalog(&db, &uuid).unwrap().groups.len(), 1);
        assert_eq!(db_evaluation_support::read::quotes(&db, &uuid, Some("ADV")).unwrap().len(), 1);
        assert!(db_evaluation_support::read::proposals(&db, &uuid).unwrap().is_empty());
    }

    #[test]
    fn a_corrected_control_enters_the_catalog_as_corrected() {
        let (window, uuid, source) = with_catalog();
        add_group(&window, &uuid, "ADV");
        let session = window.session();
        let db = session.db();
        let content = json!({"id": "ADV_FSP.4", "title": "Complete functional specification"});
        let id = proposed(&db, &uuid, ProposalKind::Control, Some("ADV"), &content, source);

        // The evaluator corrects the title before accepting.
        let mut corrected = content.clone();
        corrected["title"] = json!("Complete functional specification (corrected)");
        accept(&db, id, Some(corrected)).unwrap();

        let catalog = db_oscal::read::catalog(&db, &uuid).unwrap();
        let all = catalog.all_controls().unwrap();
        assert_eq!(all[0].0, ["Group ADV"]);
        assert_eq!(all[0].1.title, "Complete functional specification (corrected)");
        assert_eq!(db_evaluation_support::read::quotes(&db, &uuid, Some("ADV_FSP.4")).unwrap().len(), 1);
    }

    #[test]
    fn a_proposal_that_breaks_a_rule_stays_and_nothing_is_written() {
        let (window, uuid, source) = with_catalog();
        let session = window.session();
        let db = session.db();
        let id = proposed(
            &db,
            &uuid,
            ProposalKind::Control,
            Some("NOT_THERE"),
            &json!({"id": "ADV_FSP.4", "title": "A"}),
            source,
        );

        assert!(matches!(accept(&db, id, None), Err(OscalError::NoSuchObject(_))));
        assert_eq!(db_evaluation_support::read::proposals(&db, &uuid).unwrap().len(), 1, "the proposal waits");
        assert!(db_oscal::read::catalog(&db, &uuid).unwrap().all_controls().unwrap().is_empty());
    }
}
