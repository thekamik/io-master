//! Unit tests of `db/evaluation_support/` (attached to `create.rs`;
//! they cover the whole folder).

use super::*;

use serde_json::json;

use crate::db::evaluation_support::{delete, read};
use crate::test_support::standards::{database, load};

#[test]
fn quotes_are_stored_read_and_deleted() {
    let test = database();
    let source = load(&test.db, b"%PDF support");
    let draft = QuoteDraft { source_document_id: source, page: 2, text: "The developer shall".to_owned(), found: true };

    let id = quote(&test.db, "doc", "ADV_FSP.4", &draft).unwrap();
    quote(&test.db, "doc", "ATE_IND.2", &draft).unwrap();

    assert_eq!(read::quotes(&test.db, "doc", None).unwrap().len(), 2);
    assert_eq!(read::quotes(&test.db, "doc", Some("ADV_FSP.4")).unwrap()[0].id, id);
    assert!(read::quotes_source(&test.db, source).unwrap());

    delete::quotes_of(&test.db, "doc", &["ATE_IND.2".to_owned()]).unwrap();
    assert!(delete::quote(&test.db, id).unwrap());
    assert!(read::quotes(&test.db, "doc", None).unwrap().is_empty());
}

#[test]
fn proposals_keep_their_content_and_quotes() {
    let test = database();
    let source = load(&test.db, b"%PDF proposals");
    let content = json!({"id": "ADV_FSP.4", "title": "Complete functional specification"});
    let quotes = [QuoteDraft { source_document_id: source, page: 2, text: "ADV_FSP.4".to_owned(), found: false }];

    let id = proposal(
        &test.db,
        &NewProposal {
            document_uuid: "doc",
            kind: ProposalKind::Control,
            parent_id: Some("ADV"),
            content: &content,
            quotes: &quotes,
            note: "The component of ADV.",
            model: "gemma4",
        },
    )
    .unwrap();

    let stored = read::proposal(&test.db, id).unwrap().unwrap();
    assert_eq!(stored.content, content);
    assert_eq!(stored.quotes, quotes);
    assert_eq!(stored.parent_id.as_deref(), Some("ADV"));
    assert_eq!(read::proposals(&test.db, "doc").unwrap().len(), 1);

    delete::of_document(&test.db, "doc").unwrap();
    assert!(read::proposals(&test.db, "doc").unwrap().is_empty());
}
