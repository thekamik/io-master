//! Help for the tests of the test projects: a database of an
//! evaluator in a temporary directory, a small standard of three pages
//! loaded into it, and a project with schemes on it.

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::user::database::{self as user_db, UserDb};
use crate::security::random::fill_random;
use crate::test_support::TempDir;

use super::workspace::{OpenProject, ProjectWorkspace};
use super::{projects, schemes};
use crate::business::standards;
use crate::model::standards::DocumentFields;
use crate::model::test_projects::{ProjectFields, ProposalState, SchemeFields};

/// The pages of the small standard of the tests.
pub const PAGES: [&str; 3] = [
    "Part 3: Security assurance components\n1 Scope\nThis part defines the assurance requirements.",
    "ADV_FSP.4 Complete functional specification\nThe developer shall provide a functional specifi-\ncation.\n\
     ADV_FSP.4.1E The evaluator shall confirm that the information provided meets all requirements.",
    "ATE_IND.2 Independent testing - sample\nThe developer shall provide the TOE for testing.\n\
     ATE_IND.2.1E The evaluator shall test a subset of the TSF.",
];

/// A database of an evaluator; the directory lives as long as the
/// returned value.
pub struct TestDb {
    /// Held so that the folder lives as long as the database.
    #[allow(dead_code)]
    pub directory: TempDir,
    pub db: UserDb,
}

pub fn database() -> TestDb {
    let directory = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let db = user_db::create(directory.path(), 1, &key).unwrap();

    TestDb { directory, db }
}

/// The fields of the small standard.
pub fn fields() -> DocumentFields {
    DocumentFields { title: "Common Criteria".to_owned(), part: "Part 3".to_owned(), version: "CC:2022 R1".to_owned() }
}

/// Loads the small standard (with `content` as its file, so that
/// different calls make different files); returns its number.
pub fn load(db: &UserDb, content: &[u8]) -> i64 {
    let pages: Vec<String> = PAGES.iter().map(|page| (*page).to_owned()).collect();

    standards::create::add(db, &fields(), "cc-part3.pdf", content, &pages).unwrap()
}

/// A small but real PDF with one page per text (Helvetica; a line
/// break in a text starts a new line), for the tests that run
/// `pdftotext`.
pub fn pdf(pages: &[&str]) -> Vec<u8> {
    let font = 3 + pages.len() * 2;
    let kids: Vec<String> = (0..pages.len()).map(|index| format!("{} 0 R", 3 + index * 2)).collect();

    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), pages.len()),
    ];

    for (index, text) in pages.iter().enumerate() {
        let lines: Vec<String> = text.split('\n').map(|line| format!("({line}) Tj 0 -14 Td")).collect();
        let content = format!("BT /F1 12 Tf 72 720 Td {} ET", lines.join(" "));

        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 {font} 0 R >> >> \
             /Contents {} 0 R >>",
            4 + index * 2
        ));
        objects.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
    }

    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned());

    let mut file = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();

    for (number, object) in objects.iter().enumerate() {
        offsets.push(file.len());
        file.push_str(&format!("{} 0 obj\n{object}\nendobj\n", number + 1));
    }

    let xref = file.len();
    file.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));

    for offset in offsets {
        file.push_str(&format!("{offset:010} 00000 n \n"));
    }

    file.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R /Info << /Title (Test standard) >> >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    ));

    file.into_bytes()
}

/// The fields of a project that uses `documents`.
pub fn project_fields(name: &str, documents: Vec<i64>) -> ProjectFields {
    ProjectFields {
        name: name.to_owned(),
        version: "1.0".to_owned(),
        description: "The plan of the tests".to_owned(),
        author: "Head of the laboratory".to_owned(),
        documents,
    }
}

/// The fields of an accepted scheme without pages.
pub fn scheme_fields(number: &str, name: &str) -> SchemeFields {
    SchemeFields { number: number.to_owned(), name: name.to_owned(), pages: None, state: ProposalState::Accepted }
}

/// A project "P" that uses `document`, with one scheme "5 ADV";
/// returns the project and the scheme.
pub fn project_with_scheme(db: &UserDb, document: i64) -> (i64, i64) {
    let project = projects::create::project(db, &project_fields("P", vec![document])).unwrap();
    let mut workspace = ProjectWorkspace::default();
    let scheme = schemes::create::scheme(db, workspace.open(db, project).unwrap(), &scheme_fields("5", "ADV")).unwrap();

    (project, scheme)
}

/// Project `project`, opened in a workspace of its own (read from the
/// database as it is stored now).
pub fn opened(db: &UserDb, project: i64) -> OpenProject {
    let mut workspace = ProjectWorkspace::default();
    workspace.open(db, project).unwrap();

    workspace.take(project).expect("just opened")
}

/// The CRUD of an open project as single calls on the database, for
/// the tests of the rules: each call opens the project from the
/// database, makes one change on it, and checks that memory and the
/// database are still the same.
pub mod single {
    use crate::business::test_projects::content;
    use crate::business::test_projects::error::StoreError;
    use crate::business::test_projects::projects;
    use crate::business::test_projects::workspace::{OpenProject, ProjectWorkspace};
    use crate::business::test_projects::{links as link_crud, schemes, tiles};
    use crate::db::error::DbError;
    use crate::db::test_projects::read as stored;
    use crate::db::user::database::UserDb;
    use crate::model::test_projects::rules::SchemeError;
    use crate::model::test_projects::{Link, ProjectFields, Scheme, SchemeFields, Tile, TileFields};

    use super::opened;

    /// Runs `work` on project `project` opened from the database, and
    /// checks afterwards that memory and the database agree.
    pub fn on<T>(db: &UserDb, project: i64, work: impl FnOnce(&mut OpenProject) -> T) -> T {
        let mut open = opened(db, project);
        let result = work(&mut open);

        assert_eq!(&projects::read::data(db, project).unwrap().unwrap(), open.data(), "memory and database differ");

        result
    }

    pub fn project_of_scheme(db: &UserDb, scheme: i64) -> i64 {
        stored::scheme(db, scheme).unwrap().map_or(0, |scheme| scheme.project)
    }

    pub fn project_of_tile(db: &UserDb, tile: i64) -> i64 {
        stored::project_of_tile(db, tile).unwrap().unwrap_or(0)
    }

    pub fn create(db: &UserDb, project: i64, fields: &SchemeFields) -> Result<i64, StoreError> {
        on(db, project, |open| schemes::create::scheme(db, open, fields))
    }

    pub fn of_project(db: &UserDb, project: i64) -> Result<Vec<Scheme>, DbError> {
        stored::schemes(db, project)
    }

    pub fn existing(db: &UserDb, id: i64) -> Result<Scheme, StoreError> {
        get(db, id)?.ok_or(StoreError::Scheme(SchemeError::NotFound("scheme")))
    }

    pub fn get(db: &UserDb, id: i64) -> Result<Option<Scheme>, DbError> {
        stored::scheme(db, id)
    }

    pub fn accept(db: &UserDb, id: i64) -> Result<(), StoreError> {
        on(db, project_of_scheme(db, id), |open| schemes::update::accept(db, open, id))
    }

    pub fn update(db: &UserDb, id: i64, fields: &SchemeFields) -> Result<(), StoreError> {
        on(db, project_of_scheme(db, id), |open| schemes::update::scheme(db, open, id, fields))
    }

    pub fn delete(db: &UserDb, id: i64) -> Result<bool, DbError> {
        on(db, project_of_scheme(db, id), |open| schemes::delete::scheme(db, open, id))
    }

    pub fn add_tile(db: &UserDb, scheme: i64, fields: &TileFields) -> Result<i64, StoreError> {
        on(db, project_of_scheme(db, scheme), |open| tiles::create::tile(db, open, scheme, fields))
    }

    pub fn update_tile(db: &UserDb, id: i64, fields: &TileFields) -> Result<(), StoreError> {
        on(db, project_of_tile(db, id), |open| tiles::update::tile(db, open, id, fields))
    }

    pub fn move_tile(db: &UserDb, id: i64, x: f64, y: f64) -> Result<(), StoreError> {
        let project = project_of_tile(db, id);

        if project == 0 {
            return Err(SchemeError::NotFound("tile").into());
        }

        on(db, project, |open| tiles::update::position(db, open, id, x, y))
    }

    pub fn move_to_scheme(db: &UserDb, ids: &[i64], scheme: i64) -> Result<(), StoreError> {
        on(db, project_of_scheme(db, scheme), |open| tiles::update::to_scheme(db, open, ids, scheme))
    }

    pub fn accept_tile(db: &UserDb, id: i64) -> Result<(), StoreError> {
        on(db, project_of_tile(db, id), |open| tiles::update::accept(db, open, id))
    }

    pub fn delete_tile(db: &UserDb, id: i64) -> Result<bool, DbError> {
        on(db, project_of_tile(db, id), |open| tiles::delete::tile(db, open, id))
    }

    pub fn tile(db: &UserDb, id: i64) -> Result<Option<Tile>, DbError> {
        stored::tile(db, id)
    }

    pub fn project_tiles(db: &UserDb, project: i64) -> Result<Vec<Tile>, DbError> {
        stored::project_tiles(db, project)
    }

    pub fn add_link(db: &UserDb, from: i64, to: i64) -> Result<i64, StoreError> {
        let (one, other) = (project_of_tile(db, from), project_of_tile(db, to));

        if one == 0 || other == 0 {
            return Err(SchemeError::NotFound("tile").into());
        }

        if one != other {
            return Err(SchemeError::Invalid("link", "only tiles of one project can be linked".to_owned()).into());
        }

        on(db, one, |open| link_crud::create::link(db, open, from, to))
    }

    pub fn delete_link(db: &UserDb, id: i64) -> Result<bool, DbError> {
        let project = projects::read::list(db)?
            .into_iter()
            .find(|project| stored::links(db, project.id).unwrap().iter().any(|link| link.id == id))
            .map_or(0, |project| project.id);

        if project == 0 {
            return Ok(false);
        }

        on(db, project, |open| link_crud::delete::link(db, open, id))
    }

    pub fn links(db: &UserDb, project: i64) -> Result<Vec<Link>, DbError> {
        stored::links(db, project)
    }

    pub fn replace_content(
        db: &UserDb,
        project: i64,
        tiles: &[Tile],
        links: &[Link],
    ) -> Result<Vec<(i64, i64)>, StoreError> {
        content::replace(db, project, tiles, links)
    }

    pub fn create_project(db: &UserDb, fields: &ProjectFields) -> Result<i64, StoreError> {
        projects::create::project(db, fields)
    }

    pub fn update_project(db: &UserDb, project: i64, fields: &ProjectFields) -> Result<(), StoreError> {
        on(db, project, |open| projects::update::fields(db, open, fields))
    }

    pub fn set_ready(db: &UserDb, project: i64, ready: bool) -> Result<(), StoreError> {
        on(db, project, |open| projects::update::ready(db, open, ready))
    }

    pub fn delete_project(db: &UserDb, project: i64) -> Result<bool, DbError> {
        projects::delete::project(db, &mut ProjectWorkspace::default(), project)
    }
}
