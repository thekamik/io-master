//! Help for the tests of the desktop interface: a state with a real
//! installation (databases, accounts) in a temporary directory.

use crate::installation::installation::Role;
use crate::installation::lifecycle::{self, Opened};
use crate::tpm::test_support::TempDir;

use super::account_actions;
use super::state::UiState;

/// The password of the user of the tests.
pub const PASSWORD: &str = "correct horse battery";

/// A state and its data directory; dropping it removes the
/// directory.
pub struct TestState {
    /// Not read by the tests; held so that the folder lives exactly
    /// as long as the state that uses it.
    #[allow(dead_code)]
    pub directory: TempDir,
    pub state: UiState,
}

/// A state at the first start.
pub fn first_start() -> TestState {
    let directory = TempDir::new();
    let state = UiState::new(directory.path().to_owned(), Opened::FirstRun);

    TestState { directory, state }
}

/// A workstation installed by "anna", nobody logged in yet.
pub fn installed() -> TestState {
    let directory = TempDir::new();
    let (running, _) = lifecycle::install(directory.path(), Role::Workstation, "anna", PASSWORD).unwrap();
    let state = UiState::new(directory.path().to_owned(), Opened::Installed(running));

    TestState { directory, state }
}

/// A workstation with "anna" logged in.
pub async fn logged_in() -> TestState {
    let test = installed();

    account_actions::log_in(&test.state, "anna".to_owned(), PASSWORD.to_owned()).await.unwrap();

    test
}

/// "anna" logged in, with one loaded document of three pages (a small
/// real PDF, read by Poppler); returns the state and the document.
pub async fn with_document() -> (TestState, i64) {
    let test = logged_in().await;
    let directory = TempDir::new();
    let path = directory.path().join("cc-part3.pdf");
    std::fs::write(
        &path,
        crate::evaluation::test_support::pdf(&[
            "ADV_FSP.4 Complete functional specification",
            "The developer shall provide a functional specification.",
            "ATE_IND.2 The evaluator shall test a subset of the TSF.",
        ]),
    )
    .unwrap();

    super::standard_actions::load(&test.state, vec![path], |_| true).await.unwrap();
    let id = super::standard_actions::list(&test.state).await.unwrap()[0].id;

    (test, id)
}

/// The form of a project that uses `documents`.
pub fn project_form(name: &str, documents: Vec<i64>) -> super::serializer::project_view::ProjectForm {
    super::serializer::project_view::ProjectForm {
        name: name.into(),
        version: "1".into(),
        description: String::new(),
        author: "Head".into(),
        documents,
    }
}

/// The form of an accepted scheme, with the pages `pages` of
/// `document` when given.
pub fn scheme_form(number: &str, name: &str, pages: Option<(i64, u32, u32)>) -> super::serializer::project_view::SchemeForm {
    super::serializer::project_view::SchemeForm {
        number: number.into(),
        name: name.into(),
        pages: pages.map(|(document_id, first_page, last_page)| super::serializer::project_view::PageSpanView {
            document_id,
            first_page,
            last_page,
        }),
        state: "accepted".into(),
    }
}
