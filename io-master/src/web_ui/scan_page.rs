//! The scan page: the serial ports of the computer, and the
//! settings of the scanner.
//!
//! ```text
//! GET  /scan            scan with the stored settings and show the table
//! POST /scan/settings   store new settings, then back to GET /scan
//! ```
//!
//! The settings (kinds of port, columns) are kept in the encrypted
//! database of the evaluator (`devices/scan_settings.rs`), so every
//! evaluator has their own. After the settings are stored the
//! browser is sent back to `GET /scan`: reloading the page then
//! scans again instead of sending the form a second time.
//!
//! The scan itself is `devices::scan`; this file only turns forms
//! into settings and ports into a table.

use std::path::Path;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{Extension, Form};

use crate::accounts::accounts::Account;
use crate::db::database_key::DatabaseKey;
use crate::db::user_db;
use crate::devices::devices::{self, PortFilter, PortType, SerialPortInfo};
use crate::devices::port_property::PortProperty;
use crate::devices::scan_settings::ScanSettings;
use crate::installation::lifecycle::Running;

use super::pages::{self, Page, escape};
use super::session::{CurrentUser, UserKey};
use super::web_ui::{AppState, not_found};

/// The value of the "all kinds of port" box of the form.
const ALL_TYPES: &str = "all";

/// What a scan produced, for the page.
enum Outcome {
    Ports(Vec<SerialPortInfo>),
    Problem(String),
}

/// The running installation, when it is a workstation: only a
/// workstation has this page.
fn workstation(state: &AppState) -> Option<Arc<Running>> {
    state
        .running()
        .filter(|running| Page::of_role(running.installation.role).contains(&Page::Scan))
}

/// Reads the settings of the evaluator and scans with them. Runs on
/// a thread for blocking work: it opens a database and reads many
/// small files.
fn scan_with_settings(data_dir: &Path, account_id: i64, key: &DatabaseKey) -> (ScanSettings, Outcome) {
    let settings = match user_db::open(data_dir, account_id, key).and_then(|db| ScanSettings::load(&db)) {
        Ok(settings) => settings,
        Err(error) => {
            return (
                ScanSettings::default(),
                Outcome::Problem(format!("Your settings cannot be read: {error}.")),
            );
        }
    };

    let outcome = match devices::scan(&settings.filter) {
        Ok(ports) => Outcome::Ports(ports),
        Err(error) => Outcome::Problem(format!("The ports cannot be scanned: {error}.")),
    };

    (settings, outcome)
}

/// `GET /scan`: scans with the stored settings.
pub async fn show(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Extension(UserKey(key)): Extension<UserKey>,
) -> Response {
    let Some(running) = workstation(&state) else {
        return not_found().await;
    };

    let data_dir = state.data_dir.as_ref().clone();
    let account_id = account.id;

    let result = tokio::task::spawn_blocking(move || scan_with_settings(&data_dir, account_id, &key)).await;

    let (settings, outcome) = result.unwrap_or_else(|_| {
        (
            ScanSettings::default(),
            Outcome::Problem("The scan was interrupted.".to_owned()),
        )
    });

    answer(&running, &account, &settings, &outcome)
}

/// The whole page.
fn answer(running: &Running, account: &Account, settings: &ScanSettings, outcome: &Outcome) -> Response {
    let html = pages::page(
        &running.installation,
        &account.name,
        Page::Scan,
        &content(settings, outcome),
    );

    let status = match outcome {
        Outcome::Ports(_) => StatusCode::OK,
        Outcome::Problem(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (status, Html(html)).into_response()
}

/// The HTML of the page: the settings, folded under a one-line
/// summary (a `<details>` element, which opens without JavaScript),
/// then the result, which is what the page is for.
fn content(settings: &ScanSettings, outcome: &Outcome) -> String {
    format!(
        "<details class=\"settings\">\n\
         <summary>Scanner settings: {summary}</summary>\n\
         {form}\n\
         </details>\n\
         <section>\n<h2>Serial ports <a class=\"button\" href=\"/scan\">Scan again</a></h2>\n{result}\n</section>",
        summary = escape(&summary(settings)),
        form = settings_form(settings),
        result = match outcome {
            Outcome::Ports(ports) => port_table(ports, &settings.columns),
            Outcome::Problem(text) => pages::problem_text(Some(text)),
        },
    )
}

/// The settings in one line: "ttyACM, ttyUSB · 5 columns".
fn summary(settings: &ScanSettings) -> String {
    let types = match &settings.filter {
        PortFilter::All => "all kinds of port".to_owned(),
        PortFilter::Types(types) => types.iter().map(PortType::base_name).collect::<Vec<_>>().join(", "),
    };

    let count = settings.columns.len();

    format!("{types} · {count} column{}", if count == 1 { "" } else { "s" })
}

/// A checkbox of the form.
fn checkbox(name: &str, value: &str, label: &str, checked: bool) -> String {
    format!(
        "<label class=\"choice\"><input type=\"checkbox\" name=\"{name}\" value=\"{value}\"{checked}> {label}</label>",
        name = name,
        value = escape(value),
        label = escape(label),
        checked = if checked { " checked" } else { "" },
    )
}

/// The form of the settings: the kinds of port and the columns.
fn settings_form(settings: &ScanSettings) -> String {
    let all = matches!(settings.filter, PortFilter::All);
    let chosen = |port_type: &PortType| !all && settings.filter.accepts(port_type);

    // The known kinds, and the unknown ones the evaluator chose.
    let mut types: Vec<PortType> = PortType::known().collect();

    if let PortFilter::Types(chosen_types) = &settings.filter {
        types.extend(chosen_types.iter().filter(|port_type| !types.contains(port_type)).cloned().collect::<Vec<_>>());
    }

    let type_boxes: String = types
        .iter()
        .map(|port_type| checkbox("type", port_type.base_name(), port_type.base_name(), chosen(port_type)))
        .collect();

    let column_boxes: String = PortProperty::ALL
        .iter()
        .map(|property| checkbox("column", property.key(), property.title(), settings.columns.contains(property)))
        .collect();

    format!(
        "<form method=\"post\" action=\"/scan/settings\">\n\
         <fieldset>\n\
         <legend>Kinds of port</legend>\n\
         {all}\n\
         <div class=\"choices\">{type_boxes}</div>\n\
         </fieldset>\n\
         <fieldset>\n\
         <legend>Columns of the table</legend>\n\
         <div class=\"choices\">{column_boxes}</div>\n\
         </fieldset>\n\
         <button type=\"submit\">Save and scan</button>\n\
         </form>",
        all = checkbox("type", ALL_TYPES, "All kinds", all),
    )
}

/// The table of ports with the chosen columns.
fn port_table(ports: &[SerialPortInfo], columns: &[PortProperty]) -> String {
    if ports.is_empty() {
        return "<p>No serial ports of the chosen kinds were found.</p>".to_owned();
    }

    let header: String = columns
        .iter()
        .map(|column| format!("<th>{}</th>", escape(column.title())))
        .collect();

    let rows: String = ports
        .iter()
        .map(|port| {
            let cells: String = columns
                .iter()
                .map(|column| {
                    // Values with several lines (the stable names)
                    // keep their lines.
                    let value = escape(&column.value(port)).replace('\n', "<br>");

                    format!("<td>{value}</td>")
                })
                .collect();

            format!("<tr>{cells}</tr>")
        })
        .collect();

    format!(
        "<p class=\"note\">{count} port{plural} found.</p>\n\
         <table class=\"ports\">\n<thead><tr>{header}</tr></thead>\n<tbody>{rows}</tbody>\n</table>",
        count = ports.len(),
        plural = if ports.len() == 1 { "" } else { "s" },
    )
}

/// The values of all fields of the form with the given name.
fn values<'a>(fields: &'a [(String, String)], name: &'a str) -> impl Iterator<Item = &'a str> {
    fields
        .iter()
        .filter(move |(field, _)| field == name)
        .map(|(_, value)| value.as_str())
}

/// Turns the fields of the form into settings. A browser sends
/// a checkbox only when it is ticked, once per ticked box, so the
/// fields arrive as a list of name and value.
fn settings_from_form(fields: &[(String, String)]) -> ScanSettings {
    let all_types = values(fields, "type").any(|value| value == ALL_TYPES);

    let types = values(fields, "type")
        .filter(|value| *value != ALL_TYPES && !value.is_empty())
        .map(PortType::from_base_name)
        .collect();

    let columns = values(fields, "column").filter_map(PortProperty::from_key).collect();

    ScanSettings::new(all_types, types, columns)
}

/// `POST /scan/settings`: stores the settings, then scans with them.
pub async fn save(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Extension(UserKey(key)): Extension<UserKey>,
    Form(fields): Form<Vec<(String, String)>>,
) -> Response {
    let Some(running) = workstation(&state) else {
        return not_found().await;
    };

    let settings = settings_from_form(&fields);
    let data_dir = state.data_dir.as_ref().clone();
    let account_id = account.id;
    let stored = settings.clone();

    let result = tokio::task::spawn_blocking(move || {
        user_db::open(&data_dir, account_id, &key).and_then(|db| stored.save(&db))
    })
    .await;

    match result {
        Ok(Ok(())) => Redirect::to("/scan").into_response(),
        Ok(Err(error)) => answer(
            &running,
            &account,
            &settings,
            &Outcome::Problem(format!("Your settings were not saved: {error}.")),
        ),
        Err(_) => answer(
            &running,
            &account,
            &settings,
            &Outcome::Problem("Saving was interrupted.".to_owned()),
        ),
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "scan_page_tests.rs"]
mod tests;
