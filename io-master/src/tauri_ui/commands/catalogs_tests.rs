//! Unit tests of `catalogs.rs`: the editor of a catalog, through the
//! commands, as the page uses them.

use super::*;

use serde_json::json;

use crate::tauri_ui::test_support::with_document;

#[tokio::test]
async fn a_catalog_is_made_edited_and_quoted_through_the_commands() {
    let (test, document) = with_document().await;

    let made = crate::business::oscal::catalogs::create(
        &test.state.session().unwrap().db(),
        "CC Part 3",
        "CC:2022",
        &[document],
    )
    .unwrap();

    let view = add_group(&test.state, made.uuid.clone(), Place::Top, json!({"id": "ADV", "title": "Development"}))
        .await
        .unwrap();
    assert_eq!(view.catalog["groups"][0]["id"], "ADV");

    let control = json!({"id": "ADV_FSP.4", "title": "Complete functional specification",
        "parts": [{"id": "ADV_FSP.4_smt", "name": "statement", "prose": "The developer shall provide a functional specification."}]});
    let view = add_control(&test.state, made.uuid.clone(), Place::Group("ADV".into()), control).await.unwrap();
    assert_eq!(view.catalog["groups"][0]["controls"][0]["id"], "ADV_FSP.4");

    let view = add_quote(
        &test.state,
        made.uuid.clone(),
        "ADV_FSP.4".into(),
        document,
        2,
        "The developer shall provide".into(),
    )
    .await
    .unwrap();
    assert!(view.quotes[0].found);

    // Something that is not OSCAL is refused before it reaches the business.
    assert!(add_group(&test.state, made.uuid.clone(), Place::Top, json!({"title": 5})).await.is_err());

    let view = delete_object(&test.state, made.uuid.clone(), "ADV".into()).await.unwrap();
    assert!(view.catalog.get("groups").is_none());
    assert!(view.quotes.is_empty());

    assert_eq!(
        instructions(&test.state, "catalog_outline".into()).await.unwrap(),
        crate::ai::tasks::catalog_outline::DEFAULT_INSTRUCTIONS
    );
    assert!(instructions(&test.state, "chat".into()).await.is_err());
}
